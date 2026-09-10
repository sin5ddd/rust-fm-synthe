//! Integration: engine is audible, WAV headers match, factory preset smoke.

use fm_synth::{
    factory_ids, load_factory, midi_to_hz, pcm_data_bytes, peak, render, render_all_factory, rms,
    write_wav, ExportParams, Preset, RenderParams, WavSettings, DEFAULT_OUTPUT_DIR,
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

/// Factory ids whose TOML lives in `presets/ld/` (the `ld-*` bank plus
/// the older lead/stab/pluck shots that were not renamed).
fn factory_ld_folder_ids() -> Vec<&'static str> {
    factory_ids()
        .into_iter()
        .filter(|id| {
            id.starts_with("ld-")
                || matches!(
                    *id,
                    "lead-fm-pluck"
                        | "stab-fm-fifth"
                        | "stab-fm-major"
                        | "filter-pluck"
                        | "stab-pluck"
                )
        })
        .collect()
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
        // Bank-count / audible smoke. Hold length is checked in
        // `factory_ld_folder_holds_three_wholes_at_130bpm` (default ~5.54 s).
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: midi_to_hz(preset.default_note),
                duration_secs: 0.25,
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

/// 130 BPM, 4/4 → 1 beat = 60/130 s. 3 whole notes = 12 beats = 720/130 ≈ 5.538 s.
/// Short `default_duration` is not enough if amp ADSR dies in 0.4 s:
/// the last 0.4 s of the default render must still have energy.
#[test]
fn factory_ld_folder_holds_three_wholes_at_130bpm() {
    let ids = factory_ld_folder_ids();
    assert!(
        ids.len() >= 55,
        "expected ld-* bank plus presets/ld extras, got {}: {ids:?}",
        ids.len()
    );

    const SR: u32 = 22_050;
    const WHOLE_NOTES_3_AT_130: f64 = 12.0 * 60.0 / 130.0;
    const TAIL_SECS: f64 = 0.4;
    let tail_n = ((TAIL_SECS * f64::from(SR)).round() as usize).max(1);

    for id in ids {
        let preset = load_factory(id).unwrap();
        assert!(
            (preset.default_duration - WHOLE_NOTES_3_AT_130).abs() < 1e-6,
            "{id} default_duration {} must be 3 whole notes @ 130 BPM ({WHOLE_NOTES_3_AT_130})",
            preset.default_duration
        );

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
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");

        let expected = (preset.default_duration * f64::from(SR)).round() as usize;
        assert_eq!(
            buf.len(),
            expected,
            "{id} sample count {} != duration*sr {}",
            buf.len(),
            expected
        );
        assert!(buf.len() >= tail_n, "{id} buffer shorter than tail window");

        let tail = &buf[buf.len() - tail_n..];
        let tail_rms = rms(tail);
        assert!(
            tail_rms > 0.01,
            "{id} last {TAIL_SECS}s is silence (rms={tail_rms}); \
             carrier sustain must hold for the full 3-whole-note key-down"
        );

        // Tone still audible near t=5 s (well before the release tail).
        let t5 = ((5.0 * f64::from(SR)).round() as usize).min(buf.len().saturating_sub(tail_n));
        let at5 = rms(&buf[t5..t5 + tail_n]);
        assert!(
            at5 > 0.01,
            "{id} silent at t=5s (rms={at5}); filter/amp env died too early"
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

/// 130 BPM, 4/4 → 1 bar ≈ 1.846 s → 8 bars ≈ 14.769 s.
/// The default render must still have energy in the last 0.5 s.
#[test]
fn eight_bar_risers_hold_at_130bpm() {
    const IDS: [&str; 6] = [
        "fm-riser",
        "fx-riser-saw",
        "fx-riser-noise",
        "fx-riser-filter",
        "fx-riser-pitch",
        "fx-uplifter",
    ];
    const SR: u32 = 22_050;
    const TAIL_SECS: f64 = 0.5;
    const BARS_8_AT_130: f64 = 32.0 * 60.0 / 130.0;
    let tail_n = ((TAIL_SECS * f64::from(SR)).round() as usize).max(1);

    for id in IDS {
        let preset = load_factory(id).unwrap();
        assert!(
            preset.default_duration + 1e-9 >= BARS_8_AT_130,
            "{id} default_duration {} must cover 8 bars at 130 BPM ({BARS_8_AT_130})",
            preset.default_duration
        );

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
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");

        let expected = (preset.default_duration * f64::from(SR)).round() as usize;
        assert_eq!(
            buf.len(),
            expected,
            "{id} sample count {} != duration*sr {}",
            buf.len(),
            expected
        );
        assert!(buf.len() >= tail_n, "{id} buffer shorter than tail window");

        let tail = &buf[buf.len() - tail_n..];
        let tail_rms = rms(tail);
        assert!(
            tail_rms > 0.01,
            "{id} last {TAIL_SECS}s is silence (rms={tail_rms}); \
             carrier sustain must hold for the full 8 bars"
        );

        let t14 = ((14.0 * f64::from(SR)).round() as usize).min(buf.len().saturating_sub(tail_n));
        let at14 = rms(&buf[t14..t14 + tail_n]);
        assert!(
            at14 > 0.01,
            "{id} silent at t=14s (rms={at14}); amp env died before bar 8"
        );
    }
}

#[test]
fn factory_bs_basses_are_fifteen_and_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("bs-"))
        .collect();
    assert_eq!(
        ids.len(),
        15,
        "expected exactly 15 bs-* factory basses, got {}: {ids:?}",
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
            "bs bass `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
    }
}

#[test]
fn factory_pc_perc_are_fifty_and_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("pc-"))
        .collect();
    assert_eq!(
        ids.len(),
        50,
        "expected exactly 50 pc-* factory perc, got {}: {ids:?}",
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
            "pc perc `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
    }
}

