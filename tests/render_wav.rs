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

#[test]
fn supersaw_factory_is_audible_and_differs_from_sine() {
    let saw = load_factory("supersaw-bass").expect("supersaw-bass");
    let mut sine = saw.clone();
    for op in &mut sine.operators {
        op.waveform = fm_synth::Waveform::Sine;
    }
    let params = RenderParams {
        frequency_hz: 82.41,
        duration_secs: 0.3,
        velocity: 0.95,
        sample_rate: 22_050,
    };
    let xa = render(&saw, &params).unwrap();
    let xb = render(&sine, &params).unwrap();
    assert!(xa.iter().all(|s| s.is_finite()));
    assert!(rms(&xa) > 0.02, "super-saw rms {}", rms(&xa));
    assert!(peak(&xa) > 0.4, "super-saw peak {}", peak(&xa));
    let diff: f32 = xa.iter().zip(&xb).map(|(l, r)| (l - r).abs()).sum();
    assert!(
        diff > 2.0,
        "factory super-saw nearly identical to sine (diff={diff})"
    );
}

fn brightness(buf: &[f32]) -> f32 {
    let mut s = 0.0f32;
    for w in buf.windows(2) {
        let d = w[1] - w[0];
        s += d * d;
    }
    (s / buf.len() as f32).sqrt()
}

#[test]
fn factory_filter_pluck_and_modes_make_sound() {
    for id in ["filter-pluck", "bp-growl", "hp-air"] {
        let preset = load_factory(id).unwrap();
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: 110.0,
                duration_secs: 0.25,
                velocity: 0.9,
                sample_rate: 22_050,
            },
        )
        .expect(id);
        assert!(rms(&buf) > 0.01, "{id} rms {}", rms(&buf));
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN");
    }
}

#[test]
fn lowpass_low_cutoff_is_darker_than_open() {
    let mut closed = load_factory("metallic-hit").unwrap();
    let mut open = closed.clone();
    closed.filter.kind = fm_synth::FilterType::Lowpass;
    closed.filter.cutoff = 200.0;
    closed.filter.resonance = 0.15;
    closed.filter.env_amount = 0.0;
    open.filter.kind = fm_synth::FilterType::Lowpass;
    open.filter.cutoff = 16_000.0;
    open.filter.resonance = 0.15;
    open.filter.env_amount = 0.0;
    let params = RenderParams {
        frequency_hz: 196.0,
        duration_secs: 0.3,
        velocity: 0.85,
        sample_rate: 44_100,
    };
    let dark = render(&closed, &params).unwrap();
    let bright = render(&open, &params).unwrap();
    let b_dark = brightness(&dark);
    let b_bright = brightness(&bright);
    assert!(
        b_dark < b_bright * 0.6,
        "low cutoff should attenuate highs (closed={b_dark}, open={b_bright})"
    );
}
