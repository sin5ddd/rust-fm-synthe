//! Integration: engine is audible, WAV headers match, factory preset smoke.

use fm_synth::{
    load_factory, pcm_data_bytes, peak, render, rms, write_wav, RenderParams, WavSettings,
};
use hound::WavReader;
use std::fs;

fn scratch_wav(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("fm_synth_tests");
    fs::create_dir_all(&dir).unwrap();
    dir.join(name)
}

#[test]
fn engine_renders_non_silent_buffer() {
    let preset = load_factory("growl-bass").expect("factory growl-bass");
    let buf = render(
        &preset,
        &RenderParams {
            frequency_hz: 82.41,
            duration_secs: 0.35,
            velocity: 0.95,
            sample_rate: 44_100,
        },
    )
    .unwrap();

    assert!(!buf.is_empty());
    assert!(buf.iter().all(|s| s.is_finite()), "NaN/Inf in buffer");
    assert!(
        buf.iter().any(|&s| s.abs() > 1e-3),
        "buffer is effectively silent"
    );
    assert!(peak(&buf) > 0.4, "peak {} too low", peak(&buf));
    assert!(rms(&buf) > 0.03, "rms {} too low", rms(&buf));
}

#[test]
fn wav_header_and_size_sanity() {
    let preset = load_factory("stab-pluck").unwrap();
    let sr = 48_000u32;
    let duration = 0.25f64;
    let bit_depth = 24u16;
    let buf = render(
        &preset,
        &RenderParams {
            frequency_hz: 196.0,
            duration_secs: duration,
            velocity: 0.8,
            sample_rate: sr,
        },
    )
    .unwrap();

    let path = scratch_wav("header-sanity.wav");
    write_wav(&path, &buf, WavSettings::new(sr, bit_depth).unwrap()).unwrap();

    let mut reader = WavReader::open(&path).unwrap();
    let spec = reader.spec();
    assert_eq!(spec.channels, 1);
    assert_eq!(spec.sample_rate, sr);
    assert_eq!(spec.bits_per_sample, bit_depth);
    assert_eq!(spec.sample_format, hound::SampleFormat::Int);

    let n = reader.samples::<i32>().count();
    let expected = (duration * f64::from(sr)).round() as usize;
    assert_eq!(n, expected, "sample count");
    assert_eq!(n, buf.len());

    let meta = fs::metadata(&path).unwrap();
    let data_bytes = pcm_data_bytes(n, bit_depth, 1);
    // RIFF/WAVE headers are typically 44 bytes; 24-bit may be 44+.
    assert!(
        meta.len() as usize >= data_bytes + 36,
        "file {} smaller than PCM+header ({data_bytes}+36)",
        meta.len()
    );
    assert!(
        meta.len() as usize <= data_bytes + 128,
        "file {} much larger than PCM ({data_bytes})",
        meta.len()
    );
}

#[test]
fn preset_render_smoke_writes_audible_wav() {
    let preset = load_factory("sub-bass").unwrap();
    let sr = 44_100u32;
    let buf = render(
        &preset,
        &RenderParams {
            frequency_hz: 55.0,
            duration_secs: 0.5,
            velocity: 1.0,
            sample_rate: sr,
        },
    )
    .unwrap();

    let path = scratch_wav("sub-bass-smoke.wav");
    write_wav(&path, &buf, WavSettings::new(sr, 16).unwrap()).unwrap();
    assert!(path.is_file());

    let mut reader = WavReader::open(&path).unwrap();
    let decoded: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
    assert_eq!(decoded.len(), buf.len());
    let abs_max = decoded.iter().map(|s| s.unsigned_abs()).max().unwrap();
    assert!(
        abs_max > 1000,
        "decoded WAV peak {abs_max} looks silent / empty"
    );
    assert!(decoded.iter().any(|&s| s != 0));
}

#[test]
fn every_factory_preset_makes_sound() {
    for id in fm_synth::factory_ids() {
        let preset = load_factory(id).unwrap();
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: 110.0,
                duration_secs: 0.2,
                velocity: 0.9,
                sample_rate: 22_050,
            },
        )
        .expect(id);
        assert!(
            rms(&buf) > 0.01,
            "preset `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
    }
}
