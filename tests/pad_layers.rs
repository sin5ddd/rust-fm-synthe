//! Render-time ±octave mix for sparkle / airy pads: full 4OP voices at
//! N, N+12, N−12 (few-Hz detune), plus +7/+24 with mid BP when thin.
//! Not operator-ratio surgery or `[fx.chorus] intervals`.

use fm_synth::{
    analyze_preset, factory_ids, is_airy_fresh_pad_id, is_factory_lead_id, is_factory_pad_stack_id,
    is_factory_sparkle_pad_id, load_factory, midi_to_hz, render, render_export, resolve_layer_plan,
    semitones_to_ratio, AutoLayerExtra, ExportParams, LayerInterval, LayerMode, Preset,
    PAD_DETUNE_HZ_FIFTH, PAD_DETUNE_HZ_OCTAVE2, PAD_DETUNE_HZ_OCTAVE_DOWN, PAD_DETUNE_HZ_OCTAVE_UP,
    PAD_HOLD_SECS_AT_130, PAD_MID_BP_HI_HZ, PAD_MID_BP_LO_HZ,
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

fn late_body(buf: &[f32]) -> Vec<f32> {
    // After a multi-second wind attack: last third of the hold.
    let start = buf.len() * 2 / 3;
    let end = buf.len().saturating_sub(buf.len() / 12).max(start + 256);
    hann(&buf[start..end.min(buf.len())])
}

fn sine_patch() -> Preset {
    let toml = r#"
name = "layer-test-sine-pad"
algorithm = 8
gain = 1.0
default_note = 60
default_duration = 0.6
[filter]
type = "lowpass"
cutoff = 16000
resonance = 0.0
env_amount = 0.0
[[operators]]
ratio = 1.0
level = 1.0
attack = 0.01
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
    Preset::from_toml_str("layer-test-sine-pad", toml).unwrap()
}

#[test]
fn pad_duration_is_four_wholes_at_130() {
    assert!((PAD_HOLD_SECS_AT_130 - 960.0 / 130.0).abs() < 1e-12);
    for id in [
        "ps-crystal",
        "ps-shimmer",
        "ps-fm-sparkle",
        "pf-flute-pad",
        "pf-choir-air",
        "pf-silk",
    ] {
        let p = load_factory(id).unwrap();
        assert!(
            (p.default_duration - PAD_HOLD_SECS_AT_130).abs() < 1e-6,
            "{id} duration {} != {PAD_HOLD_SECS_AT_130}",
            p.default_duration
        );
    }
}

#[test]
fn sparkle_and_airy_auto_plan_is_plus_minus_octave() {
    let crystal = load_factory("ps-crystal").unwrap();
    let plan = resolve_layer_plan("ps-crystal", &crystal, &LayerMode::Auto);
    assert_eq!(
        plan.intervals,
        vec![LayerInterval::Octave, LayerInterval::OctaveDown]
    );
    assert_eq!(plan.extra, AutoLayerExtra::FifthAndOctave2);

    let flute = load_factory("pf-flute-pad").unwrap();
    let flute_plan = resolve_layer_plan("pf-flute-pad", &flute, &LayerMode::Auto);
    assert_eq!(flute_plan.intervals, plan.intervals);
    assert_eq!(flute_plan.extra, AutoLayerExtra::FifthAndOctave2);

    assert!(is_factory_sparkle_pad_id("ps-glitter"));
    assert!(is_airy_fresh_pad_id("pf-silk"));
    assert!(is_factory_pad_stack_id("pf-choir-air"));
    assert!(!is_factory_pad_stack_id("pf-juno-air"));
    assert!(!is_factory_pad_stack_id("ld-flute"));
}

#[test]
fn pad_auto_stacks_detuned_octaves_leads_stay_up_only() {
    let pad = load_factory("ps-crystal").unwrap();
    let out = render_export("ps-crystal", &pad, &export(LayerMode::Auto, 1.2)).unwrap();
    assert!(
        out.semitones.contains(&0)
            && out.semitones.contains(&12)
            && out.semitones.contains(&-12),
        "ps-crystal auto should stack 0/±12, got {:?}",
        out.semitones
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
    assert!(is_factory_lead_id("ld-sine"));
}

#[test]
fn non_airy_fresh_pad_stays_single_note_on_auto() {
    let juno = load_factory("pf-juno-air").unwrap();
    let out = render_export("pf-juno-air", &juno, &export(LayerMode::Auto, 0.4)).unwrap();
    assert_eq!(out.semitones, vec![0], "got {:?}", out.semitones);
}

#[test]
fn pad_octave_voices_are_detuned_a_few_hz() {
    let preset = sine_patch();
    // Long enough that a Goertzel bin (~sr/n) is narrower than the 1–3 Hz offset.
    let secs = 3.2;
    let layered = render_export(
        "ps-detune-probe",
        &preset,
        &export(
            LayerMode::Explicit(vec![LayerInterval::Octave, LayerInterval::OctaveDown]),
            secs,
        ),
    )
    .unwrap();
    assert_eq!(layered.semitones, vec![-12, 0, 12]);

    let f0 = midi_to_hz(60);
    let win = late_body(&layered.samples);
    let sr = SR as f32;
    let bin_hz = sr / win.len() as f32;
    assert!(
        bin_hz < 2.0,
        "window too short to resolve a few Hz (bin={bin_hz})"
    );
    let e_up_exact = goertzel_power(&win, sr, (f0 * 2.0) as f32);
    let e_up_det = goertzel_power(&win, sr, (f0 * 2.0 + PAD_DETUNE_HZ_OCTAVE_UP) as f32);
    let e_dn_exact = goertzel_power(&win, sr, (f0 * 0.5) as f32);
    let e_dn_det = goertzel_power(&win, sr, (f0 * 0.5 + PAD_DETUNE_HZ_OCTAVE_DOWN) as f32);
    assert!(
        e_up_det > e_up_exact * 1.4,
        "octave-up should sit a few Hz sharp (exact={e_up_exact}, det={e_up_det}, bin={bin_hz})"
    );
    assert!(
        e_dn_det > e_dn_exact * 1.4,
        "octave-down should sit a few Hz flat (exact={e_dn_exact}, det={e_dn_det}, bin={bin_hz})"
    );
}

#[test]
fn thin_pad_auto_adds_fifth_and_two_octaves() {
    let preset = sine_patch();
    let out = render_export("ps-thin-probe", &preset, &export(LayerMode::Auto, 0.7)).unwrap();
    assert!(
        out.semitones.contains(&-12) && out.semitones.contains(&12),
        "thin pad auto must include ±12, got {:?}",
        out.semitones
    );
    assert!(
        out.semitones.contains(&7) && out.semitones.contains(&24),
        "thin sine pad should add fifth and +24, got {:?}",
        out.semitones
    );

    let f0 = out.frequency_hz;
    let win = late_body(&out.samples);
    let e15 = goertzel_power(
        &win,
        SR as f32,
        (f0 * semitones_to_ratio(7.0) + PAD_DETUNE_HZ_FIFTH) as f32,
    );
    let e4 = goertzel_power(&win, SR as f32, (f0 * 4.0 + PAD_DETUNE_HZ_OCTAVE2) as f32);
    let e0 = goertzel_power(&win, SR as f32, f0 as f32);
    assert!(
        e15 > e0 * 0.04,
        "auto fifth should add ~1.5×f0 (e0={e0}, e15={e15})"
    );
    assert!(
        e4 > e0 * 0.02,
        "auto +24 should add ~4×f0 (e0={e0}, e4={e4})"
    );
}

#[test]
fn pad_fifth_and_two_oct_are_mid_bandpassed() {
    let preset = sine_patch();
    let layered = render_export(
        "ps-bp-probe",
        &preset,
        &export(
            LayerMode::Explicit(vec![LayerInterval::Fifth, LayerInterval::Octave2]),
            0.5,
        ),
    )
    .unwrap();
    let f0 = midi_to_hz(60);
    // Fifth of C4 ≈ 392 Hz (inside 200–2k2). Two octaves ≈ 1047 Hz (inside).
    // A 65 Hz component is not in this mix; check that energy below the BP
    // floor is weak relative to the mid fifth.
    let win = late_body(&layered.samples);
    let mid = goertzel_power(
        &win,
        SR as f32,
        (f0 * semitones_to_ratio(7.0) + PAD_DETUNE_HZ_FIFTH) as f32,
    );
    let low = goertzel_power(&win, SR as f32, 80.0);
    assert!(mid > 0.0, "mid fifth must survive the BP");
    assert!(
        mid > low * 8.0,
        "mid BP should keep fifth energy above the sub floor (mid={mid}, low={low})"
    );
    assert!(PAD_MID_BP_LO_HZ < 250.0 && PAD_MID_BP_HI_HZ > 1_800.0);
}

#[test]
fn factory_pads_keep_pitch_and_hold_through_default() {
    for id in ["ps-crystal", "ps-shimmer", "pf-flute-pad", "pf-silk"] {
        let preset = load_factory(id).unwrap();
        assert!(
            preset.operators[0].attack >= 1.2,
            "{id} wind-body attack {} should bloom for seconds",
            preset.operators[0].attack
        );
        assert!(
            matches!(
                preset.filter.kind,
                fm_synth::FilterType::Highpass | fm_synth::FilterType::Bandpass
            ),
            "{id} must keep HP/BP so pads do not steal kick/sub"
        );
        assert!(preset.filter.cutoff >= 140.0, "{id} HP cutoff too low");
        assert!(
            preset.lfo.depth_cents.abs() < 1e-9,
            "{id} pitch LFO depth {} must be 0 (うねり is static ±oct Hz, not a wobbling root)",
            preset.lfo.depth_cents
        );

        let (buf, analysis) = analyze_preset(
            id,
            &preset,
            &export(LayerMode::Auto, 2.4),
            Some("wind+sparkle pad"),
        )
        .unwrap();
        assert!(buf.iter().all(|s| s.is_finite()), "{id} NaN");
        assert!(
            analysis.report.render_layers.iter().any(|s| s == "octave")
                && analysis
                    .report
                    .render_layers
                    .iter()
                    .any(|s| s == "octave-down"),
            "{id} layers {:?}",
            analysis.report.render_layers
        );
        assert!(
            analysis.report.band_energy.sub_20_80 < 0.08,
            "{id} sub {} too hot for a pad",
            analysis.report.band_energy.sub_20_80
        );
        assert!(
            analysis.report.energy_at_frac.t85 > 0.02,
            "{id} died before t=0.85 ({})",
            analysis.report.energy_at_frac.t85
        );

        let f0 = midi_to_hz(preset.default_note) as f32;
        let win = late_body(&buf);
        let e0 = goertzel_power(&win, SR as f32, f0);
        let e_up = goertzel_power(&win, SR as f32, f0 * 2.0 + PAD_DETUNE_HZ_OCTAVE_UP as f32);
        let e_dn = goertzel_power(&win, SR as f32, f0 * 0.5 + PAD_DETUNE_HZ_OCTAVE_DOWN as f32);
        assert!(
            e0 > 0.0 && (e_up > 0.0 || e_dn > 0.0),
            "{id} missing pitch ridges (e0={e0}, up={e_up}, dn={e_dn})"
        );
    }
}

#[test]
fn layers_off_pad_matches_single_render() {
    let preset = load_factory("ps-crystal").unwrap();
    let single = render(
        &preset,
        &fm_synth::RenderParams {
            frequency_hz: midi_to_hz(preset.default_note),
            duration_secs: 0.8,
            velocity: 0.9,
            sample_rate: SR,
        },
    )
    .unwrap();
    let off = render_export("ps-crystal", &preset, &export(LayerMode::Off, 0.8)).unwrap();
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
fn pad_stack_presets_have_no_pitch_lfo() {
    for id in factory_ids() {
        if !is_factory_pad_stack_id(id) {
            continue;
        }
        let p = load_factory(id).unwrap();
        assert!(
            p.lfo.depth_cents.abs() < 1e-9,
            "{id} lfo.depth_cents {} must be 0 so the root does not wobble",
            p.lfo.depth_cents
        );
    }
}

#[test]
fn root_ridge_stays_flat_beating_is_from_static_octaves() {
    // Single-note: Goertzel at f0 stays stronger than f0+3 Hz early and late
    // (the autocorr tracker can octave-jump on bright pads; that is not LFO).
    for id in ["ps-crystal", "ps-shimmer", "pf-flute-pad"] {
        let preset = load_factory(id).unwrap();
        let off = render_export(id, &preset, &export(LayerMode::Off, 3.0)).unwrap();
        let f0 = midi_to_hz(preset.default_note) as f32;
        let n = off.samples.len();
        let early = hann(&off.samples[n / 5..n * 2 / 5]);
        let late = hann(&off.samples[n * 3 / 5..n * 4 / 5]);
        let sr = SR as f32;
        for (label, win) in [("early", &early), ("late", &late)] {
            let e0 = goertzel_power(win, sr, f0);
            let e_wobble = goertzel_power(win, sr, f0 + 3.0);
            assert!(
                e0 > e_wobble * 3.0,
                "{id} {label} Off root wandered (e0={e0}, f0+3={e_wobble})"
            );
        }
    }

    // Layered sine: root Goertzel stays on f0 early and late; ±oct sit 2 Hz off.
    let preset = sine_patch();
    let layered = render_export(
        "ps-beat-probe",
        &preset,
        &export(
            LayerMode::Explicit(vec![LayerInterval::Octave, LayerInterval::OctaveDown]),
            3.0,
        ),
    )
    .unwrap();
    let f0 = midi_to_hz(60) as f32;
    let n = layered.samples.len();
    let early = hann(&layered.samples[n / 8..n / 3]);
    let late = hann(&layered.samples[n * 2 / 3..n * 11 / 12]);
    let sr = SR as f32;
    for (label, win) in [("early", &early), ("late", &late)] {
        let e0 = goertzel_power(win, sr, f0);
        let e_wobble = goertzel_power(win, sr, f0 + 3.0);
        assert!(
            e0 > e_wobble * 4.0,
            "{label} root must stay on f0, not wander +3 Hz (e0={e0}, wobble={e_wobble})"
        );
        let e_up = goertzel_power(win, sr, f0 * 2.0 + PAD_DETUNE_HZ_OCTAVE_UP as f32);
        let e_up_exact = goertzel_power(win, sr, f0 * 2.0);
        assert!(
            e_up > e_up_exact * 1.2,
            "{label} octave-up must be a static +2 Hz offset (det={e_up}, exact={e_up_exact})"
        );
    }
    assert!(
        (PAD_DETUNE_HZ_OCTAVE_UP.abs() - 2.0).abs() < 1e-9
            && (PAD_DETUNE_HZ_OCTAVE_DOWN + 2.0).abs() < 1e-9,
        "±octave offsets must stay in the 1–3 Hz static band"
    );
}
