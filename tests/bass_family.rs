//! Distorted / mid-bass factory shots: ~120 Hz body, ~5 s tail, highs die in ~3 s.
//!
//! Character (wobble, growl, neuro, frenchcore, metal) sits on top of that
//! low-bass band. `reese-mid` is explicit mid glue and is not in this set.

use fm_synth::{
    analyze_buffer, analyze_preset, load_factory, AnalysisReport, AnalyzeOpts, ExportParams,
    FilterType, Waveform,
};

const SR: u32 = 44_100;
const BODY_IDS: [&str; 8] = [
    "bs-wobble",
    "growl-bass",
    "bs-growl-2",
    "bp-growl",
    "bs-reese-neuro",
    "bs-frenchcore",
    "bs-metal-fm",
    "bs-dist-square",
];

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

fn window_report(buf: &[f32], t0: f64, t1: f64) -> AnalysisReport {
    let a = ((t0 * f64::from(SR)) as usize).min(buf.len());
    let b = ((t1 * f64::from(SR)) as usize).min(buf.len());
    assert!(b > a + 256, "window {t0}..{t1} too short");
    analyze_buffer(&buf[a..b], SR, &AnalyzeOpts::default())
        .unwrap()
        .report
}

fn has_peak_near_120(report: &AnalysisReport) -> bool {
    report
        .peaks_hz
        .iter()
        .any(|p| (95.0..=155.0).contains(&p.hz) && p.db > -18.0)
}

fn assert_bass_body(id: &str, report: &AnalysisReport) {
    let b = &report.band_energy;
    assert!(
        b.bass_80_250 >= 0.28,
        "{id} bass_80_250 {} too thin for a bass (want ~120 Hz body)",
        b.bass_80_250
    );
    assert!(
        b.sub_20_80 + b.bass_80_250 > b.mid_250_2000 * 0.85,
        "{id} low end should compete with mid (sub={}, bass={}, mid={})",
        b.sub_20_80,
        b.bass_80_250,
        b.mid_250_2000
    );
    assert!(
        has_peak_near_120(report),
        "{id} should show a spectral peak near 120 Hz, got {:?}",
        report.peaks_hz
    );
}

#[test]
fn distorted_basses_have_120hz_body() {
    for id in BODY_IDS {
        let (_preset, _buf, report) = analyze_id(id);
        assert_bass_body(id, &report);
    }
}

#[test]
fn distorted_basses_last_about_five_seconds() {
    for id in BODY_IDS {
        let (preset, _buf, report) = analyze_id(id);
        assert!(
            (4.8..=5.2).contains(&preset.default_duration),
            "{id} default_duration {} should be ~5 s",
            preset.default_duration
        );
        assert!(
            (4.8..=5.2).contains(&report.duration_secs),
            "{id} rendered duration {} should be ~5 s",
            report.duration_secs
        );
        assert!(
            report.energy_at_frac.t85 > 0.02,
            "{id} t=0.85 energy {} is silence — need a 5 s bass tail",
            report.energy_at_frac.t85
        );
        assert!(
            report.energy_at_frac.t85 > report.energy_at_frac.t20 * 0.15,
            "{id} late energy {} vs early {} — tail died too soon",
            report.energy_at_frac.t85,
            report.energy_at_frac.t20
        );
    }
}

#[test]
fn distorted_basses_highs_decay_over_about_three_seconds() {
    for id in BODY_IDS {
        let (preset, buf, report) = analyze_id(id);
        assert!(
            (2.6..=3.4).contains(&preset.filter.decay),
            "{id} filter decay {} should be ~3 s",
            preset.filter.decay
        );
        let early = window_report(&buf, 0.08, 0.9);
        let late = window_report(&buf, 3.3, 4.6);
        let early_bright = early.band_energy.mid_250_2000 + early.band_energy.high_2000_plus;
        let late_bright = late.band_energy.mid_250_2000 + late.band_energy.high_2000_plus;
        assert!(
            late_bright < early_bright * 0.85,
            "{id} highs should roll off over ~3 s (early mid+high={}, late={})",
            early_bright,
            late_bright
        );
        assert!(
            late.spectral_centroid_hz < early.spectral_centroid_hz * 0.92,
            "{id} late centroid {} should be darker than early {}",
            late.spectral_centroid_hz,
            early.spectral_centroid_hz
        );
        assert!(
            late.band_energy.sub_20_80 + late.band_energy.bass_80_250 > 0.45,
            "{id} late window lost the ~120 Hz bed (sub={}, bass={})",
            late.band_energy.sub_20_80,
            late.band_energy.bass_80_250
        );
        assert_bass_body(id, &report);
    }
}