#[test]
fn factory_dr_drones_are_fifty_and_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("dr-"))
        .collect();
    assert_eq!(
        ids.len(),
        50,
        "expected exactly 50 dr-* factory drones, got {}: {ids:?}",
        ids.len()
    );

    for id in ids {
        let preset = load_factory(id).unwrap();
        // Bank-count / audible smoke. Hold length is checked in
        // `factory_dr_drones_hold_eight_bars_at_120bpm` (default ~16 s).
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: midi_to_hz(preset.default_note),
                duration_secs: 0.25,
                velocity: 0.9,
                sample_rate: 22_050,
            },
        )
        .expect(id);
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");
        assert!(
            rms(&buf) > 0.01,
            "dr drone `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
    }
}

/// 120 BPM, 4/4 → 1 bar = 2 s → 8 bars = 16 s held note.
/// Short `default_duration` is not enough if amp ADSR dies in 0.4 s:
/// the last 1 s of the default render must still have energy, and
/// carriers must still be audible at t=14 s.
#[test]
fn factory_dr_drones_hold_eight_bars_at_120bpm() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("dr-"))
        .collect();
    assert_eq!(
        ids.len(),
        50,
        "expected exactly 50 dr-* factory drones, got {}: {ids:?}",
        ids.len()
    );

    const SR: u32 = 22_050;
    const TAIL_SECS: f64 = 1.0;
    let tail_n = ((TAIL_SECS * f64::from(SR)).round() as usize).max(1);

    for id in ids {
        let preset = load_factory(id).unwrap();
        assert!(
            (16.2..=18.0).contains(&preset.default_duration),
            "{id} default_duration {} must be ~16 s+ (8 bars @ 120 BPM)",
            preset.default_duration
        );

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
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");

        let expected = (preset.default_duration * f64::from(SR)).round() as usize;
        assert_eq!(
            buf.len(),
            expected,
            "{id} sample count {} != duration*sr {}",
            buf.len(),
            expected
        );
        assert!(buf.len() >= tail_n, "{id} buffer shorter than tail window");

        let tail = &buf[buf.len() - tail_n..];
        let tail_rms = rms(tail);
        assert!(
            tail_rms > 0.01,
            "{id} last {TAIL_SECS}s is silence (rms={tail_rms}); \
             carrier sustain must hold for the full 16 s key-down"
        );

        // Tone still audible at t=14 s (well before the release tail).
        let t14 = ((14.0 * f64::from(SR)).round() as usize).min(buf.len().saturating_sub(tail_n));
        let at14 = rms(&buf[t14..t14 + tail_n]);
        assert!(
            at14 > 0.01,
            "{id} silent at t=14s (rms={at14}); filter/amp env died too early"
        );
    }
}

