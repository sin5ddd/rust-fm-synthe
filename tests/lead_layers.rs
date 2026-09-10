//! Render-time octave / fifth mix: two (or three) full 4OP voices, not
//! operator surgery or `[fx.chorus] intervals`.

use fm_synth::{
    analyze_preset, load_factory, midi_to_hz, render, render_export, semitones_to_ratio, Algorithm,
    ExportParams, LayerInterval, LayerMode, Preset, RenderParams, LAYER_GAIN_FIFTH,
    LAYER_GAIN_OCTAVE, LAYER_GAIN_ROOT,
};

const SR: u32 = 22_050;

fn sine_patch() -> Preset {
    let toml = r#"
name = "layer-test-sine"
algorithm = 8
gain = 1.0
default_note = 48
default_duration = 0.5
[filter]
type = "lowpass"
cutoff = 16000
resonance = 0.0
env_amount = 0.0
[[operators]]
ratio = 1.0
level = 1.0
attack = 0.005
decay = 0.0
sustain = 1.0
release = 0.02
waveform = "sine"
[[operators]]
level = 0.0
[[operators]]
level = 0.0
[[operators]]
level = 0.0
"#;
    Preset::from_toml_str("layer-test-sine", toml).unwrap()
}

fn params(preset: &Preset, secs: f64) -> RenderParams {
    RenderParams {
        frequency_hz: midi_to_hz(preset.default_note),
        duration_secs: secs,
        velocity: 0.9,
        sample_rate: SR,
    }
}

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

fn body(buf: &[f32]) -> Vec<f32> {
    let start = buf.len() / 8;
    let end = (buf.len() * 5 / 8).max(start + 256).min(buf.len());
    hann(&buf[start..end])
}

#[test]
fn mix_keeps_duration_and_octave_gain_ratio() {
    let preset = sine_patch();
    let p = params(&preset, 0.45);
    let single = render(&preset, &p).unwrap();
    let layered = render_export(
        "layer-test-sine",
        &preset,
        &export(LayerMode::Explicit(vec![LayerInterval::Octave]), 0.45),
    )
    .unwrap();

    assert_eq!(layered.samples.len(), single.len());
    assert_eq!(layered.semitones, vec![0, 12]);

    let f0 = p.frequency_hz as f32;
    let win = body(&layered.samples);
    let e0 = goertzel_power(&win, SR as f32, f0);
    let e2 = goertzel_power(&win, SR as f32, f0 * 2.0);
    let ratio = e2 / e0;
    let expected = (LAYER_GAIN_OCTAVE / LAYER_GAIN_ROOT).powi(2);
    assert!(
        (ratio - f64::from(expected)).abs() < 0.18,
        "octave/root power {ratio} expected ~{expected} (e0={e0}, e2={e2})"
    );
    assert!(e0 > 0.0 && e2 > 0.0, "both voices must be present");
}

#[test]
fn fifth_layer_is_quieter_and_adds_15xf0() {
    let preset = sine_patch();
    let layered = render_export(
        "layer-test-sine",
        &preset,
        &export(
            LayerMode::Explicit(vec![LayerInterval::Octave, LayerInterval::Fifth]),
            0.45,
        ),
    )
    .unwrap();
    assert_eq!(layered.semitones, vec![0, 7, 12]);

    let f0 = layered.frequency_hz;
    let win = body(&layered.samples);
    let e0 = goertzel_power(&win, SR as f32, f0 as f32);
    let e15 = goertzel_power(&win, SR as f32, (f0 * semitones_to_ratio(7.0)) as f32);
    let e2 = goertzel_power(&win, SR as f32, (f0 * 2.0) as f32);
    assert!(e0 > 0.0 && e15 > 0.0 && e2 > 0.0);
    let fifth_ratio = e15 / e0;
    let oct_ratio = e2 / e0;
    let expected_fifth = (LAYER_GAIN_FIFTH / LAYER_GAIN_ROOT).powi(2);
    assert!(
        (fifth_ratio - f64::from(expected_fifth)).abs() < 0.16,
        "fifth/root {fifth_ratio} expected ~{expected_fifth}"
    );
    assert!(
        fifth_ratio < oct_ratio,
        "fifth should be quieter than octave ({fifth_ratio} vs {oct_ratio})"
    );
}

