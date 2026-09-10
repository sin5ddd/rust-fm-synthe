//! Render-time multi-note mix: the same 4OP patch at N, N+12, optionally N+7.
//!
//! This is **not** `[fx.chorus] intervals` (a delayed pitch-shift of the mono
//! bus) and **not** reallocating operators inside a preset.

use crate::error::{Error, Result};
use crate::midi::semitones_to_ratio;
use crate::preset::Preset;
use crate::render::{normalize_loudness, render, RenderParams, TARGET_PEAK, TARGET_RMS};
use crate::resolve_frequency;
use crate::ExportParams;
use serde::Deserialize;
use std::path::Path;

/// Mix gain for the unison (root) voice.
pub const LAYER_GAIN_ROOT: f32 = 1.0;
/// Octave-up voice, slightly quieter than the root.
pub const LAYER_GAIN_OCTAVE: f32 = 0.72;
/// Perfect-fifth voice, quieter still.
pub const LAYER_GAIN_FIFTH: f32 = 0.48;

/// One extra full-patch voice, expressed as a musical interval above the root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayerInterval {
    /// +12 semitones (2× frequency). Also accepts `octave-up` in TOML/CLI.
    #[serde(alias = "octave-up")]
    Octave,
    /// +7 semitones (perfect fifth).
    Fifth,
}

impl LayerInterval {
    pub fn semitones(self) -> i16 {
        match self {
            Self::Octave => 12,
            Self::Fifth => 7,
        }
    }

    pub fn gain(self) -> f32 {
        match self {
            Self::Octave => LAYER_GAIN_OCTAVE,
            Self::Fifth => LAYER_GAIN_FIFTH,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Octave => "octave",
            Self::Fifth => "fifth",
        }
    }

    pub fn parse_token(raw: &str) -> Result<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "octave" | "octave-up" => Ok(Self::Octave),
            "fifth" => Ok(Self::Fifth),
            other => Err(Error::InvalidParam {
                message: format!(
                    "unknown layer `{other}` (use auto, none, octave, fifth, octave,fifth)"
                ),
            }),
        }
    }
}

/// How `render` / `analyze` choose extra full-patch voices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerMode {
    /// Factory leads (`ld-*` / `lead-*` / ld-folder extras): octave, plus fifth
    /// when the octave mix still looks thin. Other banks stay single-note unless
    /// the preset sets `render_layers`.
    Auto,
    /// Single-note render (the historical `render()` behavior).
    Off,
    /// Always mix these intervals on top of the unison root.
    Explicit(Vec<LayerInterval>),
}

impl Default for LayerMode {
    fn default() -> Self {
        Self::Auto
    }
}

impl LayerMode {
    /// Parse CLI `--layers` values: `auto`, `none`, `octave`, `octave,fifth`.
    pub fn parse(spec: &str) -> Result<Self> {
        let spec = spec.trim();
        if spec.is_empty() || spec.eq_ignore_ascii_case("auto") {
            return Ok(Self::Auto);
        }
        if spec.eq_ignore_ascii_case("none") || spec.eq_ignore_ascii_case("off") {
            return Ok(Self::Off);
        }

        let mut intervals = Vec::new();
        for part in spec.split(',') {
            let part = part.trim();
            if part.is_empty()
                || part.eq_ignore_ascii_case("unison")
                || part.eq_ignore_ascii_case("root")
            {
                continue;
            }
            intervals.push(LayerInterval::parse_token(part)?);
        }
        intervals.sort();
        intervals.dedup();
        if intervals.is_empty() {
            Ok(Self::Off)
        } else {
            Ok(Self::Explicit(intervals))
        }
    }
}

/// One completed export buffer plus the semitone offsets that were mixed.
#[derive(Clone, Debug)]
pub struct LayeredRender {
    pub samples: Vec<f32>,
    pub frequency_hz: f64,
    pub duration_secs: f64,
    /// Semitone offsets actually mixed, always including `0` (unison).
    pub semitones: Vec<i16>,
}

impl LayeredRender {
    pub fn layer_labels(&self) -> Vec<String> {
        self.semitones
            .iter()
            .map(|&st| match st {
                0 => "unison".to_string(),
                12 => "octave".to_string(),
                7 => "fifth".to_string(),
                other => format!("{other:+}"),
            })
            .collect()
    }
}

/// Factory lead bank: `ld-*`, `lead-*`, and the older shots that live in `presets/ld/`.
pub fn is_factory_lead_id(id: &str) -> bool {
    let stem = Path::new(id)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(id);
    let stem = stem.rsplit('/').next().unwrap_or(stem);
    stem.starts_with("ld-")
        || stem.starts_with("lead-")
        || matches!(
            stem,
            "stab-fm-fifth" | "stab-fm-major" | "filter-pluck" | "stab-pluck"
        )
}