#[test]
fn factory_pf_pads_are_thirty_and_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("pf-"))
        .collect();
    assert_eq!(
        ids.len(),
        30,
        "expected exactly 30 pf-* factory pads, got {}: {ids:?}",
        ids.len()
    );

    for id in ids {
        let preset = load_factory(id).unwrap();
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: midi_to_hz(preset.default_note),
                duration_secs: 0.25,
                velocity: 0.9,
                sample_rate: 22_050,
            },
        )
        .expect(id);
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");
        assert!(
            rms(&buf) > 0.01,
            "pf pad `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
    }
}

#[test]
fn factory_ps_pads_are_thirty_and_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("ps-"))
        .collect();
    assert_eq!(
        ids.len(),
        30,
        "expected exactly 30 ps-* factory pads, got {}: {ids:?}",
        ids.len()
    );

    for id in ids {
        let preset = load_factory(id).unwrap();
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: midi_to_hz(preset.default_note),
                duration_secs: 0.25,
                velocity: 0.9,
                sample_rate: 22_050,
            },
        )
        .expect(id);
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");
        assert!(
            rms(&buf) > 0.01,
            "ps pad `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
    }
}

/// Sparkle pads are sampled at C4. HP must not steal the root (or they read as G/E).
#[test]
fn factory_ps_pads_root_at_c4() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("ps-"))
        .collect();
    assert_eq!(ids.len(), 30);

    const SR: u32 = 22_050;
    let f0 = midi_to_hz(60) as f32;

    for id in ids {
        let preset = load_factory(id).unwrap();
        assert_eq!(
            preset.default_note, 60,
            "{id} default_note {} must be MIDI 60 (C4)",
            preset.default_note
        );
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: f64::from(f0),
                duration_secs: 1.5,
                velocity: 0.9,
                sample_rate: SR,
            },
        )
        .expect(id);
        let start = (SR as usize) / 2;
        let end = ((SR as usize) * 6 / 5).min(buf.len());
        assert!(end > start + 64, "{id} too short to measure root");
        let body = hann_window(&buf[start..end]);
        let sr_f = SR as f32;
        let c = goertzel_power(&body, sr_f, f0).max(goertzel_power(&body, sr_f, f0 * 2.0));
        let g = goertzel_power(&body, sr_f, f0 * 1.5).max(goertzel_power(&body, sr_f, f0 * 3.0));
        let e = goertzel_power(&body, sr_f, f0 * 1.25).max(goertzel_power(&body, sr_f, f0 * 2.5));
        assert!(
            c > g,
            "{id} G stronger than C (c={c}, g={g}); HP or 3× partial stole the root"
        );
        assert!(c > e, "{id} E stronger than C (c={c}, e={e})");
    }
}