#[test]
fn factory_leads_auto_stack_non_leads_do_not() {
    let lead = load_factory("ld-sine").unwrap();
    let bass = load_factory("sub-bass").unwrap();
    let lead_out = render_export("ld-sine", &lead, &export(LayerMode::Auto, 0.35)).unwrap();
    let bass_out = render_export("sub-bass", &bass, &export(LayerMode::Auto, 0.35)).unwrap();
    assert!(
        lead_out.semitones.contains(&12),
        "factory lead auto should include +12, got {:?}",
        lead_out.semitones
    );
    assert_eq!(bass_out.semitones, vec![0]);
}

#[test]
fn layers_off_matches_single_render() {
    let preset = load_factory("ld-fm-pluck").unwrap();
    let single = render(&preset, &params(&preset, 0.4)).unwrap();
    let off = render_export("ld-fm-pluck", &preset, &export(LayerMode::Off, 0.4)).unwrap();
    assert_eq!(off.semitones, vec![0]);
    assert_eq!(off.samples.len(), single.len());
    let err: f32 = off
        .samples
        .iter()
        .zip(&single)
        .map(|(a, b)| (a - b).abs())
        .sum();
    assert!(err < 1e-4, "Off should be a single 4OP render, err={err}");
}

#[test]
fn ld_fm_pluck_stays_serial_and_late_octave_is_a_second_voice() {
    let preset = load_factory("ld-fm-pluck").unwrap();
    assert_eq!(preset.algorithm, Algorithm::Serial);
    assert_eq!(preset.default_note, 48);
    assert!(preset.render_layers.is_none());

    let secs = 0.9;
    let single = render(&preset, &params(&preset, secs)).unwrap();
    let layered = render_export(
        "ld-fm-pluck",
        &preset,
        &export(LayerMode::Explicit(vec![LayerInterval::Octave]), secs),
    )
    .unwrap();
    assert_eq!(layered.samples.len(), single.len());

    let f0 = midi_to_hz(48) as f32;
    let start = (SR as usize) * 2 / 5; // 0.4 s — ~200 ms FM attack has settled
    let end = ((SR as usize) * 7 / 10).min(single.len());
    let single_w = hann(&single[start..end]);
    let layer_w = hann(&layered.samples[start..end]);
    let s0 = goertzel_power(&single_w, SR as f32, f0);
    let s2 = goertzel_power(&single_w, SR as f32, f0 * 2.0);
    let l0 = goertzel_power(&layer_w, SR as f32, f0);
    let l2 = goertzel_power(&layer_w, SR as f32, f0 * 2.0);

    assert!(
        s0 > s2 * 4.0,
        "single-note sustain should be C3, not a ratio-2 carrier (c3={s0}, c4={s2})"
    );
    assert!(
        l0 > 0.0 && l2 > l0 * 0.25,
        "layered sustain must keep C3 and a real C4 voice (c3={l0}, c4={l2})"
    );
    assert!(
        l2 / l0 > (s2 / s0) * 3.0,
        "late 2×f0 must come from a second full render, not the dying FM click (single {} layered {})",
        s2 / s0,
        l2 / l0
    );
}

#[test]
fn analyze_octave_stack_is_thicker_than_single_note() {
    let ids = ["ld-sine", "ld-fm-pluck", "ld-chip", "ld-supersaw"];
    for id in ids {
        let preset = load_factory(id).unwrap();
        let (single, a_off) =
            analyze_preset(id, &preset, &export(LayerMode::Off, 0.7), Some("single")).unwrap();
        let (layered, a_oct) = analyze_preset(
            id,
            &preset,
            &export(LayerMode::Explicit(vec![LayerInterval::Octave]), 0.7),
            Some("octave"),
        )
        .unwrap();
        assert_eq!(single.len(), layered.len(), "{id} duration");
        assert_eq!(a_oct.report.render_layers, ["unison", "octave"]);

        let f0 = midi_to_hz(preset.default_note) as f32;
        let off_w = body(&single);
        let oct_w = body(&layered);
        let off2 = goertzel_power(&off_w, SR as f32, f0 * 2.0);
        let oct2 = goertzel_power(&oct_w, SR as f32, f0 * 2.0);
        assert!(
            oct2 > off2 * 1.4,
            "{id} layered 2×f0 should be thicker (single={off2}, layered={oct2})"
        );
        assert!(
            a_oct.report.band_energy.mid_250_2000 + a_oct.report.band_energy.high_2000_plus
                >= a_off.report.band_energy.mid_250_2000 + a_off.report.band_energy.high_2000_plus
                    - 0.02,
            "{id} layered should not lose mid/high presence"
        );
    }
}

