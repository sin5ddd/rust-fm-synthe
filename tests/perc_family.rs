//! Family checks on the real factory load + render path.
//!
//! These encode "the rendered one-shot matches the existing description":
//! sand/hats sizzle in the mid-high band, membranes are mid bodies not 808s,
//! cowbell is two-partial metal, claps are slap clusters, open hats outlive closed.

use fm_synth::{
    analyze_preset, factory_ids, load_factory, midi_to_hz, render, rms, AnalysisReport,
    ExportParams, RenderParams,
};

const SR: u32 = 48_000;

fn perc_bank_ids() -> Vec<&'static str> {
    let mut ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("pc-"))
        .collect();
    ids.extend(["cp-house", "glass-hit", "metallic-hit"]);
    ids
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

fn render_secs(id: &str, secs: f64) -> Vec<f32> {
    let preset = load_factory(id).expect(id);
    render(
        &preset,
        &RenderParams {
            frequency_hz: midi_to_hz(preset.default_note),
            duration_secs: secs,
            velocity: 0.9,
            sample_rate: SR,
        },
    )
    .expect(id)
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

fn band_energy(buf: &[f32], lo: f32, hi: f32, step: f32) -> f64 {
    let body = hann_window(buf);
    let mut e = 0.0;
    let mut f = lo;
    while f < hi {
        e += goertzel_power(&body, f);
        f += step;
    }
    e
}

fn is_thud(id: &str) -> bool {
    id == "pc-foley-thud"
}

fn is_open_hat(id: &str) -> bool {
    matches!(id, "pc-hat-open" | "pc-hat-dnb-op" | "pc-hat-fc-op")
}

#[test]
fn factory_perc_bank_shots_are_audible_finite_and_not_pads() {
    let ids = perc_bank_ids();
    let pc: Vec<_> = ids
        .iter()
        .copied()
        .filter(|id| id.starts_with("pc-"))
        .collect();
    assert_eq!(
        pc.len(),
        50,
        "expected 50 pc-* ids, got {}: {pc:?}",
        pc.len()
    );
    assert!(ids.contains(&"cp-house"));
    assert!(ids.contains(&"glass-hit"));
    assert!(ids.contains(&"metallic-hit"));

    for id in ids {
        let (preset, buf, report) = analyze_id(id);
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");
        assert!(rms(&buf) > 0.01, "{id} near-silence rms={}", rms(&buf));
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
        assert!(
            preset.default_duration < 1.5,
            "{id} default_duration {} looks like a pad",
            preset.default_duration
        );
        if is_open_hat(id) || id == "pc-ride-fm" {
            assert!(
                (0.85..=1.25).contains(&preset.default_duration),
                "{id} open/ride length {} should stay ~1 s",
                preset.default_duration
            );
        }
        let sub = report.band_energy.sub_20_80;
        if is_thud(id) {
            assert!(sub < 0.18, "{id} thud still has kick-like sub ({sub})");
        } else {
            assert!(
                sub < 0.14,
                "{id} is a sub kick (sub_20_80={sub}) but description is not thud"
            );
        }
        let mid = report.band_energy.mid_250_2000;
        let high = report.band_energy.high_2000_plus;
        assert!(
            mid + high > sub,
            "{id} energy is sub-dominated (sub={sub}, mid={mid}, high={high})"
        );
    }
}

#[test]
fn sand_family_sizzles_in_mid_high_not_nyquist_or_sub() {
    let hats = [
        "pc-hat-closed",
        "pc-hat-house",
        "pc-hat-dnb-cl",
        "pc-hat-fc",
        "pc-hat-pedal",
        "pc-hat-tight",
        "pc-hat-dark",
        "pc-hat-noise",
        "pc-hat-chip",
        "pc-hat-open",
        "pc-hat-dnb-op",
        "pc-hat-fc-op",
    ];
    let shakers = ["pc-shaker", "pc-shaker-short", "pc-cabasa"];
    for id in hats.iter().chain(shakers.iter()) {
        let (_preset, buf, report) = analyze_id(id);
        assert!(
            report.band_energy.sub_20_80 < 0.06,
            "{id} sand/hat has sub {sub}",
            sub = report.band_energy.sub_20_80
        );
        assert!(
            report.spectral_centroid_hz > 2_400.0,
            "{id} centroid {} too low for sand/hat",
            report.spectral_centroid_hz
        );
        assert!(
            report.band_energy.high_2000_plus > 0.42,
            "{id} high_2000_plus {} too small",
            report.band_energy.high_2000_plus
        );
        let sizzle = band_energy(&buf, 6_000.0, 10_000.0, 80.0);
        let air = band_energy(&buf, 14_000.0, 20_000.0, 80.0);
        assert!(
            sizzle > air * 1.15,
            "{id} Nyquist tick (sizzle={sizzle}, air={air})"
        );
    }
}

#[test]
fn open_hats_still_speak_after_closed_hats_die() {
    let pairs = [
        ("pc-hat-closed", "pc-hat-open"),
        ("pc-hat-dnb-cl", "pc-hat-dnb-op"),
        ("pc-hat-fc", "pc-hat-fc-op"),
    ];
    for (closed_id, open_id) in pairs {
        let closed = render_secs(closed_id, 1.2);
        let open = render_secs(open_id, 1.2);
        let c_late = window_rms(&closed, 0.22, 0.45);
        let o_late = window_rms(&open, 0.22, 0.45);
        assert!(
            o_late > c_late * 2.5,
            "{open_id} should still sizzle after {closed_id} died (open={o_late}, closed={c_late})"
        );
        let c_early = window_rms(&closed, 0.0, 0.08);
        assert!(c_early > 0.02, "{closed_id} missing attack (rms={c_early})");
    }
}

#[test]
fn membranes_are_mid_bodies_not_808_kicks() {
    let ids = [
        "pc-tom-lo",
        "pc-tom-mid",
        "pc-tom-hi",
        "pc-conga-lo",
        "pc-conga-hi",
        "pc-bongo-lo",
        "pc-bongo-hi",
        "pc-foley-thud",
    ];
    for id in ids {
        let (_preset, _buf, report) = analyze_id(id);
        let sub = report.band_energy.sub_20_80;
        let mid = report.band_energy.mid_250_2000;
        let bass = report.band_energy.bass_80_250;
        let cap = if is_thud(id) { 0.16 } else { 0.12 };
        assert!(sub < cap, "{id} membrane has 808-like sub ({sub})");
        assert!(mid > sub * 1.4, "{id} mid {mid} not above sub {sub}");
        assert!(
            bass + mid > 0.45,
            "{id} missing membrane body (bass={bass}, mid={mid})"
        );
        assert!(
            report.spectral_centroid_hz > 150.0,
            "{id} centroid {} looks like a kick",
            report.spectral_centroid_hz
        );
        if let Some(pitch) = report.pitch.as_ref() {
            assert!(
                pitch.drop_semitones < 9.0,
                "{id} 808-like pitch dump ({} st)",
                pitch.drop_semitones
            );
        }
        if let Some(p0) = report.peaks_hz.first() {
            assert!(
                p0.hz > 90.0,
                "{id} strongest peak {} Hz looks like a sub kick, not a membrane body",
                p0.hz
            );
        }
    }
}

#[test]
fn hi_vs_lo_membranes_keep_register_split() {
    let pairs = [
        ("pc-tom-lo", "pc-tom-hi"),
        ("pc-conga-lo", "pc-conga-hi"),
        ("pc-bongo-lo", "pc-bongo-hi"),
        ("pc-agogo-lo", "pc-agogo-hi"),
    ];
    for (lo_id, hi_id) in pairs {
        let (_, _, lo) = analyze_id(lo_id);
        let (_, _, hi) = analyze_id(hi_id);
        assert!(
            hi.spectral_centroid_hz > lo.spectral_centroid_hz * 1.12,
            "{hi_id} centroid {} should sit above {lo_id} {}",
            hi.spectral_centroid_hz,
            lo.spectral_centroid_hz
        );
    }
    let (_, _, mid) = analyze_id("pc-tom-mid");
    let (_, _, lo) = analyze_id("pc-tom-lo");
    let (_, _, hi) = analyze_id("pc-tom-hi");
    assert!(
        mid.spectral_centroid_hz > lo.spectral_centroid_hz,
        "tom-mid {} should be above tom-lo {}",
        mid.spectral_centroid_hz,
        lo.spectral_centroid_hz
    );
    assert!(
        hi.spectral_centroid_hz > mid.spectral_centroid_hz,
        "tom-hi {} should be above tom-mid {}",
        hi.spectral_centroid_hz,
        mid.spectral_centroid_hz
    );
}

#[test]
fn cowbell_is_two_mid_partials_not_clave() {
    let (_p, buf, report) = analyze_id("pc-cowbell");
    assert!(
        report.spectral_flatness < 0.35,
        "cowbell should be tonal, flatness {}",
        report.spectral_flatness
    );
    assert!(
        report.band_energy.sub_20_80 < 0.08,
        "cowbell sub {}",
        report.band_energy.sub_20_80
    );
    let body = hann_window(&buf);
    let p587 = goertzel_power(&body, 587.0);
    let p845 = goertzel_power(&body, 845.0);
    let p200 = goertzel_power(&body, 200.0);
    let p2350 = goertzel_power(&body, 2350.0);
    assert!(
        p587 > p200 * 3.0 && p845 > p200 * 3.0,
        "cowbell missing 587/845 pair (587={p587}, 845={p845}, 200={p200})"
    );
    assert!(
        p587 + p845 > p2350,
        "cowbell looks like a clave click (2350={p2350}, pair={})",
        p587 + p845
    );

    let (_c, clave_buf, clave) = analyze_id("pc-clave");
    assert!(
        clave.spectral_centroid_hz > report.spectral_centroid_hz * 1.15,
        "clave centroid {} should sit above cowbell {}",
        clave.spectral_centroid_hz,
        report.spectral_centroid_hz
    );
    let clave_body = hann_window(&clave_buf);
    let c2350 = goertzel_power(&clave_body, 2350.0);
    let c587 = goertzel_power(&clave_body, 587.0);
    assert!(
        c2350 > c587,
        "clave should be a wood click near 2.3 kHz, not cowbell (2350={c2350}, 587={c587})"
    );
}

#[test]
fn claps_are_slap_noise_clusters_not_snare_bodies() {
    for id in ["cp-house", "pc-clap-dry", "pc-clap-gate", "pc-clap-room"] {
        let (preset, buf, report) = analyze_id(id);
        assert!(preset.noise.is_active(), "{id} clap needs the noise bus");
        assert!(
            report.band_energy.sub_20_80 < 0.08,
            "{id} clap has snare-like sub {}",
            report.band_energy.sub_20_80
        );
        assert!(
            report.band_energy.mid_250_2000 + report.band_energy.high_2000_plus > 0.7,
            "{id} clap missing slap/noise energy"
        );
        let early = window_rms(&buf, 0.0, 0.012);
        let cluster = window_rms(&buf, 0.016, 0.055);
        assert!(
            cluster > early * 0.8,
            "{id} should be a delayed slap cluster (cluster={cluster}, early={early})"
        );
        if let Some(pitch) = report.pitch.as_ref() {
            assert!(
                pitch.start_hz > 280.0 || pitch.confidence < 0.5,
                "{id} looks like a pitched snare body ({} Hz)",
                pitch.start_hz
            );
        }
    }
    let (_, _, dry) = analyze_id("pc-clap-dry");
    let (_, _, room) = analyze_id("pc-clap-room");
    assert!(
        room.duration_secs > dry.duration_secs,
        "room clap should outlast dry (room={}, dry={})",
        room.duration_secs,
        dry.duration_secs
    );
}

#[test]
fn guiro_and_scratch_are_noisy_scrapes_not_bass() {
    for id in ["pc-guiro", "pc-foley-scratch"] {
        let (preset, _buf, report) = analyze_id(id);
        assert!(preset.noise.is_active(), "{id} scrape needs noise");
        assert!(
            report.band_energy.sub_20_80 < 0.08,
            "{id} scrape has bass {sub}",
            sub = report.band_energy.sub_20_80
        );
        assert!(
            report.spectral_flatness > 0.08,
            "{id} should be noisy/scrape, flatness {}",
            report.spectral_flatness
        );
        assert!(
            report.band_energy.mid_250_2000 + report.band_energy.high_2000_plus > 0.7,
            "{id} missing scrape band"
        );
        assert!(
            report.spectral_centroid_hz > 800.0,
            "{id} centroid {} too low for a scrape",
            report.spectral_centroid_hz
        );
    }
}

#[test]
fn ride_is_longer_and_more_metallic_than_closed_hat() {
    let closed = render_secs("pc-hat-closed", 1.2);
    let ride = render_secs("pc-ride-fm", 1.2);
    let c_late = window_rms(&closed, 0.40, 0.80);
    let r_late = window_rms(&ride, 0.40, 0.80);
    assert!(
        r_late > c_late * 2.0,
        "ride should still ring after closed hat (ride={r_late}, hat={c_late})"
    );
    let (_, _, hat) = analyze_id("pc-hat-closed");
    let (_, _, ride_rep) = analyze_id("pc-ride-fm");
    assert!(
        ride_rep.spectral_flatness < hat.spectral_flatness * 0.95
            || ride_rep.band_energy.mid_250_2000 > hat.band_energy.mid_250_2000,
        "ride should be more metallic than a closed hat (flat ride={}, hat={}; mid ride={}, hat={})",
        ride_rep.spectral_flatness,
        hat.spectral_flatness,
        ride_rep.band_energy.mid_250_2000,
        hat.band_energy.mid_250_2000
    );
    assert!(
        ride_rep.duration_secs > 0.9,
        "ride duration {} should stay in the ~1 s class",
        ride_rep.duration_secs
    );
}

#[test]
fn triangle_and_glass_ring_not_hiss_or_kick() {
    for id in [
        "pc-triangle",
        "pc-chime",
        "glass-hit",
        "metallic-hit",
        "pc-agogo-hi",
    ] {
        let (_p, _buf, report) = analyze_id(id);
        assert!(
            report.band_energy.sub_20_80 < 0.08,
            "{id} metal/glass has sub {}",
            report.band_energy.sub_20_80
        );
        assert!(
            report.spectral_flatness < 0.45,
            "{id} should be partials/ring, not a hiss sheet (flatness {})",
            report.spectral_flatness
        );
        assert!(
            report.spectral_centroid_hz > 600.0,
            "{id} centroid {} too low",
            report.spectral_centroid_hz
        );
    }
}

#[test]
fn zap_falls_in_mid_not_as_a_kick() {
    for id in ["pc-zap", "pc-zap-lo"] {
        let (_p, _buf, report) = analyze_id(id);
        assert!(
            report.band_energy.sub_20_80 < 0.10,
            "{id} zap dumped into sub ({})",
            report.band_energy.sub_20_80
        );
        assert!(
            report.band_energy.mid_250_2000 > report.band_energy.sub_20_80,
            "{id} zap mid {} vs sub {}",
            report.band_energy.mid_250_2000,
            report.band_energy.sub_20_80
        );
        if let Some(pitch) = report.pitch.as_ref() {
            assert!(
                pitch.drop_semitones > 1.8 && pitch.start_hz > pitch.end_hz * 1.1,
                "{id} should fall (drop {} st, {} -> {} Hz)",
                pitch.drop_semitones,
                pitch.start_hz,
                pitch.end_hz
            );
            assert!(
                pitch.start_hz > 250.0,
                "{id} fall starts too low ({} Hz)",
                pitch.start_hz
            );
        }
    }
}

#[test]
fn leftover_perc_folder_hits_stay_short_and_audible() {
    for id in ["cp-house", "glass-hit", "metallic-hit"] {
        let (preset, buf) = render_default(id);
        assert!(rms(&buf) > 0.01, "{id} silent");
        assert!(
            preset.default_duration < 1.0,
            "{id} duration {} is not a short hit",
            preset.default_duration
        );
    }
}
