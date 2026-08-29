//! Integration: engine is audible, WAV headers match, factory preset smoke.

use fm_synth::{
    factory_ids, load_factory, midi_to_hz, pcm_data_bytes, peak, render, render_all_factory, rms,
    write_wav, ExportParams, RenderParams, WavSettings, DEFAULT_OUTPUT_DIR,
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
fn factory_bd_kicks_are_twenty_and_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("bd-"))
        .collect();
    assert_eq!(
        ids.len(),
        20,
        "expected exactly 20 bd-* factory kicks, got {}: {ids:?}",
        ids.len()
    );

    for id in ids {
        let preset = load_factory(id).unwrap();
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: midi_to_hz(preset.default_note),
                duration_secs: preset.default_duration,
                velocity: 0.9,
                sample_rate: 22_050,
            },
        )
        .expect(id);
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");
        assert!(
            rms(&buf) > 0.01,
            "bd kick `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(
            peak(&buf) > 0.4,
            "bd kick `{id}` peak {} too low",
            peak(&buf)
        );
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
    }
}

#[test]
fn factory_sd_snares_are_twenty_and_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("sd-"))
        .collect();
    assert_eq!(
        ids.len(),
        20,
        "expected exactly 20 sd-* factory snares, got {}: {ids:?}",
        ids.len()
    );

    for id in ids {
        let preset = load_factory(id).unwrap();
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: midi_to_hz(preset.default_note),
                duration_secs: preset.default_duration,
                velocity: 0.9,
                sample_rate: 22_050,
            },
        )
        .expect(id);
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");
        assert!(
            rms(&buf) > 0.01,
            "sd snare `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(
            peak(&buf) > 0.4,
            "sd snare `{id}` peak {} too low",
            peak(&buf)
        );
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
    }
}

#[test]
fn factory_ld_leads_are_fifty_and_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("ld-"))
        .collect();
    assert_eq!(
        ids.len(),
        50,
        "expected exactly 50 ld-* factory leads, got {}: {ids:?}",
        ids.len()
    );

    for id in ids {
        let preset = load_factory(id).unwrap();
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: midi_to_hz(preset.default_note),
                duration_secs: preset.default_duration,
                velocity: 0.9,
                sample_rate: 22_050,
            },
        )
        .expect(id);
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");
        assert!(
            rms(&buf) > 0.01,
            "ld lead `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
    }
}

#[test]
fn factory_fx_are_fifty_and_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("fx-"))
        .collect();
    assert_eq!(
        ids.len(),
        50,
        "expected exactly 50 fx-* factory FX, got {}: {ids:?}",
        ids.len()
    );

    for id in ids {
        let preset = load_factory(id).unwrap();
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: midi_to_hz(preset.default_note),
                duration_secs: preset.default_duration,
                velocity: 0.9,
                sample_rate: 22_050,
            },
        )
        .expect(id);
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");
        assert!(
            rms(&buf) > 0.01,
            "fx `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
    }
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

