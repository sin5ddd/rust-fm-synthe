//! Spectrogram / metrics: sine, noise, pitch drop, factory kick smoke.

use fm_synth::{
    analyze_buffer, load_factory, midi_to_hz, render, write_analysis_bundle, write_wav,
    AnalyzeOpts, RenderParams, WavSettings, SPECTROGRAM_HEIGHT, SPECTROGRAM_WIDTH,
};
use std::fs;
use std::path::PathBuf;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join("fm_synth_analyze_tests");
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

fn sine(freq: f32, sr: u32, secs: f64, amp: f32) -> Vec<f32> {
    let n = (secs * f64::from(sr)).round() as usize;
    (0..n)
        .map(|i| {
            let t = i as f32 / sr as f32;
            (std::f32::consts::TAU * freq * t).sin() * amp
        })
        .collect()
}

fn chirp_down(start_hz: f32, end_hz: f32, sr: u32, secs: f64) -> Vec<f32> {
    let n = (secs * f64::from(sr)).round() as usize;
    let mut phase = 0.0f32;
    let dt = 1.0 / sr as f32;
    (0..n)
        .map(|i| {
            let t = i as f32 / (n.saturating_sub(1).max(1) as f32);
            let hz = start_hz * (end_hz / start_hz).powf(t);
            phase += std::f32::consts::TAU * hz * dt;
            phase.sin() * 0.5
        })
        .collect()
}

fn white_noise(n: usize, seed: u64) -> Vec<f32> {
    let mut s = seed | 1;
    (0..n)
        .map(|_| {
            s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
            let u = ((s >> 33) as f32) / (u32::MAX as f32);
            u * 2.0 - 1.0
        })
        .collect()
}

#[test]
fn sine_440_centroid_and_low_flatness() {
    let sr = 22_050u32;
    let buf = sine(440.0, sr, 0.5, 0.5);
    let a = analyze_buffer(&buf, sr, &AnalyzeOpts::default()).unwrap();
    assert!(
        (a.report.spectral_centroid_hz - 440.0).abs() < 30.0,
        "centroid {}",
        a.report.spectral_centroid_hz
    );
    assert!(
        a.report.spectral_flatness < 0.2,
        "flatness {}",
        a.report.spectral_flatness
    );
    assert!(a.report.band_energy.mid_250_2000 > a.report.band_energy.sub_20_80);
}

#[test]
fn noise_is_flatter_than_sine() {
    let sr = 22_050u32;
    let n = (0.4 * f64::from(sr)).round() as usize;
    let noise = white_noise(n, 0xC0FFEE);
    let tone = sine(440.0, sr, 0.4, 0.5);
    let a_n = analyze_buffer(&noise, sr, &AnalyzeOpts::default()).unwrap();
    let a_s = analyze_buffer(&tone, sr, &AnalyzeOpts::default()).unwrap();
    assert!(
        a_n.report.spectral_flatness > a_s.report.spectral_flatness * 2.0,
        "noise flatness {} vs sine {}",
        a_n.report.spectral_flatness,
        a_s.report.spectral_flatness
    );
}

#[test]
fn chirp_pitch_drops() {
    let sr = 22_050u32;
    let buf = chirp_down(220.0, 55.0, sr, 0.8);
    let a = analyze_buffer(&buf, sr, &AnalyzeOpts::default()).unwrap();
    let pitch = a
        .report
        .pitch
        .as_ref()
        .expect("expected voiced pitch track on exponential chirp");
    assert!(
        pitch.start_hz > pitch.end_hz * 1.5,
        "start {} end {}",
        pitch.start_hz,
        pitch.end_hz
    );
    assert!(
        pitch.drop_semitones > 6.0,
        "drop {} st",
        pitch.drop_semitones
    );
}

#[test]
fn empty_and_short_buffers_error() {
    let err = analyze_buffer(&[], 44_100, &AnalyzeOpts::default()).unwrap_err();
    assert!(err.to_string().contains("256"));
    let err = analyze_buffer(&[0.1; 100], 44_100, &AnalyzeOpts::default()).unwrap_err();
    assert!(err.to_string().contains("256"));
}

#[test]
fn png_and_json_roundtrip_from_sine() {
    let sr = 22_050u32;
    let buf = sine(440.0, sr, 0.35, 0.6);
    let a = analyze_buffer(
        &buf,
        sr,
        &AnalyzeOpts {
            preset_id: Some("sine-440".into()),
            intent: Some("pure 440 Hz tone".into()),
            ..AnalyzeOpts::default()
        },
    )
    .unwrap();
    let png = scratch("sine-440.png");
    let json = scratch("sine-440.json");
    write_analysis_bundle(&a, &png, &json).unwrap();
    assert!(png.is_file());
    assert!(json.is_file());

    let file = fs::File::open(&png).unwrap();
    let decoder = png::Decoder::new(file);
    let reader = decoder.read_info().unwrap();
    let info = reader.info();
    assert_eq!(info.width, SPECTROGRAM_WIDTH);
    assert_eq!(info.height, SPECTROGRAM_HEIGHT);
    assert_eq!(info.color_type, png::ColorType::Rgb);

    let text = fs::read_to_string(&json).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert!(v["spectral_centroid_hz"].as_f64().unwrap() > 400.0);
    assert_eq!(v["preset_id"].as_str().unwrap(), "sine-440");
    assert_eq!(v["intent"].as_str().unwrap(), "pure 440 Hz tone");
    assert!(v["images"]["spectrogram"]
        .as_str()
        .unwrap()
        .ends_with("sine-440.png"));
}

#[test]
fn wav_read_matches_written_pcm_length() {
    let sr = 22_050u32;
    let buf = sine(220.0, sr, 0.25, 0.4);
    let path = scratch("roundtrip.wav");
    write_wav(&path, &buf, WavSettings::new(sr, 16).unwrap()).unwrap();
    let data = fm_synth::read_wav(&path).unwrap();
    assert_eq!(data.sample_rate, sr);
    assert_eq!(data.samples.len(), buf.len());
    assert!(data.samples.iter().any(|s| s.abs() > 0.05));
}

#[test]
fn bd_808_boom_has_sub_and_pitch_drop() {
    let preset = load_factory("bd-808-boom").unwrap();
    let sr = 22_050u32;
    let buf = render(
        &preset,
        &RenderParams {
            frequency_hz: midi_to_hz(preset.default_note),
            duration_secs: preset.default_duration,
            velocity: 0.9,
            sample_rate: sr,
        },
    )
    .unwrap();
    let a = analyze_buffer(
        &buf,
        sr,
        &AnalyzeOpts {
            preset_id: Some("bd-808-boom".into()),
            description: Some(preset.description.clone()),
            ..AnalyzeOpts::default()
        },
    )
    .unwrap();
    let b = &a.report.band_energy;
    assert!(
        b.sub_20_80 + b.bass_80_250 > b.mid_250_2000,
        "808 boom should be low (sub={}, bass={}, mid={})",
        b.sub_20_80,
        b.bass_80_250,
        b.mid_250_2000
    );
    if let Some(pitch) = &a.report.pitch {
        assert!(
            pitch.start_hz > pitch.end_hz,
            "808 boom pitch should drop ({} -> {})",
            pitch.start_hz,
            pitch.end_hz
        );
    } else {
        panic!("bd-808-boom should yield a pitch track (start_semitones=24)");
    }
}