#[test]
fn wobble_keeps_pitch_lfo_and_filter_motion() {
    let (preset, _buf, report) = analyze_id("bs-wobble");
    assert!(
        preset.lfo.rate_hz > 4.0 && preset.lfo.depth_cents > 20.0,
        "bs-wobble should keep a fast pitch LFO"
    );
    assert_eq!(preset.filter.kind, FilterType::Bandpass);
    assert!(
        (100.0..=180.0).contains(&preset.filter.cutoff),
        "wobble BP should sit near the 120 Hz bed, got {}",
        preset.filter.cutoff
    );
    assert!(
        preset.filter.env_amount > 1.0,
        "wobble should still move the BP"
    );
    assert_bass_body("bs-wobble", &report);
}

#[test]
fn growl_bass_has_body_and_mid_throat() {
    let (preset, buf, report) = analyze_id("growl-bass");
    let early = window_report(&buf, 0.08, 0.9);
    assert!(
        early.band_energy.mid_250_2000 >= 0.10,
        "growl-bass early mid {} too dull (need formant/throat)",
        early.band_energy.mid_250_2000
    );
    assert!(
        preset.operators[2].ratio > 1.2 && preset.operators[2].ratio < 1.8,
        "growl-bass should keep an inharmonic formant op"
    );
    assert_bass_body("growl-bass", &report);
}

#[test]
fn growl_variants_stay_distinct() {
    let (a, abuf, ra) = analyze_id("growl-bass");
    let (b, bbuf, rb) = analyze_id("bs-growl-2");
    let (c, _cbuf, rc) = analyze_id("bp-growl");
    assert_ne!(a.operators[2].ratio, b.operators[2].ratio);
    assert_eq!(c.filter.kind, FilterType::Bandpass);
    assert!(c.filter.env_amount > a.filter.env_amount);
    let early_a = window_report(&abuf, 0.08, 0.9);
    let early_b = window_report(&bbuf, 0.08, 0.9);
    assert!(
        (early_b.band_energy.mid_250_2000 - early_a.band_energy.mid_250_2000).abs() > 0.03
            || (early_b.spectral_centroid_hz - early_a.spectral_centroid_hz).abs() > 30.0,
        "growl variants should differ early (growl-bass mid={}, growl-2 mid={})",
        early_a.band_energy.mid_250_2000,
        early_b.band_energy.mid_250_2000
    );
    assert_bass_body("growl-bass", &ra);
    assert_bass_body("bs-growl-2", &rb);
    assert_bass_body("bp-growl", &rc);
}

#[test]
fn neuro_and_frenchcore_keep_mid_character() {
    let (_n, nbuf, neuro) = analyze_id("bs-reese-neuro");
    let (_f, fbuf, french) = analyze_id("bs-frenchcore");
    let (_m, mbuf, metal) = analyze_id("bs-metal-fm");
    let early_n = window_report(&nbuf, 0.08, 0.9);
    let early_f = window_report(&fbuf, 0.08, 0.9);
    let early_m = window_report(&mbuf, 0.08, 0.9);
    assert!(
        early_n.band_energy.mid_250_2000 >= 0.10,
        "neuro throat too thin early ({})",
        early_n.band_energy.mid_250_2000
    );
    assert!(
        early_f.band_energy.mid_250_2000 >= 0.08,
        "frenchcore mid aggression too thin early ({})",
        early_f.band_energy.mid_250_2000
    );
    assert!(
        early_m.spectral_centroid_hz > early_n.spectral_centroid_hz * 0.8,
        "metal-fm should still read brighter/ringier than neuro early"
    );
    assert_bass_body("bs-reese-neuro", &neuro);
    assert_bass_body("bs-frenchcore", &french);
    assert_bass_body("bs-metal-fm", &metal);
}

#[test]
fn dist_square_fills_even_harmonic_hole() {
    let (preset, _buf, report) = analyze_id("bs-dist-square");
    assert_eq!(preset.operators[0].waveform, Waveform::Pulse);
    assert!(
        (1.7..=2.0).contains(&preset.operators[1].ratio),
        "dist-square OP2 should supply the ~120 Hz even partial, got {}",
        preset.operators[1].ratio
    );
    assert_bass_body("bs-dist-square", &report);
}