/// 120 BPM, 4/4 → 1 bar = 2 s → 8 bars = 16 s held note.
/// Pads are long-shot like drones, but not the sub/rumble bed.
/// Last 1 s and t=14 s must still have energy.
#[test]
fn factory_pf_ps_pads_hold_eight_bars_at_120bpm() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("pf-") || id.starts_with("ps-"))
        .collect();
    assert_eq!(
        ids.len(),
        60,
        "expected exactly 30 pf-* + 30 ps-* factory pads, got {}: {ids:?}",
        ids.len()
    );

    const SR: u32 = 22_050;
    const TAIL_SECS: f64 = 1.0;
    let tail_n = ((TAIL_SECS * f64::from(SR)).round() as usize).max(1);

    for id in ids {
        let preset = load_factory(id).unwrap();
        assert!(
            (16.2..=18.0).contains(&preset.default_duration),
            "{id} default_duration {} must be ~16 s+ (8 bars @ 120 BPM)",
            preset.default_duration
        );

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
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN/Inf");

        let expected = (preset.default_duration * f64::from(SR)).round() as usize;
        assert_eq!(
            buf.len(),
            expected,
            "{id} sample count {} != duration*sr {}",
            buf.len(),
            expected
        );
        assert!(buf.len() >= tail_n, "{id} buffer shorter than tail window");

        let tail = &buf[buf.len() - tail_n..];
        let tail_rms = rms(tail);
        assert!(
            tail_rms > 0.01,
            "{id} last {TAIL_SECS}s is silence (rms={tail_rms}); \
             carrier sustain must hold for the full 16 s key-down"
        );

        let t14 = ((14.0 * f64::from(SR)).round() as usize).min(buf.len().saturating_sub(tail_n));
        let at14 = rms(&buf[t14..t14 + tail_n]);
        assert!(
            at14 > 0.01,
            "{id} silent at t=14s (rms={at14}); filter/amp env died too early"
        );
    }
}

#[test]
fn factory_pl_plucks_are_thirty_and_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("pl-"))
        .collect();
    assert_eq!(
        ids.len(),
        30,
        "expected exactly 30 pl-* factory plucks, got {}: {ids:?}",
        ids.len()
    );

    for id in ids {
        let preset = load_factory(id).unwrap();
        assert!(
            (0.50..=0.70).contains(&preset.default_duration),
            "{id} default_duration {} must be ~0.60s (usable one-shot, not a pad)",
            preset.default_duration
        );
        assert!(
            (48..=72).contains(&preset.default_note),
            "{id} default_note {} must be MIDI 48–72",
            preset.default_note
        );

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
            "pl pluck `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(
            peak(&buf) > 0.4,
            "pl pluck `{id}` peak {} too low",
            peak(&buf)
        );
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
    }
}

#[test]
fn factory_ep_bank_is_six_and_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("ep-"))
        .collect();
    assert_eq!(
        ids.len(),
        6,
        "expected exactly 6 ep-* factory EPs, got {}: {ids:?}",
        ids.len()
    );

    for id in ids {
        let preset = load_factory(id).unwrap();
        assert_eq!(
            preset.default_note, 48,
            "{id} default_note {} must be MIDI 48 (C3)",
            preset.default_note
        );
        assert!(
            preset.default_duration > 1.2,
            "{id} default_duration {} must be > 1.2s (not a click)",
            preset.default_duration
        );

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
            "ep `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(peak(&buf) > 0.4, "ep `{id}` peak {} too low", peak(&buf));
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );

        // Pedal-off piano dies by 1.2s. The others must still ring.
        if id != "ep-muted" {
            let sr = 22_050usize;
            let t12 = ((1.2 * sr as f64).round() as usize).min(buf.len().saturating_sub(1));
            let tail = &buf[t12.min(buf.len())..];
            if !tail.is_empty() {
                assert!(
                    rms(tail) > 0.005,
                    "{id} died before 1.2s (tail rms={})",
                    rms(tail)
                );
            }
        }
    }
}