/// CLI `--layers` wins when it is not `auto`. Otherwise the preset field, then
/// the factory-lead default (octave, maybe fifth).
pub fn resolve_layer_plan(
    preset_id: &str,
    preset: &Preset,
    mode: &LayerMode,
) -> (Vec<LayerInterval>, bool) {
    match mode {
        LayerMode::Off => (Vec::new(), false),
        LayerMode::Explicit(intervals) => (intervals.clone(), false),
        LayerMode::Auto => {
            if let Some(ref intervals) = preset.render_layers {
                (intervals.clone(), false)
            } else if is_factory_lead_id(preset_id) {
                (vec![LayerInterval::Octave], true)
            } else {
                (Vec::new(), false)
            }
        }
    }
}

/// Render the same 4OP patch at the root plus any requested intervals and mix.
pub fn render_with_layers(
    preset_id: &str,
    preset: &Preset,
    params: &RenderParams,
    mode: &LayerMode,
) -> Result<LayeredRender> {
    let (intervals, maybe_fifth) = resolve_layer_plan(preset_id, preset, mode);
    let root = render(preset, params)?;
    let mut parts: Vec<(i16, Vec<f32>, f32)> = vec![(0, root, LAYER_GAIN_ROOT)];

    if intervals.iter().any(|i| *i == LayerInterval::Octave) {
        parts.push(render_offset(preset, params, LayerInterval::Octave)?);
    }
    let fifth_forced = intervals.iter().any(|i| *i == LayerInterval::Fifth);
    if fifth_forced {
        parts.push(render_offset(preset, params, LayerInterval::Fifth)?);
    }

    if parts.len() == 1 && !maybe_fifth {
        return Ok(LayeredRender {
            samples: parts.remove(0).1,
            frequency_hz: params.frequency_hz,
            duration_secs: params.duration_secs,
            semitones: vec![0],
        });
    }

    let mut samples = mix_gain_parts(&parts);
    let mut semitones: Vec<i16> = parts.iter().map(|(st, _, _)| *st).collect();

    if maybe_fifth
        && !fifth_forced
        && stack_looks_thin(&samples, params.sample_rate, params.frequency_hz)
    {
        parts.push(render_offset(preset, params, LayerInterval::Fifth)?);
        samples = mix_gain_parts(&parts);
        semitones = parts.iter().map(|(st, _, _)| *st).collect();
    }

    semitones.sort();
    Ok(LayeredRender {
        samples,
        frequency_hz: params.frequency_hz,
        duration_secs: params.duration_secs,
        semitones,
    })
}

/// Resolve pitch/duration from [`ExportParams`] and render with layer policy.
pub fn render_export(
    preset_id: &str,
    preset: &Preset,
    export: &ExportParams,
) -> Result<LayeredRender> {
    let frequency_hz = resolve_frequency(preset, export.note, export.hz)?;
    let duration_secs = export.duration.unwrap_or(preset.default_duration);
    let params = RenderParams {
        frequency_hz,
        duration_secs,
        velocity: export.velocity,
        sample_rate: export.sample_rate,
    };
    let mut layered = render_with_layers(preset_id, preset, &params, &export.layers)?;
    layered.frequency_hz = frequency_hz;
    layered.duration_secs = duration_secs;
    Ok(layered)
}

fn render_offset(
    preset: &Preset,
    params: &RenderParams,
    interval: LayerInterval,
) -> Result<(i16, Vec<f32>, f32)> {
    let shifted = RenderParams {
        frequency_hz: params.frequency_hz * semitones_to_ratio(f64::from(interval.semitones())),
        ..params.clone()
    };
    Ok((
        interval.semitones(),
        render(preset, &shifted)?,
        interval.gain(),
    ))
}

fn mix_gain_parts(parts: &[(i16, Vec<f32>, f32)]) -> Vec<f32> {
    let n = parts.iter().map(|(_, b, _)| b.len()).max().unwrap_or(0);
    let mut out = vec![0.0f32; n];
    for (_, buf, gain) in parts {
        for (i, &x) in buf.iter().enumerate() {
            out[i] += x * gain;
        }
    }
    normalize_loudness(&mut out, TARGET_RMS, TARGET_PEAK);
    out
}

