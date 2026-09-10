//! Distorted / mid-bass factory one-shots need a ~120 Hz body.
//!
//! Character (wobble, growl, neuro, frenchcore, metal) sits on top of that
//! low-bass band. `reese-mid` is explicit mid glue and is not in this set.

use fm_synth::{
    analyze_preset, load_factory, AnalysisReport, ExportParams, FilterType, Waveform,
};

const SR: u32 = 44_100;
const BODY_IDS: [&str; 9] = [
    "bs-wobble",
    "growl-bass",
    "bs-growl-2",
    "bp-growl",
    "bs-reese-neuro",
    "bs-frenchcore",
    "bs-metal-fm",
    "bs-dist-square",
    "supersaw-bass",
];

fn analyze_id(id: &str) -> (fm_synth::Preset, AnalysisReport) {
    let preset = load_factory(id).expect(id);
    let export = ExportParams {
        sample_rate: SR,
        velocity: 0.9,
        ..ExportParams::default()
    };
    let intent = preset.description.clone();
    let (_buf, analysis) = analyze_preset(id, &preset, &export, Some(intent.as_str())).expect(id);
    (preset, analysis.report)
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
        let (_preset, report) = analyze_id(id);
        assert_bass_body(id, &report);
    }
}

#[test]
fn wobble_keeps_pitch_lfo_and_bp_motion() {
    let (preset, report) = analyze_id("bs-wobble");
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
    let (preset, report) = analyze_id("growl-bass");
    let b = &report.band_energy;
    assert!(
        b.mid_250_2000 >= 0.12,
        "growl-bass mid {} too dull (need formant/throat)",
        b.mid_250_2000
    );
    assert!(
        preset.operators[2].ratio > 1.2 && preset.operators[2].ratio < 1.8,
        "growl-bass should keep an inharmonic formant op"
    );
    assert_bass_body("growl-bass", &report);
}

#[test]
fn growl_variants_stay_distinct() {
    let (a, ra) = analyze_id("growl-bass");
    let (b, rb) = analyze_id("bs-growl-2");
    let (c, rc) = analyze_id("bp-growl");
    assert_ne!(a.operators[2].ratio, b.operators[2].ratio);
    assert_eq!(c.filter.kind, FilterType::Bandpass);
    assert!(c.filter.env_amount > a.filter.env_amount);
    assert!(
        rb.spectral_centroid_hz > ra.spectral_centroid_hz * 0.9,
        "bs-growl-2 should not be a darker clone of growl-bass"
    );
    assert_bass_body("growl-bass", &ra);
    assert_bass_body("bs-growl-2", &rb);
    assert_bass_body("bp-growl", &rc);
}

#[test]
fn neuro_and_frenchcore_keep_mid_character() {
    let (_n, neuro) = analyze_id("bs-reese-neuro");
    let (_f, french) = analyze_id("bs-frenchcore");
    let (_m, metal) = analyze_id("bs-metal-fm");
    assert!(
        neuro.band_energy.mid_250_2000 >= 0.12,
        "neuro throat too thin ({})",
        neuro.band_energy.mid_250_2000
    );
    assert!(
        french.band_energy.mid_250_2000 >= 0.10,
        "frenchcore mid aggression too thin ({})",
        french.band_energy.mid_250_2000
    );
    assert!(
        metal.spectral_centroid_hz > neuro.spectral_centroid_hz * 0.85,
        "metal-fm should still read brighter/ringier than neuro"
    );
    assert_bass_body("bs-reese-neuro", &neuro);
    assert_bass_body("bs-frenchcore", &french);
    assert_bass_body("bs-metal-fm", &metal);
}

#[test]
fn dist_square_fills_even_harmonic_hole() {
    let (preset, report) = analyze_id("bs-dist-square");
    assert_eq!(preset.operators[0].waveform, Waveform::Pulse);
    assert!(
        (1.7..=2.0).contains(&preset.operators[1].ratio),
        "dist-square OP2 should supply the ~120 Hz even partial, got {}",
        preset.operators[1].ratio
    );
    assert_bass_body("bs-dist-square", &report);
}