fn scratch_dir(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join("fm_synth_tests").join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn wav_is_audible(path: &std::path::Path) {
    assert!(path.is_file(), "missing {}", path.display());
    let mut reader = WavReader::open(path).unwrap();
    let decoded: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
    assert!(!decoded.is_empty(), "{} empty", path.display());
    let abs_max = decoded.iter().map(|s| s.unsigned_abs()).max().unwrap();
    assert!(
        abs_max > 1000,
        "{} decoded peak {abs_max} looks silent / empty",
        path.display()
    );
    assert!(decoded.iter().any(|&s| s != 0));
}

#[test]
fn render_all_factory_writes_one_wav_per_preset() {
    assert_eq!(DEFAULT_OUTPUT_DIR, "dist");
    // Temp dir stands in for dist/ so tests never write the repo dest.
    let dir = scratch_dir("render-all-factory");
    let ids = factory_ids();
    assert!(!ids.is_empty());

    let batch = render_all_factory(
        &dir,
        &ExportParams {
            note: None,
            hz: None,
            duration: Some(0.12),
            velocity: 0.9,
            sample_rate: 22_050,
            bit_depth: 16,
        },
    )
    .unwrap();

    assert!(
        batch.failures.is_empty(),
        "factory render-all failures: {:?}",
        batch.failures
    );
    assert_eq!(
        batch.written.len(),
        ids.len(),
        "expected one WAV per factory id"
    );

    for id in &ids {
        let path = dir.join(format!("{id}.wav"));
        let report = batch
            .written
            .iter()
            .find(|r| r.preset_id == *id)
            .unwrap_or_else(|| panic!("no report for {id}"));
        assert_eq!(report.path, path);
        wav_is_audible(&path);
    }

    let wavs: Vec<_> = fs::read_dir(&dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("wav"))
        .collect();
    assert_eq!(wavs.len(), ids.len());
}

#[test]
fn render_all_factory_applies_shared_overrides() {
    let dir = scratch_dir("render-all-overrides");
    let batch = render_all_factory(
        &dir,
        &ExportParams {
            note: Some(60),
            hz: None,
            duration: Some(0.08),
            velocity: 0.8,
            sample_rate: 22_050,
            bit_depth: 16,
        },
    )
    .unwrap()
    .into_result()
    .unwrap();

    assert_eq!(batch.len(), factory_ids().len());
    for report in &batch {
        assert!(
            (report.duration_secs - 0.08).abs() < 1e-9,
            "{}",
            report.preset_id
        );
        assert_eq!(report.sample_rate, 22_050);
        // MIDI 60 ≈ 261.63 Hz
        assert!(
            (report.frequency_hz - 261.625565).abs() < 0.01,
            "{} hz {}",
            report.preset_id,
            report.frequency_hz
        );
        wav_is_audible(&report.path);
    }
}

#[test]
fn render_all_names_failed_presets() {
    let dir = scratch_dir("render-all-fail");
    let batch = render_all_factory(
        &dir,
        &ExportParams {
            bit_depth: 8,
            duration: Some(0.05),
            sample_rate: 22_050,
            ..ExportParams::default()
        },
    )
    .unwrap();
    assert!(batch.written.is_empty());
    assert_eq!(batch.failures.len(), factory_ids().len());
    let err = batch.into_result().unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains("sub-bass"), "{msg}");
    assert!(msg.contains("supersaw-bass"), "{msg}");
}

const STRUDEL_ONESHOT_IDS: [&str; 4] = ["cp-house", "lead-fm-pluck", "stab-fm-fifth", "reese-mid"];

#[test]
fn strudel_oneshot_ids_parse() {
    for id in STRUDEL_ONESHOT_IDS {
        load_factory(id).expect(id);
        assert!(
            factory_ids().iter().any(|fid| *fid == id),
            "{id} missing from factory table"
        );
    }
}

#[test]
fn strudel_oneshots_render_nonsilent_48k_16bit() {
    for id in STRUDEL_ONESHOT_IDS {
        let preset = load_factory(id).expect(id);
        let sr = 48_000u32;
        let bit_depth = 16u16;
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: midi_to_hz(preset.default_note),
                duration_secs: preset.default_duration,
                velocity: 0.9,
                sample_rate: sr,
            },
        )
        .expect(id);
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");
        assert!(
            rms(&buf) > 0.01,
            "{id} rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(peak(&buf) > 0.4, "{id} peak {} too low", peak(&buf));

        let path = scratch_wav(&format!("{id}-48k16.wav"));
        write_wav(&path, &buf, WavSettings::new(sr, bit_depth).unwrap()).unwrap();

        let mut reader = WavReader::open(&path).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 1, "{id}");
        assert_eq!(spec.sample_rate, sr, "{id}");
        assert_eq!(spec.bits_per_sample, bit_depth, "{id}");
        assert_eq!(spec.sample_format, hound::SampleFormat::Int, "{id}");

        let decoded: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();
        assert_eq!(decoded.len(), buf.len(), "{id}");
        let abs_max = decoded.iter().map(|s| s.unsigned_abs()).max().unwrap();
        assert!(
            abs_max > 1000,
            "{id} decoded peak {abs_max} looks silent / empty"
        );
        assert!(decoded.iter().any(|&s| s != 0), "{id} all-zero PCM");
    }
}