#[test]
fn factory_vl_bank_is_audible() {
    let ids: Vec<_> = factory_ids()
        .into_iter()
        .filter(|id| id.starts_with("vl-"))
        .collect();
    assert_eq!(
        ids.len(),
        27,
        "expected exactly 27 vl-* factory vocals, got {}: {ids:?}",
        ids.len()
    );

    for id in &ids {
        let preset = load_factory(id).unwrap();
        assert_eq!(
            preset.default_note, 60,
            "{id} default_note {} must be MIDI 60 (C4)",
            preset.default_note
        );
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
            "vl `{id}` rendered near-silence (rms={})",
            rms(&buf)
        );
        assert!(peak(&buf) > 0.4, "vl `{id}` peak {} too low", peak(&buf));
        assert!(
            buf.iter().any(|&s| s.abs() > 1e-3),
            "{id} effectively silent"
        );
    }

    let ha = load_factory("vl-ha").unwrap();
    let hanami = load_factory("vl-hanami").unwrap();
    let rp = |preset: &fm_synth::Preset| RenderParams {
        frequency_hz: midi_to_hz(preset.default_note),
        duration_secs: preset.default_duration,
        velocity: 0.9,
        sample_rate: 22_050,
    };
    let buf_ha = render(&ha, &rp(&ha)).unwrap();
    let buf_hanami = render(&hanami, &rp(&hanami)).unwrap();
    assert!(
        buf_hanami.len() > buf_ha.len(),
        "vl-hanami should be longer than vl-ha"
    );

    let kasa = load_factory("vl-kasa").unwrap();
    let buf = render(&kasa, &rp(&kasa)).unwrap();
    let sr = 22_050.0f64;
    let a = (0.20 * sr).round() as usize;
    let b = (0.30 * sr).round() as usize;
    let window = &buf[a.min(buf.len())..b.min(buf.len())];
    assert!(
        rms(window) > 0.0,
        "vl-kasa 0.20–0.30s window was silent (no /s/)"
    );
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
            ..ExportParams::default()
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
            ..ExportParams::default()
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

