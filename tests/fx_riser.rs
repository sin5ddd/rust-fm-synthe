//! Factory FX risers must climb, not read as a downward 808.
//!
//! Pitch-focused shots need a measured rise. Noise / filter shots need the
//! late window brighter than the start even if pitch is unvoiced.

use fm_synth::{
    analyze_buffer, analyze_preset, load_factory, AnalyzeOpts, AnalysisReport, ExportParams,
};

const SR: u32 = 44_100;
const INTENT: &str = "riser: pitch and/or spectrum rises over 15s";

const PITCH_RISERS: [&str; 4] = ["fx-riser-pitch", "fx-riser-saw", "fx-uplifter", "fm-riser"];
const SPECTRUM_RISERS: [&str; 2] = ["fx-riser-noise", "fx-riser-filter"];

fn analyze_id(id: &str) -> (Vec<f32>, AnalysisReport) {
    let preset = load_factory(id).expect(id);
    let export = ExportParams {
        sample_rate: SR,
        velocity: 0.9,
        ..ExportParams::default()
    };
    let (buf, analysis) = analyze_preset(id, &preset, &export, Some(INTENT)).expect(id);
    (buf, analysis.report)
}

fn window_report(buf: &[f32], t0: f64, t1: f64) -> AnalysisReport {
    let a = ((t0 * f64::from(SR)) as usize).min(buf.len());
    let b = ((t1 * f64::from(SR)) as usize).min(buf.len());
    assert!(b > a + 256, "window {t0}..{t1} too short");
    analyze_buffer(&buf[a..b], SR, &AnalyzeOpts::default())
        .unwrap()
        .report
}

#[test]
fn pitch_risers_measure_an_upward_track() {
    for id in PITCH_RISERS {
        let (_buf, report) = analyze_id(id);
        let pitch = report
            .pitch
            .as_ref()
            .unwrap_or_else(|| panic!("{id} should yield a pitch track"));
        assert!(
            pitch.end_hz > pitch.start_hz * 1.25,
            "{id} pitch should rise ({} -> {} Hz, drop {} st)",
            pitch.start_hz,
            pitch.end_hz,
            pitch.drop_semitones
        );
        assert!(
            pitch.drop_semitones < -3.0,
            "{id} drop_semitones {} should be negative (a rise)",
            pitch.drop_semitones
        );
    }
}

#[test]
fn noise_and_filter_risers_brighten() {
    for id in SPECTRUM_RISERS {
        let (buf, _report) = analyze_id(id);
        let early = window_report(&buf, 0.4, 2.6);
        let late = window_report(&buf, 10.5, 13.8);
        assert!(
            late.spectral_centroid_hz > early.spectral_centroid_hz * 1.25,
            "{id} late centroid {} should exceed early {}",
            late.spectral_centroid_hz,
            early.spectral_centroid_hz
        );
        assert!(
            late.band_energy.high_2000_plus + late.band_energy.mid_250_2000
                > early.band_energy.high_2000_plus + early.band_energy.mid_250_2000,
            "{id} late mid/high energy should exceed the start (early mid+high={}, late={})",
            early.band_energy.mid_250_2000 + early.band_energy.high_2000_plus,
            late.band_energy.mid_250_2000 + late.band_energy.high_2000_plus
        );
    }
}

#[test]
fn eight_bar_fx_risers_keep_sub_and_100hz_sine() {
    for id in PITCH_RISERS.iter().chain(SPECTRUM_RISERS.iter()) {
        let preset = load_factory(id).expect(id);
        assert!(
            (preset.operators[0].ratio - 0.25).abs() < 1e-9,
            "{id} OP1 sub ratio"
        );
        let start_hz = fm_synth::midi_to_hz(preset.default_note)
            * preset.operators[1].ratio
            * fm_synth::semitones_to_ratio(preset.pitch.start_semitones);
        assert!(
            (start_hz - 100.0).abs() < 0.5,
            "{id} OP2 must start near 100 Hz, got {start_hz}"
        );
        assert!(preset.pitch.end_semitones > preset.pitch.start_semitones);
    }
}