#[test]
fn thin_auto_lead_can_gain_a_fifth() {
    let preset = load_factory("ld-sine").unwrap();
    let out = render_export("ld-sine", &preset, &export(LayerMode::Auto, 0.5)).unwrap();
    assert!(
        out.semitones.contains(&12),
        "ld-sine auto must include octave, got {:?}",
        out.semitones
    );
    assert!(
        out.semitones.contains(&7),
        "thin sine+octave should also mix a fifth, got {:?}",
        out.semitones
    );
    let f0 = out.frequency_hz;
    let win = body(&out.samples);
    let e15 = goertzel_power(&win, SR as f32, (f0 * semitones_to_ratio(7.0)) as f32);
    let e0 = goertzel_power(&win, SR as f32, f0 as f32);
    assert!(
        e15 > e0 * 0.08,
        "auto fifth should add 1.5×f0 presence (e0={e0}, e15={e15})"
    );

    let chip = load_factory("ld-chip").unwrap();
    let chip_out = render_export("ld-chip", &chip, &export(LayerMode::Auto, 0.45)).unwrap();
    assert_eq!(
        chip_out.semitones,
        vec![0, 12],
        "bright chip should stay octave-only, got {:?}",
        chip_out.semitones
    );
}

fn fm_partial_energy(buf: &[f32], t0: f64, t1: f64, f0: f32) -> f64 {
    let a = ((t0 * f64::from(SR)) as usize).min(buf.len());
    let b = ((t1 * f64::from(SR)) as usize).min(buf.len());
    assert!(b > a + 64, "window {t0}..{t1}");
    let w = hann(&buf[a..b]);
    goertzel_power(&w, SR as f32, f0 * 3.0)
        + goertzel_power(&w, SR as f32, f0 * 5.0)
        + goertzel_power(&w, SR as f32, f0 * 7.0)
}

#[test]
fn fm_plucks_keep_serial_fm_bite_for_about_200ms() {
    for id in ["ld-fm-pluck", "lead-fm-pluck"] {
        let preset = load_factory(id).unwrap();
        assert_eq!(preset.algorithm, Algorithm::Serial, "{id}");
        let mods: Vec<_> = preset
            .operators
            .iter()
            .skip(1)
            .filter(|op| op.level > 1e-6)
            .collect();
        assert!(
            mods.iter().any(|op| (0.15..=0.28).contains(&op.decay)),
            "{id} modulator decay should last ~200ms, got {:?}",
            mods.iter().map(|op| op.decay).collect::<Vec<_>>()
        );
        assert!(
            mods.iter().all(|op| op.sustain < 0.05),
            "{id} modulators must die after the attack (not become sustain carriers)"
        );

        let buf = render(&preset, &params(&preset, 0.9)).unwrap();
        let f0 = midi_to_hz(48) as f32;
        let early = fm_partial_energy(&buf, 0.04, 0.16, f0);
        let late_attack = fm_partial_energy(&buf, 0.12, 0.22, f0);
        let settled = fm_partial_energy(&buf, 0.40, 0.60, f0);
        assert!(
            early > settled * 3.0,
            "{id} FM bite should be much brighter in the first 160ms than after 400ms (early={early}, settled={settled})"
        );
        assert!(
            late_attack > settled * 1.6,
            "{id} FM should still be audible near 200ms (late_attack={late_attack}, settled={settled})"
        );
    }
}

#[test]
fn preset_render_layers_opt_in_beats_auto_off_for_non_leads() {
    let mut bass = load_factory("sub-bass").unwrap();
    bass.render_layers = Some(vec![LayerInterval::Octave]);
    let out = render_export("sub-bass", &bass, &export(LayerMode::Auto, 0.25)).unwrap();
    assert_eq!(out.semitones, vec![0, 12]);
}