const STRUDEL_ONESHOT_IDS: [&str; 5] = [
    "cp-house",
    "lead-fm-pluck",
    "stab-fm-fifth",
    "stab-fm-major",
    "reese-mid",
];

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
fn stab_fm_major_has_e_unlike_hollow_fifth() {
    let preset = load_factory("stab-fm-major").unwrap();
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
    assert!(buf.iter().all(|s| s.is_finite()), "NaN/Inf");
    assert!(rms(&buf) > 0.01, "near-silence (rms={})", rms(&buf));

    // Same decay-body window as the hollow-fifth test so the 5:4 bin is
    // comparable: hollow asserts E is absent; this asserts E is present.
    let start = (sr as usize) / 25;
    let end = ((sr as usize) * 3 / 20).min(buf.len());
    assert!(end > start + 64, "not enough decay body to measure");
    let body = hann_window(&buf[start..end]);
    let c = goertzel_power(&body, sr as f32, f0);
    let e = goertzel_power(&body, sr as f32, f0 * 1.25);
    let g = goertzel_power(&body, sr as f32, f0 * 1.5);

    assert!(
        c > 0.0 && e > 0.0 && g > 0.0,
        "missing C, E, or G (c={c}, e={e}, g={g})"
    );
    let cg = c.min(g);
    assert!(
        e > cg * 0.25,
        "major third (5:4) should be present unlike stab-fm-fifth (e={e}, cg={cg})"
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

    // After the ~200 ms FM attack window — body must still be C3, not a ratio-2 carrier.
    let start = (sr as usize) * 9 / 20; // 0.45 s
    let end = ((sr as usize) * 4 / 5).min(buf.len());
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
    // Closed-hat air (7–14 kHz) must not swallow the 1 kHz body.
    let hat = band(7000.0, 14000.0);
    assert!(
        mid > hat * 0.4,
        "cp-house 1 kHz body lost to hat-range air (mid={mid}, hat={hat})"
    );
    // A sine at ~1 kHz would spike one bin against its neighbours in the body.
    let mut body_bins = Vec::new();
    let mut f = 900.0;
    while f <= 1600.0 {
        body_bins.push(goertzel_power(&body, sr_f, f));
        f += 50.0;
    }
    let body_mean = body_bins.iter().sum::<f64>() / body_bins.len() as f64;
    let body_max = body_bins.iter().copied().fold(0.0, f64::max);
    assert!(
        body_mean > 0.0 && body_max < body_mean * 12.0,
        "cp-house 1 kHz body should be a band, not a beep (max={body_max}, mean={body_mean})"
    );

    // Decay is time-to--80 dB; a 40 ms decay is a click. 60–120 ms must still
    // carry the パン body.
    let sr_u = sr as usize;
    let t40 = sr_u / 25;
    let t60 = (sr_u * 3) / 50;
    let t120 = (sr_u * 3) / 25;
    assert!(t120 < buf.len(), "cp-house buffer shorter than 120 ms");
    let attack_rms = rms(&buf[..t40]);
    let body_rms = rms(&buf[t60..t120]);
    assert!(
        body_rms > attack_rms * 0.10,
        "cp-house died like a click by 60-120 ms (body_rms={body_rms}, attack_rms={attack_rms})"
    );
}

#[test]
fn ep_rhodes_soft_attack_has_tine_2x_3x_unlike_sine() {
    let preset = load_factory("ep-rhodes-soft").unwrap();
    assert_eq!(preset.default_note, 48, "MIDI 48 = C3 ≈ 130.8 Hz");
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
    assert!(buf.iter().all(|s| s.is_finite()), "NaN/Inf");
    assert!(rms(&buf) > 0.01, "near-silence (rms={})", rms(&buf));

    // Attack window: tines (2× / 3×) live here, then decay toward the body.
    let start = (sr as usize) / 500;
    let end = ((sr as usize) / 12).min(buf.len());
    assert!(end > start + 64, "not enough attack to measure");
    let attack = hann_window(&buf[start..end]);

    let fund = goertzel_power(&attack, sr as f32, f0);
    let h2 = goertzel_power(&attack, sr as f32, f0 * 2.0);
    let h3 = goertzel_power(&attack, sr as f32, f0 * 3.0);
    let bell = goertzel_power(&attack, sr as f32, f0 * 3.5);

    let sine: Vec<f32> = (0..attack.len())
        .map(|i| {
            let t = i as f32 / sr as f32;
            (std::f32::consts::TAU * f0 * t).sin()
        })
        .collect();
    let sine_w = hann_window(&sine);
    let sine_h2 = goertzel_power(&sine_w, sr as f32, f0 * 2.0);
    let sine_h3 = goertzel_power(&sine_w, sr as f32, f0 * 3.0);

    assert!(
        fund > 0.0 && h2 > 0.0 && h3 > 0.0,
        "missing 1×/2×/3× on attack (1×={fund}, 2×={h2}, 3×={h3})"
    );
    assert!(
        h2 > sine_h2 * 20.0,
        "2× tine (~262 Hz) no stronger than a pure sine (h2={h2}, sine={sine_h2})"
    );
    assert!(
        h3 > sine_h3 * 20.0,
        "3× tine (~392 Hz) no stronger than a pure sine (h3={h3}, sine={sine_h3})"
    );
    assert!(
        h2 > fund * 0.08,
        "2× tine too weak vs body on attack (h2={h2}, fund={fund})"
    );
    assert!(
        h3 > fund * 0.05,
        "3× tine too weak vs body on attack (h3={h3}, fund={fund})"
    );
    assert!(
        h2 > bell && h3 > bell,
        "inharmonic 3.5× bell partial beat the tines (h2={h2}, h3={h3}, bell={bell})"
    );
}

#[test]
fn ep_wurli_attack_has_audible_2x_tine() {
    let preset = load_factory("ep-wurli").unwrap();
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
    let start = (sr as usize) / 500;
    let end = ((sr as usize) / 12).min(buf.len());
    let attack = hann_window(&buf[start..end]);
    let fund = goertzel_power(&attack, sr as f32, f0);
    let h2 = goertzel_power(&attack, sr as f32, f0 * 2.0);
    let h3 = goertzel_power(&attack, sr as f32, f0 * 3.0);
    assert!(
        h2 > fund * 0.08,
        "wurli 2× tine missing on attack (h2={h2}, fund={fund})"
    );
    assert!(
        h3 > fund * 0.05,
        "wurli 3× tine missing on attack (h3={h3}, fund={fund})"
    );
}

#[test]
fn ep_rhodes_hard_and_muted_differ_from_soft_body() {
    fn shot(id: &str) -> (u32, Vec<f32>, f32) {
        let preset = load_factory(id).unwrap();
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
        (sr, buf, f0)
    }
    let (sr, soft, f0) = shot("ep-rhodes-soft");
    let (_, hard, _) = shot("ep-rhodes-hard");
    let (_, muted, _) = shot("ep-muted");
    let a = ((sr as usize) * 3 / 10).min(soft.len().min(hard.len()) - 65);
    let b = ((sr as usize) * 6 / 10).min(soft.len().min(hard.len()));
    let soft_w = hann_window(&soft[a..b]);
    let hard_w = hann_window(&hard[a..b]);
    let soft_fund = goertzel_power(&soft_w, sr as f32, f0);
    let soft_h2 = goertzel_power(&soft_w, sr as f32, f0 * 2.0);
    let hard_fund = goertzel_power(&hard_w, sr as f32, f0);
    let hard_h2 = goertzel_power(&hard_w, sr as f32, f0 * 2.0);
    let soft_ratio = soft_h2 / (soft_fund + 1e-12);
    let hard_ratio = hard_h2 / (hard_fund + 1e-12);
    assert!(
        hard_ratio > soft_ratio * 3.0,
        "hard body should keep 2× bark (hard={hard_ratio}, soft={soft_ratio})"
    );
    let late = ((sr as usize) * 12 / 10).min(soft.len().min(muted.len()) - 1);
    let late_n = (sr as usize / 5)
        .min(soft.len() - late)
        .min(muted.len() - late);
    let soft_late = rms(&soft[late..late + late_n]);
    let muted_late = rms(&muted[late..late + late_n]);
    assert!(
        muted_late < soft_late * 0.45,
        "muted should die unlike the Rhodes organ hold (muted={muted_late}, soft={soft_late})"
    );
}

#[test]
fn ep_muted_dies_while_sustain_rings() {
    fn shot(id: &str) -> (u32, Vec<f32>) {
        let preset = load_factory(id).unwrap();
        let sr = 48_000u32;
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: midi_to_hz(48),
                duration_secs: preset.default_duration,
                velocity: 0.9,
                sample_rate: sr,
            },
        )
        .unwrap();
        (sr, buf)
    }
    let (sr, muted) = shot("ep-muted");
    let (_, sus) = shot("ep-sustain");
    let t = ((sr as usize) * 12 / 10).min(muted.len().min(sus.len()) - 1);
    let n = (sr as usize / 5).min(muted.len() - t).min(sus.len() - t);
    let muted_late = rms(&muted[t..t + n]);
    let sus_late = rms(&sus[t..t + n]);
    assert!(
        muted_late < 0.04,
        "muted still singing at 1.2s (rms={muted_late})"
    );
    assert!(
        sus_late > muted_late * 4.0,
        "sustain should still ring (sustain={sus_late}, muted={muted_late})"
    );
}

