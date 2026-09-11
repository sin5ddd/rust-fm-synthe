//! Factory drone classes retuned against external reference *metrics*
//! (centroid / band energy / hold length). Spectral checks use a short
//! window — drones are stationary beds, not one-shots.

use fm_synth::{analyze_preset, load_factory, AnalysisReport, ExportParams};

const SR: u32 = 48_000;
const ANALYZE_SECS: f64 = 5.0;

const SUB_BEDS: [&str; 4] = ["dr-sine-sub", "dr-void", "dr-abyss", "dr-sub-octave"];
const WARM_PADS: [&str; 5] = [
    "dr-pad-dark",
    "dr-ambient-dark",
    "dr-choir-dark",
    "dr-choir-low",
    "dr-fog",
];
const BRIGHT_PADS: [&str; 4] = [
    "dr-score-hold",
    "dr-ghost-choir",
    "dr-cathedral",
    "dr-ice-cave",
];
const SPACEY: [&str; 3] = ["dr-scifi-hum", "dr-reactor", "dr-horror"];
const AIRY: [&str; 1] = ["dr-engine"];

fn analyze_id(id: &str) -> (fm_synth::Preset, AnalysisReport) {
    let preset = load_factory(id).expect(id);
    let export = ExportParams {
        sample_rate: SR,
        velocity: 0.9,
        duration: Some(ANALYZE_SECS),
        ..ExportParams::default()
    };
    let intent = preset.description.clone();
    let (_buf, analysis) = analyze_preset(id, &preset, &export, Some(intent.as_str())).expect(id);
    (preset, analysis.report)
}

#[test]
fn sub_beds_are_long_pure_sub() {
    for id in SUB_BEDS {
        let (preset, report) = analyze_id(id);
        assert!(
            (39.0..=42.0).contains(&preset.default_duration),
            "{id} default_duration {} should be a ~41 s sub bed",
            preset.default_duration
        );
        assert!(
            (28.0..=70.0).contains(&report.spectral_centroid_hz),
            "{id} centroid {} Hz (want ~42 Hz sub bed)",
            report.spectral_centroid_hz
        );
        assert!(
            report.band_energy.sub_20_80 > 0.85,
            "{id} sub_20_80 {} too thin",
            report.band_energy.sub_20_80
        );
        assert!(
            report.band_energy.mid_250_2000 + report.band_energy.high_2000_plus < 0.08,
            "{id} mid+high {} leaked into a sub bed",
            report.band_energy.mid_250_2000 + report.band_energy.high_2000_plus
        );
    }
}

#[test]
fn warm_pad_drones_sit_in_low_mid() {
    for id in WARM_PADS {
        let (preset, report) = analyze_id(id);
        assert!(
            (19.5..=21.0).contains(&preset.default_duration),
            "{id} default_duration {} should be a ~20 s pad drone",
            preset.default_duration
        );
        assert!(
            (180.0..=480.0).contains(&report.spectral_centroid_hz),
            "{id} centroid {} Hz (want ~280 Hz warm pad)",
            report.spectral_centroid_hz
        );
        assert!(
            report.band_energy.mid_250_2000 > 0.70,
            "{id} mid {} too thin for a warm pad drone",
            report.band_energy.mid_250_2000
        );
        assert!(
            report.band_energy.sub_20_80 < 0.08,
            "{id} sub {} should be near zero on a pad drone",
            report.band_energy.sub_20_80
        );
    }
}

#[test]
fn bright_pad_drones_are_airy_cinematic() {
    for id in BRIGHT_PADS {
        let (preset, report) = analyze_id(id);
        assert!(
            (19.5..=21.0).contains(&preset.default_duration),
            "{id} default_duration {} should be a ~20 s pad drone",
            preset.default_duration
        );
        assert!(
            (700.0..=1700.0).contains(&report.spectral_centroid_hz),
            "{id} centroid {} Hz (want ~1073 Hz bright pad)",
            report.spectral_centroid_hz
        );
        assert!(
            report.band_energy.mid_250_2000 > 0.55,
            "{id} mid {} too thin for a bright pad",
            report.band_energy.mid_250_2000
        );
        assert!(
            report.band_energy.high_2000_plus > 0.03,
            "{id} high {} missing air",
            report.band_energy.high_2000_plus
        );
        assert!(
            report.band_energy.sub_20_80 < 0.08,
            "{id} sub {} should be near zero",
            report.band_energy.sub_20_80
        );
    }
}

#[test]
fn spacey_drones_are_dark_mid() {
    for id in SPACEY {
        let (preset, report) = analyze_id(id);
        assert!(
            preset.default_duration + 1e-9 >= 16.2,
            "{id} default_duration {} too short",
            preset.default_duration
        );
        assert!(
            (380.0..=850.0).contains(&report.spectral_centroid_hz),
            "{id} centroid {} Hz (want ~573 Hz space drone)",
            report.spectral_centroid_hz
        );
        assert!(
            report.band_energy.mid_250_2000 > 0.80,
            "{id} mid {} too thin for a dark space drone",
            report.band_energy.mid_250_2000
        );
        assert!(
            report.band_energy.sub_20_80 < 0.08,
            "{id} sub {} should be near zero",
            report.band_energy.sub_20_80
        );
    }
}

#[test]
fn airy_mid_drone_has_high_shelf() {
    for id in AIRY {
        let (preset, report) = analyze_id(id);
        assert!(
            preset.default_duration + 1e-9 >= 16.2,
            "{id} default_duration {} too short",
            preset.default_duration
        );
        assert!(
            (1200.0..=2400.0).contains(&report.spectral_centroid_hz),
            "{id} centroid {} Hz (want ~1735 Hz airy mid)",
            report.spectral_centroid_hz
        );
        assert!(
            report.band_energy.mid_250_2000 > 0.60,
            "{id} mid {} too thin",
            report.band_energy.mid_250_2000
        );
        assert!(
            report.band_energy.high_2000_plus > 0.08,
            "{id} high {} missing air",
            report.band_energy.high_2000_plus
        );
        assert!(
            report.band_energy.sub_20_80 < 0.08,
            "{id} sub {} should be near zero",
            report.band_energy.sub_20_80
        );
    }
}
