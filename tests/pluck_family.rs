//! Family checks on the real factory pl-* load + render path.
//!
//! These encode "the rendered one-shot matches the existing description":
//! ~600ms audible body (not silent padding), one-shot (not a pad/kick),
//! and measurable character (hollow fifth, major triad, reverse attack, …).

use fm_synth::{
    analyze_preset, factory_ids, load_factory, midi_to_hz, render, rms, AnalysisReport,
    ExportParams, FilterType, RenderParams, Waveform,
};

const SR: u32 = 48_000;

fn pluck_ids() -> Vec<&'static str> {
    factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("pl-"))
        .collect()
}

fn render_default(id: &str) -> (fm_synth::Preset, Vec<f32>) {
    let preset = load_factory(id).expect(id);
    let buf = render(
        &preset,
        &RenderParams {
            frequency_hz: midi_to_hz(preset.default_note),
            duration_secs: preset.default_duration,
            velocity: 0.9,
            sample_rate: SR,
        },
    )
    .expect(id);
    (preset, buf)
}

fn analyze_id(id: &str) -> (fm_synth::Preset, Vec<f32>, AnalysisReport) {
    let preset = load_factory(id).expect(id);
    let export = ExportParams {
        sample_rate: SR,
        velocity: 0.9,
        ..ExportParams::default()
    };
    let intent = preset.description.clone();
    let (buf, analysis) = analyze_preset(id, &preset, &export, Some(intent.as_str())).expect(id);
    (preset, buf, analysis.report)
}