#[test]
fn bs_house_tight_has_mid_click_on_attack() {
    let preset = load_factory("bs-house-tight").unwrap();
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
    let click_n = ((sr as usize) / 25).min(buf.len()); // ~40 ms
    let late_a = ((sr as usize) / 5).min(buf.len()); // ~200 ms
    let late_b = ((sr as usize) / 3).min(buf.len()); // ~333 ms
    let sr_f = sr as f32;
    let band = |slice: &[f32], lo, hi| {
        let w = hann_window(slice);
        let mut e = 0.0;
        let mut f = lo;
        while f < hi {
            e += goertzel_power(&w, sr_f, f);
            f += 40.0;
        }
        e
    };
    let click = band(&buf[..click_n], 800.0, 2800.0);
    let click_late = band(&buf[late_a..late_b], 800.0, 2800.0);
    assert!(click > 0.0, "house-tight mid click missing (click={click})");
    assert!(
        click > click_late * 6.0,
        "house-tight mid click should live on the attack (click={click}, late={click_late})"
    );
}

#[test]
fn pc_hat_closed_has_7khz_sizzle_not_nyquist_tick() {
    let preset = load_factory("pc-hat-closed").unwrap();
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
    let sr_f = sr as f32;
    let body = hann_window(&buf);
    let band = |lo, hi| {
        let mut e = 0.0;
        let mut f = lo;
        while f < hi {
            e += goertzel_power(&body, sr_f, f);
            f += 80.0;
        }
        e
    };
    let sizzle = band(6000.0, 10000.0);
    let air_tick = band(14000.0, 20000.0);
    assert!(
        sizzle > air_tick * 1.2,
        "closed hat should sizzle near 7-8 kHz, not a 16-20 kHz tick (sizzle={sizzle}, air={air_tick})"
    );

    let sr_u = sr as usize;
    let early = rms(&buf[..sr_u / 100]); // 0-10 ms
    let t10 = sr_u / 100;
    let t40 = sr_u / 25;
    let late = rms(&buf[t10..t40]); // 10-40 ms
    assert!(
        late > early * 0.18,
        "closed hat died like a click by 10-40 ms (late={late}, early={early})"
    );
    let t80 = (sr_u * 2) / 25;
    assert!(t80 < buf.len(), "closed hat buffer shorter than 80 ms");
    let body = rms(&buf[t40..t80]); // 40-80 ms
    assert!(
        body > early * 0.06,
        "closed hat should still sizzle at 40-80 ms (body={body}, early={early})"
    );
}