/// Sustained body looks like one or two sine ridges (root ± octave) with little
/// mid/odd-harmonic presence — the `fm-synth analyze` "thin lead" case.
pub fn stack_looks_thin(buf: &[f32], sample_rate: u32, f0: f64) -> bool {
    if buf.len() < 256 || !f0.is_finite() || f0 <= 0.0 {
        return false;
    }
    let start = buf.len() / 10;
    let end = (buf.len() * 4 / 10).max(start + 256).min(buf.len());
    if end <= start + 64 {
        return false;
    }
    let windowed = hann_window(&buf[start..end]);
    let sr = sample_rate as f32;
    let e0 = goertzel_power(&windowed, sr, f0 as f32);
    let e15 = goertzel_power(&windowed, sr, (f0 * semitones_to_ratio(7.0)) as f32);
    let e2 = goertzel_power(&windowed, sr, (f0 * 2.0) as f32);
    let e3 = goertzel_power(&windowed, sr, (f0 * 3.0) as f32);
    let e4 = goertzel_power(&windowed, sr, (f0 * 4.0) as f32);
    let e5 = goertzel_power(&windowed, sr, (f0 * 5.0) as f32);
    if e0 + e2 < 1e-18 {
        return false;
    }
    // Compare upper harmonics to the *root* so adding a 2× voice cannot
    // hide an already-bright spectrum (chip / supersaw).
    let extra = e3 + e4 + e5;
    let harmonic_thin = extra / e0.max(1e-18) < 0.22 && e15 / (e0 + e2) < 0.22;

    let bass = band_goertzel(&windowed, sr, 80.0, 250.0, 8.0);
    let mid = band_goertzel(&windowed, sr, 250.0, 2_000.0, 16.0);
    let high = band_goertzel(&windowed, sr, 2_000.0, (sr * 0.45).min(8_000.0), 40.0);
    let tot = bass + mid + high;
    let mid_weak = tot > 0.0 && (mid + high) / tot < 0.55;

    harmonic_thin && mid_weak
}

fn band_goertzel(buf: &[f32], sr: f32, lo: f32, hi: f32, step: f32) -> f64 {
    let mut e = 0.0;
    let mut f = lo;
    while f < hi && f < sr * 0.45 {
        e += goertzel_power(buf, sr, f);
        f += step;
    }
    e
}

fn hann_window(buf: &[f32]) -> Vec<f32> {
    let n = buf.len() as f32;
    buf.iter()
        .enumerate()
        .map(|(i, &x)| {
            let w = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / (n - 1.0)).cos();
            x * w
        })
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_layer_specs() {
        assert_eq!(LayerMode::parse("auto").unwrap(), LayerMode::Auto);
        assert_eq!(LayerMode::parse("none").unwrap(), LayerMode::Off);
        assert_eq!(
            LayerMode::parse("octave").unwrap(),
            LayerMode::Explicit(vec![LayerInterval::Octave])
        );
        assert_eq!(
            LayerMode::parse("octave,fifth").unwrap(),
            LayerMode::Explicit(vec![LayerInterval::Octave, LayerInterval::Fifth])
        );
        assert_eq!(
            LayerMode::parse("octave-up, fifth").unwrap(),
            LayerMode::Explicit(vec![LayerInterval::Octave, LayerInterval::Fifth])
        );
        assert!(LayerMode::parse("chorus").is_err());
    }

    #[test]
    fn lead_id_detection() {
        assert!(is_factory_lead_id("ld-sine"));
        assert!(is_factory_lead_id("lead-fm-pluck"));
        assert!(is_factory_lead_id("stab-fm-fifth"));
        assert!(is_factory_lead_id("presets/ld/ld-chip.toml"));
        assert!(!is_factory_lead_id("sub-bass"));
        assert!(!is_factory_lead_id("bd-808-boom"));
        assert!(!is_factory_lead_id("bs-wobble"));
    }

    #[test]
    fn two_sines_look_thin_saw_does_not() {
        let sr = 22_050u32;
        let n = (sr as usize) / 2;
        let f0 = 130.81f32;
        let mut sine_oct = vec![0.0f32; n];
        let mut saw = vec![0.0f32; n];
        let mut pulse_c4 = vec![0.0f32; n];
        for i in 0..n {
            let t = i as f32 / sr as f32;
            sine_oct[i] = (std::f32::consts::TAU * f0 * t).sin()
                + 0.72 * (std::f32::consts::TAU * f0 * 2.0 * t).sin();
            let phase = (f0 * t).fract();
            saw[i] = 2.0 * phase - 1.0;
            let c4 = 261.63f32;
            pulse_c4[i] = if (std::f32::consts::TAU * c4 * t).sin() >= 0.0 {
                0.7
            } else {
                -0.7
            };
        }
        assert!(stack_looks_thin(&sine_oct, sr, f64::from(f0)));
        assert!(!stack_looks_thin(&saw, sr, f64::from(f0)));
        assert!(!stack_looks_thin(&pulse_c4, sr, 261.63));
    }
}
