//! Render-time downward octave mix for factory FX: full 4OP voices at
//! N, N−12, N−24 (and maybe N−36). Not operator-ratio surgery or chorus.

use fm_synth::{
    analyze_preset, factory_ids, is_factory_fx_id, is_factory_lead_id, is_pitched_fx_id,
    load_factory, midi_to_hz, render, render_export, ExportParams, LayerInterval, LayerMode,
    LAYER_GAIN_OCTAVE_DOWN, LAYER_GAIN_OCTAVE_DOWN_2, LAYER_GAIN_ROOT,
};

const SR: u32 = 22_050;

fn export(layers: LayerMode, secs: f64) -> ExportParams {
    ExportParams {
        duration: Some(secs),
        sample_rate: SR,
        layers,
        ..ExportParams::default()
    }
}

fn goertzel_power(buf: &[f32], sr: f32, freq: f32) -> f64 {
    if freq <= 0.0 || freq >= sr * 0.5 || buf.len() < 8 {
        return 0.0;
    }
    let n = buf.len() as f64;
    let k = (f64::from(freq) * n / f64::from(sr)).round();
    let w = std::f64::consts::TAU * k / n;
    let coeff = 2.0 * w.cos();
    let mut s1 = 0.0;
    let mut s2 = 0.0;
    for &x in buf {
        let s0 = f64::from(x) + coeff * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    (s1 * s1 + s2 * s2 - coeff * s1 * s2).max(0.0)
}

fn hann(buf: &[f32]) -> Vec<f32> {
    let n = buf.len() as f32;
    buf.iter()
        .enumerate()
        .map(|(i, &x)| {
            let w = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (n - 1.0)).cos();
            x * w
        })
        .collect()
}

fn early_body(buf: &[f32]) -> Vec<f32> {
    let start = buf.len() / 16;
    let end = (buf.len() * 9 / 16).max(start + 256).min(buf.len());
    hann(&buf[start..end])
}

#[test]
fn pitched_fx_auto_stacks_octave_downs_leads_stay_up() {
    let laser = load_factory("fx-laser").unwrap();
    let laser_out = render_export("fx-laser", &laser, &export(LayerMode::Auto, 0.35)).unwrap();
    assert!(
        laser_out.semitones.contains(&0)
            && laser_out.semitones.contains(&-12)
            && laser_out.semitones.contains(&-24),
        "fx-laser auto should stack 0/-12/-24, got {:?}",
        laser_out.semitones
    );
    assert!(
        !laser_out.semitones.contains(&12),
        "fx-laser must not grow an upward octave, got {:?}",
        laser_out.semitones
    );

    let lead = load_factory("ld-sine").unwrap();
    let lead_out = render_export("ld-sine", &lead, &export(LayerMode::Auto, 0.35)).unwrap();
    assert!(
        lead_out.semitones.contains(&12),
        "lead auto must keep +12, got {:?}",
        lead_out.semitones
    );
    assert!(
        !lead_out.semitones.contains(&-12),
        "lead auto must not grow a downward octave, got {:?}",
        lead_out.semitones
    );
}

#[test]
fn laser_down_stack_is_separate_full_voices() {
    let preset = load_factory("fx-laser").unwrap();
    let secs = 0.4;
    let single = render(
        &preset,
        &fm_synth::RenderParams {
            frequency_hz: midi_to_hz(preset.default_note),
            duration_secs: secs,
            velocity: 0.9,
            sample_rate: SR,
        },
    )
    .unwrap();
    let layered = render_export(
        "fx-laser",
        &preset,
        &export(
            LayerMode::Explicit(vec![LayerInterval::OctaveDown, LayerInterval::OctaveDown2]),
            secs,
        ),
    )
    .unwrap();
    assert_eq!(layered.samples.len(), single.len());
    assert_eq!(layered.semitones, vec![-24, -12, 0]);

    let f0 = midi_to_hz(preset.default_note) as f32;
    let off_w = early_body(&single);
    let lay_w = early_body(&layered.samples);
    let off0 = goertzel_power(&off_w, SR as f32, f0);
    let off_d1 = goertzel_power(&off_w, SR as f32, f0 * 0.5);
    let off_d2 = goertzel_power(&off_w, SR as f32, f0 * 0.25);
    let l0 = goertzel_power(&lay_w, SR as f32, f0);
    let ld1 = goertzel_power(&lay_w, SR as f32, f0 * 0.5);
    let ld2 = goertzel_power(&lay_w, SR as f32, f0 * 0.25);

    assert!(
        l0 > 0.0 && ld1 > 0.0 && ld2 > 0.0,
        "all three voices present"
    );
    assert!(
        ld1 > off_d1 * 1.6,
        "−12 must be a second full render (single={off_d1}, layered={ld1})"
    );
    assert!(
        ld2 > off_d2 * 1.5,
        "−24 must be a third full render (single={off_d2}, layered={ld2})"
    );
    let r1 = ld1 / l0;
    let r2 = ld2 / l0;
    let expect1 = (LAYER_GAIN_OCTAVE_DOWN / LAYER_GAIN_ROOT).powi(2);
    let expect2 = (LAYER_GAIN_OCTAVE_DOWN_2 / LAYER_GAIN_ROOT).powi(2);
    assert!(
        r1 > expect1 * 0.25,
        "−12/root power {r1} should be in the neighborhood of {expect1}"
    );
    assert!(
        r2 < r1,
        "−24 should taper under −12 ({r2} vs {r1}); expected ~{expect2}"
    );
}