#[test]
fn noise_only_white_is_sand_not_a_tone() {
    let toml = r#"
name = "noise-only"
algorithm = 8
gain = 1.0
default_note = 60
default_duration = 0.25
[noise]
type = "white"
level = 1.0
attack = 0.0
decay = 0.0
sustain = 1.0
release = 0.02
vel_sens = 0.0
[noise.filter]
type = "highpass"
cutoff = 4000.0
[[operators]]
level = 0.0
[[operators]]
level = 0.0
[[operators]]
level = 0.0
[[operators]]
level = 0.0
"#;
    let preset = Preset::from_toml_str("noise-only", toml).unwrap();
    let sr = 48_000u32;
    let buf = render(
        &preset,
        &RenderParams {
            frequency_hz: 261.63,
            duration_secs: 0.25,
            velocity: 1.0,
            sample_rate: sr,
        },
    )
    .unwrap();
    assert!(buf.iter().all(|s| s.is_finite()), "NaN/Inf");
    assert!(rms(&buf) > 0.02, "noise-only silent (rms={})", rms(&buf));
    let body = hann_window(&buf);
    let sr_f = sr as f32;
    let mut bins = Vec::new();
    let mut f = 500.0;
    while f <= 8_000.0 {
        bins.push(goertzel_power(&body, sr_f, f));
        f += 80.0;
    }
    let mean = bins.iter().sum::<f64>() / bins.len() as f64;
    let max = bins.iter().copied().fold(0.0, f64::max);
    assert!(
        mean > 0.0 && max < mean * 12.0,
        "white noise should be a band, not a beep (max={max}, mean={mean})"
    );
}
