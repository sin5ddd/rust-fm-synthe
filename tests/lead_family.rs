//! Factory lead bank: monophonic notes should stack an audible octave
//! (carrier ratio 2, chorus `octave-up`/`octave-down`, or supersaw unison
//! plus an octave layer) so they read lush rather than a thin beep.
//!
//! Identities stay: C3-rooted plucks, hollow fifths, inharmonic bells.
//! Leads must not steal the kick/bass sub.

use fm_synth::{
    analyze_buffer, factory_ids, load_factory, midi_to_hz, render, rms, Algorithm, AnalyzeOpts,
    ChorusInterval, ExportParams, RenderParams, Waveform,
};

const SR: u32 = 48_000;

fn ld_folder_ids() -> Vec<&'static str> {
    factory_ids()
        .into_iter()
        .filter(|id| {
            id.starts_with("ld-")
                || matches!(
                    *id,
                    "lead-fm-pluck"
                        | "stab-fm-fifth"
                        | "stab-fm-major"
                        | "filter-pluck"
                        | "stab-pluck"
                )
        })
        .collect()
}

fn carrier_indices(algo: Algorithm) -> &'static [usize] {
    match algo {
        Algorithm::Serial
        | Algorithm::ParallelMod
        | Algorithm::DoubleMod
        | Algorithm::SharedMod => &[0],
        Algorithm::DualStack => &[0, 2],
        Algorithm::TripleCarrier | Algorithm::StackPlusCarriers => &[0, 1, 2],
        Algorithm::AllCarriers => &[0, 1, 2, 3],
    }
}

fn audible_carrier_octave(preset: &fm_synth::Preset) -> bool {
    for &i in carrier_indices(preset.algorithm) {
        let Some(op) = preset.operators.get(i) else {
            continue;
        };
        let audible = op.level >= 0.22 && (op.sustain >= 0.15 || op.level >= 0.35);
        if audible && (1.85..=2.20).contains(&op.ratio) {
            return true;
        }
    }
    false
}

fn octave_chorus(preset: &fm_synth::Preset) -> bool {
    let ch = &preset.fx.chorus;
    ch.mix >= 0.12
        && ch
            .intervals
            .iter()
            .any(|iv| matches!(iv, ChorusInterval::OctaveUp | ChorusInterval::OctaveDown))
}

fn thick_supersaw_octave(preset: &fm_synth::Preset) -> bool {
    audible_carrier_octave(preset)
        && carrier_indices(preset.algorithm).iter().any(|&i| {
            preset.operators.get(i).is_some_and(|op| {
                op.waveform == Waveform::SuperSaw && op.unison >= 5 && op.level >= 0.35
            })
        })
}

fn is_thickened(preset: &fm_synth::Preset) -> bool {
    audible_carrier_octave(preset) || octave_chorus(preset) || thick_supersaw_octave(preset)
}

fn hann_window(buf: &[f32]) -> Vec<f32> {
    let n = buf.len() as f32;
    buf.iter()
        .enumerate()
        .map(|(i, &x)| {
            let w = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / n).cos();
            x * w
        })
        .collect()
}

fn goertzel_power(buf: &[f32], freq: f32) -> f64 {
    let n = buf.len();
    if n == 0 {
        return 0.0;
    }
    let k = (n as f64 * f64::from(freq) / f64::from(SR)).round();
    let w = 2.0 * std::f64::consts::PI * k / n as f64;
    let coeff = 2.0 * w.cos();
    let mut s1 = 0.0;
    let mut s2 = 0.0;
    for &x in buf {
        let s0 = f64::from(x) + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    s1 * s1 + s2 * s2 - coeff * s1 * s2
}

fn render_lead(id: &str, secs: f64) -> (fm_synth::Preset, Vec<f32>, f32) {
    let preset = load_factory(id).expect(id);
    let f0 = midi_to_hz(preset.default_note) as f32;
    let buf = render(
        &preset,
        &RenderParams {
            frequency_hz: f64::from(f0),
            duration_secs: secs,
            velocity: 0.9,
            sample_rate: SR,
        },
    )
    .expect(id);
    (preset, buf, f0)
}

fn body_window<'a>(id: &str, buf: &'a [f32], secs: f64) -> &'a [f32] {
    let n = buf.len();
    let (t0, t1) = if id == "ld-reverse" {
        (0.35, secs.min(0.70))
    } else if id.contains("pluck") || id.contains("mallet") || id.contains("bell") {
        (0.02, 0.16)
    } else {
        (0.06, 0.40)
    };
    let a = ((t0 * f64::from(SR)) as usize).min(n);
    let b = ((t1 * f64::from(SR)) as usize).min(n);
    assert!(b > a + 64, "{id} body window too short");
    &buf[a..b]
}

#[test]
fn factory_leads_have_octave_unison_or_chorus() {
    let ids = ld_folder_ids();
    assert!(
        ids.len() >= 50,
        "expected the ld folder bank, got {}: {ids:?}",
        ids.len()
    );
    let mut thin = Vec::new();
    for id in &ids {
        let preset = load_factory(id).expect(id);
        if !is_thickened(&preset) {
            thin.push(*id);
        }
        assert!(
            (8.0..=8.5).contains(&preset.default_duration),
            "{id} duration {} must stay a usable ~8.2 s lead",
            preset.default_duration
        );
    }
    assert!(
        thin.is_empty(),
        "leads still thin (no audible ratio-2 carrier, octave chorus, or octave supersaw): {thin:?}"
    );
}