#[test]
fn zap_and_riser_auto_use_down_stack() {
    for id in ["fx-zap", "zap", "fx-riser-pitch"] {
        let preset = load_factory(id).unwrap();
        let out = render_export(id, &preset, &export(LayerMode::Auto, 0.45)).unwrap();
        assert!(
            out.semitones.contains(&-12) && out.semitones.contains(&-24),
            "{id} auto should include −12 and −24, got {:?}",
            out.semitones
        );
    }
}

#[test]
fn boom_gets_mild_down_noise_hit_does_too_sub_drop_does_not() {
    let boom = render_export(
        "fx-boom",
        &load_factory("fx-boom").unwrap(),
        &export(LayerMode::Auto, 0.5),
    )
    .unwrap();
    assert_eq!(
        boom.semitones,
        vec![-12, 0],
        "impact FX should be a single octave-down, got {:?}",
        boom.semitones
    );

    let hit = render_export(
        "fx-noise-hit",
        &load_factory("fx-noise-hit").unwrap(),
        &export(LayerMode::Auto, 0.3),
    )
    .unwrap();
    assert_eq!(
        hit.semitones,
        vec![-12, 0],
        "noise FX get a mild −12, got {:?}",
        hit.semitones
    );

    let drop = render_export(
        "fx-sub-drop",
        &load_factory("fx-sub-drop").unwrap(),
        &export(LayerMode::Auto, 0.5),
    )
    .unwrap();
    assert_eq!(
        drop.semitones,
        vec![0],
        "sub-drop is already a falling sub, got {:?}",
        drop.semitones
    );
}

#[test]
fn factory_fx_bank_is_classified_and_leads_are_not() {
    for id in factory_ids() {
        if is_factory_fx_id(id) {
            assert!(
                !is_factory_lead_id(id),
                "{id} must not be classified as both FX and lead"
            );
        }
    }
    assert!(is_pitched_fx_id("fx-laser"));
    assert!(is_pitched_fx_id("fm-riser"));
    assert!(!is_pitched_fx_id("fx-riser-noise"));
}

#[test]
fn analyze_laser_stack_is_richer_than_single_note() {
    let preset = load_factory("fx-laser").unwrap();
    let (single, a_off) = analyze_preset(
        "fx-laser",
        &preset,
        &export(LayerMode::Off, 0.4),
        Some("single"),
    )
    .unwrap();
    let (layered, a_auto) = analyze_preset(
        "fx-laser",
        &preset,
        &export(LayerMode::Auto, 0.4),
        Some("auto-downs"),
    )
    .unwrap();
    assert_eq!(single.len(), layered.len());
    assert!(
        a_auto.report.render_layers.contains(&"octave-down".into())
            && a_auto
                .report
                .render_layers
                .contains(&"octave-down-2".into()),
        "analyze should report down layers, got {:?}",
        a_auto.report.render_layers
    );
    let low_off = a_off.report.band_energy.sub_20_80 + a_off.report.band_energy.bass_80_250;
    let low_auto = a_auto.report.band_energy.sub_20_80 + a_auto.report.band_energy.bass_80_250;
    assert!(
        low_auto + 0.01 >= low_off,
        "layered laser should not lose low-band share (off={low_off}, auto={low_auto})"
    );
    let f0 = midi_to_hz(preset.default_note) as f32;
    let off_w = early_body(&single);
    let auto_w = early_body(&layered);
    let off_d1 = goertzel_power(&off_w, SR as f32, f0 * 0.5);
    let auto_d1 = goertzel_power(&auto_w, SR as f32, f0 * 0.5);
    assert!(
        auto_d1 > off_d1 * 1.4,
        "auto −12 ridge should beat the single note (off={off_d1}, auto={auto_d1})"
    );
}

#[test]
fn layers_off_matches_single_render_for_fx() {
    let preset = load_factory("fx-zap").unwrap();
    let single = render(
        &preset,
        &fm_synth::RenderParams {
            frequency_hz: midi_to_hz(preset.default_note),
            duration_secs: 0.3,
            velocity: 0.9,
            sample_rate: SR,
        },
    )
    .unwrap();
    let off = render_export("fx-zap", &preset, &export(LayerMode::Off, 0.3)).unwrap();
    assert_eq!(off.semitones, vec![0]);
    let err: f32 = off
        .samples
        .iter()
        .zip(&single)
        .map(|(a, b)| (a - b).abs())
        .sum();
    assert!(err < 1e-4, "Off should be a single 4OP render, err={err}");
}