fn window_rms(buf: &[f32], t0: f64, t1: f64) -> f32 {
    let a = ((t0 * f64::from(SR)) as usize).min(buf.len());
    let b = ((t1 * f64::from(SR)) as usize).min(buf.len());
    if a >= b {
        return 0.0;
    }
    rms(&buf[a..b])
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

#[test]
fn factory_pl_plucks_are_thirty_about_600ms_and_audible() {
    let ids = pluck_ids();
    assert_eq!(
        ids.len(),
        30,
        "expected exactly 30 pl-* factory plucks, got {}: {ids:?}",
        ids.len()
    );

    for id in ids {
        let (preset, buf, report) = analyze_id(id);
        assert!(
            (0.50..=0.70).contains(&preset.default_duration),
            "{id} default_duration {} is not ~0.60s",
            preset.default_duration
        );
        assert!(
            preset.description.contains("0.60秒"),
            "{id} description lost the new length: {}",
            preset.description
        );
        assert!(
            !preset.description.contains("極短い"),
            "{id} still claims 極短い at 600ms: {}",
            preset.description
        );
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");
        assert!(rms(&buf) > 0.01, "{id} near-silence rms={}", rms(&buf));
        let max_sustain = preset
            .operators
            .iter()
            .filter(|op| op.level > 1e-6)
            .map(|op| op.sustain)
            .fold(0.0f32, |a, s| a.max(s));
        assert!(
            max_sustain <= 0.08,
            "{id} live-op sustain {max_sustain} looks like a held lead"
        );
        assert!(
            report.band_energy.sub_20_80 < 0.18,
            "{id} is kick-like (sub_20_80={})",
            report.band_energy.sub_20_80
        );
    }
}

#[test]
fn pluck_bodies_fill_the_shot_then_decay() {
    let mut fails: Vec<String> = Vec::new();
    for id in pluck_ids() {
        let (_preset, buf) = render_default(id);
        let early = window_rms(&buf, 0.00, 0.08);
        let mid = window_rms(&buf, 0.35, 0.50);
        let belly = window_rms(&buf, 0.16, 0.30);
        let tail = window_rms(&buf, 0.50, 0.60);

        if id == "pl-reverse-swell" {
            let swell = window_rms(&buf, 0.14, 0.26);
            let start = window_rms(&buf, 0.00, 0.05);
            if swell <= start * 1.6 {
                fails.push(format!("{id} should swell (start={start}, swell={swell})"));
            }
            if tail >= swell * 0.75 {
                fails.push(format!(
                    "{id} reverse should die after the swell (swell={swell}, tail={tail})"
                ));
            }
            continue;
        }

        if early <= 0.02 {
            fails.push(format!("{id} missing attack (early rms={early})"));
        }
        if !(mid > 0.010 && mid > early * 0.04) {
            fails.push(format!(
                "{id} second half is padded silence (early={early}, mid={mid})"
            ));
        }
        if tail >= belly * 0.95 {
            fails.push(format!(
                "{id} tail is not decaying (belly={belly}, tail={tail})"
            ));
        }
    }
    assert!(
        fails.is_empty(),
        "pluck length/decay mismatches:\n  {}",
        fails.join("\n  ")
    );
}

#[test]
fn reverse_swell_attacks_late_then_dies() {
    let (preset, _buf, report) = analyze_id("pl-reverse-swell");
    assert!(
        report.attack_ms > 70.0,
        "reverse attack_ms {} should be a slow swell, not a pluck click",
        report.attack_ms
    );
    assert!(
        preset.operators[0].attack > 0.08,
        "reverse carrier attack {} is too snappy",
        preset.operators[0].attack
    );
    assert!(
        report.energy_at_frac.t85 < report.energy_at_frac.t20 * 0.9
            || report.energy_at_frac.t85 < report.energy_at_frac.t50,
        "reverse should die by the end (t20={}, t50={}, t85={})",
        report.energy_at_frac.t20,
        report.energy_at_frac.t50,
        report.energy_at_frac.t85
    );
}

#[test]
fn stab_fifth_is_hollow_fifth_not_major() {
    let (preset, buf, _report) = analyze_id("pl-stab-fifth");
    assert_eq!(preset.default_note, 55);
    let f0 = midi_to_hz(preset.default_note) as f32;
    let body = hann_window(&buf);
    let root = goertzel_power(&body, f0);
    let third = goertzel_power(&body, f0 * 1.25);
    let fifth = goertzel_power(&body, f0 * 1.5);
    assert!(
        fifth > third * 3.0,
        "pl-stab-fifth leaked a major third (root={root}, third={third}, fifth={fifth})"
    );
    assert!(
        fifth > root * 0.15 && root > fifth * 0.15,
        "pl-stab-fifth should be C+G (root={root}, fifth={fifth})"
    );
    let live: Vec<_> = preset
        .operators
        .iter()
        .filter(|op| op.level > 1e-6)
        .map(|op| op.ratio)
        .collect();
    assert!(live.contains(&1.0) && live.contains(&1.5), "ratios {live:?}");
    assert!(
        !live.iter().any(|&r| (r - 1.25).abs() < 0.02),
        "fifth stab must not carry 5:4 ({live:?})"
    );
}

#[test]
fn stab_major_is_c_e_g() {
    let (preset, buf, _report) = analyze_id("pl-stab-major");
    assert_eq!(preset.default_note, 55);
    let f0 = midi_to_hz(preset.default_note) as f32;
    let body = hann_window(&buf);
    let root = goertzel_power(&body, f0);
    let third = goertzel_power(&body, f0 * 1.25);
    let fifth = goertzel_power(&body, f0 * 1.5);
    assert!(
        third > root * 0.12,
        "pl-stab-major missing the major third (root={root}, third={third})"
    );
    assert!(
        fifth > root * 0.12,
        "pl-stab-major missing the fifth (root={root}, fifth={fifth})"
    );
    let live: Vec<_> = preset
        .operators
        .iter()
        .filter(|op| op.level > 1e-6)
        .map(|op| op.ratio)
        .collect();
    assert!(
        live.iter().any(|&r| (r - 1.25).abs() < 0.02)
            && live.iter().any(|&r| (r - 1.5).abs() < 0.02),
        "major stab ratios {live:?}"
    );
}

#[test]
fn acid_is_reso_mid_not_a_kick() {
    let (preset, _buf, report) = analyze_id("pl-acid-short");
    assert_eq!(preset.filter.kind, FilterType::Lowpass);
    assert!(
        preset.filter.resonance > 0.6,
        "acid Q {} is not a 303-style reso LP",
        preset.filter.resonance
    );
    assert_eq!(preset.default_note, 48);
    assert!(
        report.band_energy.sub_20_80 < 0.12,
        "acid became a kick (sub={})",
        report.band_energy.sub_20_80
    );
    assert!(
        report.band_energy.mid_250_2000 + report.band_energy.high_2000_plus
            > report.band_energy.sub_20_80 * 3.0,
        "acid should be a bright mid, not sub ({:?})",
        report.band_energy
    );
    assert!(
        report.spectral_centroid_hz > 300.0,
        "acid centroid {} is too low for a reso mid",
        report.spectral_centroid_hz
    );
}

#[test]
fn bass_pluck_is_dark_c3() {
    let (preset, _buf, report) = analyze_id("pl-bass-pluck");
    let (_house, _hbuf, house) = analyze_id("pl-house-dry");
    assert_eq!(preset.default_note, 48, "bass pluck must be C3");
    assert_eq!(preset.filter.kind, FilterType::Lowpass);
    assert!(
        preset.filter.cutoff < 400.0,
        "bass LP cutoff {} is not low",
        preset.filter.cutoff
    );
    assert!(
        report.spectral_centroid_hz < house.spectral_centroid_hz,
        "bass centroid {} should sit below house-dry {}",
        report.spectral_centroid_hz,
        house.spectral_centroid_hz
    );
    assert!(
        report.band_energy.sub_20_80 < 0.18,
        "bass pluck dumped into kick sub ({})",
        report.band_energy.sub_20_80
    );
    assert!(
        report.band_energy.bass_80_250 + report.band_energy.mid_250_2000 > 0.55,
        "bass pluck missing body ({:?})",
        report.band_energy
    );
}

#[test]
fn perc_click_is_bright_hp() {
    let (preset, _buf, report) = analyze_id("pl-perc-click");
    assert_eq!(preset.filter.kind, FilterType::Highpass);
    assert!(
        preset.filter.cutoff > 800.0,
        "perc-click HP cutoff {} is too low",
        preset.filter.cutoff
    );
    assert!(
        report.spectral_centroid_hz > 1_500.0,
        "perc-click centroid {} is not a bright click",
        report.spectral_centroid_hz
    );
    assert!(
        report.band_energy.high_2000_plus > 0.30,
        "perc-click high {} too small",
        report.band_energy.high_2000_plus
    );
}

#[test]
fn house_dry_is_single_saw_not_a_stack() {
    let (preset, _buf, report) = analyze_id("pl-house-dry");
    assert_eq!(preset.algorithm.id(), 8);
    let live: Vec<_> = preset
        .operators
        .iter()
        .filter(|op| op.level > 1e-6)
        .collect();
    assert_eq!(live.len(), 1, "house-dry should be a single saw");
    assert_eq!(live[0].waveform, Waveform::Saw);
    assert_eq!(preset.filter.kind, FilterType::Lowpass);
    assert!(
        report.band_energy.sub_20_80 < 0.10,
        "house-dry has kick sub ({})",
        report.band_energy.sub_20_80
    );
}