#[test]
fn representative_leads_have_octave_energy() {
    // pluck / supersaw / sine / growl / chip / frenchcore / anthem
    const IDS: [&str; 7] = [
        "ld-fm-pluck",
        "ld-supersaw",
        "ld-sine",
        "ld-growl",
        "ld-chip",
        "ld-frenchcore",
        "ld-anthem",
    ];
    for id in IDS {
        let (_preset, buf, f0) = render_lead(id, 0.85);
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");
        assert!(rms(&buf) > 0.01, "{id} near-silence rms={}", rms(&buf));
        let body = hann_window(body_window(id, &buf, 0.85));
        let fund = goertzel_power(&body, f0);
        let oct = goertzel_power(&body, f0 * 2.0);
        let half = goertzel_power(&body, f0 * 0.5);
        assert!(fund > 0.0, "{id} missing fundamental at {f0} Hz (p={fund})");
        assert!(
            oct > fund * 0.08,
            "{id} octave ({}) too weak vs fund {} (ratio {})",
            f0 * 2.0,
            fund,
            oct / fund.max(1e-12)
        );
        // Octave-down may exist but must not become the body of the note.
        assert!(
            half < fund * 1.15,
            "{id} half-octave {} overpowered the lead fund {}",
            half,
            fund
        );
    }
}

#[test]
fn lead_fm_pluck_stays_c3_rooted_but_has_octave_air() {
    let (_preset, buf, f0) = render_lead("lead-fm-pluck", 0.45);
    assert!((f0 - 130.81).abs() < 0.05);
    let body = hann_window(body_window("lead-fm-pluck", &buf, 0.45));
    let c3 = goertzel_power(&body, f0);
    let c4 = goertzel_power(&body, f0 * 2.0);
    assert!(
        c3 > c4 * 2.0,
        "lead-fm-pluck must stay C3, not flip to C4 (c3={c3}, c4={c4})"
    );
    assert!(
        c4 > c3 * 0.02,
        "lead-fm-pluck octave air missing (c3={c3}, c4={c4})"
    );
}

#[test]
fn factory_leads_do_not_steal_kick_sub() {
    const IDS: [&str; 8] = [
        "ld-sine",
        "ld-supersaw",
        "ld-growl",
        "ld-frenchcore",
        "ld-anthem",
        "ld-chip",
        "ld-fm-pluck",
        "ld-octave",
    ];
    for id in IDS {
        let preset = load_factory(id).expect(id);
        let export = ExportParams {
            sample_rate: SR,
            velocity: 0.9,
            duration: Some(0.7),
            ..ExportParams::default()
        };
        let (_buf, analysis) =
            fm_synth::analyze_preset(id, &preset, &export, Some(preset.description.as_str()))
                .expect(id);
        let b = &analysis.report.band_energy;
        assert!(
            b.sub_20_80 < 0.28,
            "{id} sub_20_80 {} looks like a kick/bass (leads may HP, not own the sub)",
            b.sub_20_80
        );
        assert!(
            b.mid_250_2000 + b.high_2000_plus > b.sub_20_80,
            "{id} mid/high {} should beat sub {}",
            b.mid_250_2000 + b.high_2000_plus,
            b.sub_20_80
        );
    }
}

#[test]
fn locked_fifth_and_major_stabs_keep_partial_counts() {
    let fifth = load_factory("stab-fm-fifth").unwrap();
    let live: Vec<_> = fifth
        .operators
        .iter()
        .filter(|op| op.level > 1e-6)
        .collect();
    assert_eq!(live.len(), 2, "hollow fifth must stay two live operators");
    assert!(octave_chorus(&fifth));

    let major = load_factory("stab-fm-major").unwrap();
    let live: Vec<_> = major
        .operators
        .iter()
        .filter(|op| op.level > 1e-6)
        .collect();
    assert_eq!(live.len(), 3, "major triad must stay three live operators");
    assert!(octave_chorus(&major));
}

#[test]
fn analyze_representative_leads_show_octave_peak() {
    const IDS: [&str; 5] = [
        "ld-sine",
        "ld-anthem",
        "ld-chip",
        "ld-supersaw",
        "ld-octave",
    ];
    for id in IDS {
        let (preset, buf, f0) = render_lead(id, 0.6);
        let a = analyze_buffer(
            &buf,
            SR,
            &AnalyzeOpts {
                preset_id: Some(id.into()),
                description: Some(preset.description.clone()),
                ..AnalyzeOpts::default()
            },
        )
        .expect(id);
        let oct_hz = f0 * 2.0;
        let near_oct = a.report.peaks_hz.iter().any(|p| {
            let ratio = p.hz / oct_hz;
            (0.92..=1.08).contains(&ratio) && p.db > -24.0
        });
        let body = hann_window(body_window(id, &buf, 0.6));
        let oct = goertzel_power(&body, oct_hz);
        let fund = goertzel_power(&body, f0);
        assert!(
            near_oct || oct > fund * 0.10,
            "{id} spectrogram should show octave energy near {oct_hz} Hz (peaks {:?}, oct/fund={})",
            a.report.peaks_hz,
            oct / fund.max(1e-12)
        );
    }
}