fn goertzel_power(buf: &[f32], sr: f32, freq: f32) -> f64 {
    let n = buf.len();
    if n == 0 {
        return 0.0;
    }
    let k = (n as f64 * f64::from(freq) / f64::from(sr)).round();
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

#[test]
fn stab_fm_fifth_is_hollow_c_and_g_only() {
    let preset = load_factory("stab-fm-fifth").unwrap();
    assert_eq!(preset.default_note, 48);
    let sr = 48_000u32;
    let f0 = midi_to_hz(48) as f32;
    let buf = render(
        &preset,
        &RenderParams {
            frequency_hz: f64::from(f0),
            duration_secs: preset.default_duration,
            velocity: 0.9,
            sample_rate: sr,
        },
    )
    .unwrap();

    // Skip the attack transient (broadband click) and Hann-window the decay
    // so a 5:4 bin is not filled by envelope smear.
    let start = (sr as usize) / 25;
    let end = ((sr as usize) * 3 / 20).min(buf.len());
    assert!(end > start + 64, "not enough decay body to measure");
    let body = hann_window(&buf[start..end]);
    let c = goertzel_power(&body, sr as f32, f0);
    let g = goertzel_power(&body, sr as f32, f0 * 1.5);
    let e = goertzel_power(&body, sr as f32, f0 * 1.25);
    let e4 = goertzel_power(&body, sr as f32, f0 * 2.5);
    let fifth_h = goertzel_power(&body, sr as f32, f0 * 5.0);

    assert!(c > 0.0 && g > 0.0, "missing C or G (c={c}, g={g})");
    let cg = c.min(g);
    assert!(
        e < cg * 0.08,
        "major third (5:4) leaked through (e={e}, cg={cg})"
    );
    assert!(
        e4 < cg * 0.08,
        "E an octave up (2.5×) leaked (e4={e4}, cg={cg})"
    );
    assert!(
        fifth_h < cg * 0.08,
        "5th harmonic (E) leaked (h5={fifth_h}, cg={cg})"
    );
}

#[test]
fn lead_fm_pluck_fundamental_is_c3_not_c4() {
    let preset = load_factory("lead-fm-pluck").unwrap();
    assert_eq!(preset.default_note, 48, "MIDI 48 = C3 ≈ 130.8 Hz, not C4");
    let sr = 48_000u32;
    let f0 = midi_to_hz(48) as f32;
    assert!((f0 - 130.81).abs() < 0.05);
    let buf = render(
        &preset,
        &RenderParams {
            frequency_hz: f64::from(f0),
            duration_secs: preset.default_duration,
            velocity: 0.9,
            sample_rate: sr,
        },
    )
    .unwrap();

    let start = (sr as usize) / 20;
    let end = ((sr as usize) / 5).min(buf.len());
    let body = hann_window(&buf[start..end]);
    let c3 = goertzel_power(&body, sr as f32, f0);
    let c4 = goertzel_power(&body, sr as f32, f0 * 2.0);
    assert!(
        c3 > c4 * 2.0,
        "lead-fm-pluck fundamental should be C3 not C4 (c3={c3}, c4={c4})"
    );
}

#[test]
fn cp_house_has_1khz_body_without_sub() {
    let preset = load_factory("cp-house").unwrap();
    let sr = 48_000u32;
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
    let body = hann_window(&buf);
    let sr_f = sr as f32;
    let band = |lo, hi| {
        let mut e = 0.0;
        let mut f = lo;
        while f < hi {
            e += goertzel_power(&body, sr_f, f);
            f += 40.0;
        }
        e
    };
    let sub = band(20.0, 200.0);
    let mid = band(800.0, 1400.0);
    let air = band(2000.0, 4500.0);
    assert!(
        mid > sub * 8.0,
        "cp-house should have 1 kHz body, not sub (mid={mid}, sub={sub})"
    );
    assert!(
        mid > 0.0 && air > 0.0,
        "cp-house needs both 1 kHz body and high slap (mid={mid}, air={air})"
    );
    assert!(
        air > mid * 0.15,
        "cp-house high slap disappeared (mid={mid}, air={air})"
    );
}
