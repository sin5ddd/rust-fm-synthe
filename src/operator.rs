use crate::adsr::{Adsr, AdsrParams};
use crate::midi::cents_to_ratio;
use serde::Deserialize;
use std::f64::consts::TAU;

const TWO_PI: f32 = std::f32::consts::TAU;

/// Hard cap so a typo in TOML cannot spawn hundreds of saws per operator.
const MAX_UNISON: usize = 16;

/// JP-8000-ish relative detune for the classic 7-voice supersaw, scaled to ±1.
const SUPERSAW7_CENTS: [f64; 7] = [-1.0000, -0.5716, -0.1775, 0.0, 0.1810, 0.5716, 0.9766];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Waveform {
    #[default]
    Sine,
    /// Positive half of a sine (TX81Z-ish). Adds DC; useful as a modulator.
    HalfSine,
    /// Full-wave rectified sine. Metallic / formant-ish.
    AbsSine,
    /// Soft square (sign of sine). Harsh, good for hits.
    Pulse,
    /// Single bandlimited saw (polyBLEP). Safe for bass; naive `phase/π-1` is not.
    Saw,
    /// Pseudo supersaw *inside one operator*: several detuned bandlimited saws.
    /// This is the expensive waveform (≈7 saws per op; 4 ops at once is heavy).
    SuperSaw,
}

impl Waveform {
    /// `phase_inc` is the per-sample phase step in radians (used by saw / super-saw).
    pub fn evaluate(self, phase: f64, phase_inc: f64) -> f32 {
        match self {
            Waveform::Sine => phase.sin() as f32,
            Waveform::HalfSine => (phase.sin() as f32).max(0.0),
            Waveform::AbsSine => (phase.sin() as f32).abs(),
            Waveform::Pulse => {
                if phase.sin() >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
            Waveform::Saw | Waveform::SuperSaw => polyblep_saw(phase, phase_inc),
        }
    }

    pub fn is_supersaw(self) -> bool {
        matches!(self, Waveform::SuperSaw)
    }
}

/// 2-point polyBLEP residual around a discontinuity at t=0 (t in [0, 1)).
fn poly_blep(t: f64, dt: f64) -> f64 {
    if dt <= 0.0 {
        return 0.0;
    }
    if t < dt {
        let x = t / dt;
        x + x - x * x - 1.0
    } else if t > 1.0 - dt {
        let x = (t - 1.0) / dt;
        x * x + x + x + 1.0
    } else {
        0.0
    }
}

/// Rising bipolar saw in [-1, 1], polyBLEP-corrected at the wrap.
pub(crate) fn polyblep_saw(phase: f64, phase_inc: f64) -> f32 {
    let t = {
        let x = phase / TAU;
        x - x.floor()
    };
    let dt = (phase_inc / TAU).abs().clamp(0.0, 0.5);
    let naive = 2.0 * t - 1.0;
    (naive - poly_blep(t, dt)) as f32
}

/// Naive saw used only in tests to prove the bandlimited one is actually different.
#[cfg(test)]
pub(crate) fn naive_saw(phase: f64) -> f32 {
    let t = {
        let x = phase / TAU;
        x - x.floor()
    };
    (2.0 * t - 1.0) as f32
}

fn default_unison() -> u8 {
    7
}

fn default_unison_detune() -> f64 {
    20.0
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FreqMode {
    #[default]
    Ratio,
    Fixed,
}

/// Static per-operator patch data.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct OperatorParams {
    /// Frequency ratio against the note (ignored in fixed mode).
    pub ratio: f64,
    pub detune_cents: f64,
    /// Output level 0–1+. When this op is a modulator, this is the index scale.
    pub level: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    /// 0 = ignore velocity, 1 = fully follow it.
    pub vel_sens: f32,
    pub waveform: Waveform,
    pub freq_mode: FreqMode,
    pub fixed_hz: f64,
    /// Super-saw voice count. Ignored for other waveforms. Default 7.
    #[serde(default = "default_unison")]
    pub unison: u8,
    /// Super-saw cents spread (outer voices ≈ ±this). Default 20.
    #[serde(default = "default_unison_detune")]
    pub unison_detune: f64,
}

impl Default for OperatorParams {
    fn default() -> Self {
        Self {
            ratio: 1.0,
            detune_cents: 0.0,
            level: 1.0,
            attack: 0.005,
            decay: 0.3,
            sustain: 0.0,
            release: 0.08,
            vel_sens: 0.35,
            waveform: Waveform::Sine,
            freq_mode: FreqMode::Ratio,
            fixed_hz: 440.0,
            unison: 7,
            unison_detune: 20.0,
        }
    }
}

impl OperatorParams {
    pub fn adsr(&self) -> AdsrParams {
        AdsrParams {
            attack: self.attack.max(0.0),
            decay: self.decay.max(0.0),
            sustain: self.sustain.clamp(0.0, 1.0),
            release: self.release.max(0.0),
        }
    }

    fn unison_voices(&self) -> usize {
        if self.waveform.is_supersaw() {
            usize::from(self.unison).clamp(1, MAX_UNISON)
        } else {
            1
        }
    }
}

/// Runtime operator: phase accumulator + envelope.
#[derive(Clone, Debug)]
pub struct Operator {
    params: OperatorParams,
    env: Adsr,
    phases: [f64; MAX_UNISON],
    phase_incs: [f64; MAX_UNISON],
    n_voices: usize,
    last: f32,
    prev: f32,
    sample_rate: f64,
    vel_amp: f32,
}

impl Operator {
    pub fn new(params: OperatorParams, sample_rate: f32) -> Self {
        let env = Adsr::new(params.adsr(), sample_rate);
        let n_voices = params.unison_voices();
        Self {
            params,
            env,
            phases: [0.0; MAX_UNISON],
            phase_incs: [0.0; MAX_UNISON],
            n_voices,
            last: 0.0,
            prev: 0.0,
            sample_rate: f64::from(sample_rate),
            vel_amp: 1.0,
        }
    }

    pub fn note_on(&mut self, velocity: f32) {
        let vel = velocity.clamp(0.0, 1.0);
        let s = self.params.vel_sens.clamp(0.0, 1.0);
        self.vel_amp = (1.0 - s) + s * vel;
        self.last = 0.0;
        self.prev = 0.0;
        self.env.note_on();
        // Spread initial phases so a supersaw does not start as one giant transient.
        let n = self.n_voices;
        for i in 0..n {
            self.phases[i] = if n <= 1 {
                0.0
            } else {
                TAU * (i as f64) / (n as f64)
            };
        }
    }

    pub fn note_off(&mut self) {
        self.env.note_off();
    }

    pub fn is_idle(&self) -> bool {
        self.env.is_idle()
    }

    pub fn release_secs(&self) -> f32 {
        self.env.release_secs()
    }

    pub fn sustain(&self) -> f32 {
        self.env.sustain()
    }

    /// DX-style 2-sample averaged feedback source.
    pub fn feedback_source(&self) -> f32 {
        0.5 * (self.last + self.prev)
    }

    pub fn update_frequency(&mut self, note_hz: f64, pitch_mult: f64) {
        let base = match self.params.freq_mode {
            FreqMode::Ratio => {
                note_hz * self.params.ratio * cents_to_ratio(self.params.detune_cents) * pitch_mult
            }
            FreqMode::Fixed => self.params.fixed_hz * cents_to_ratio(self.params.detune_cents),
        };
        let nyquist = self.sample_rate * 0.49;
        let n = self.n_voices;
        let spread = self.params.unison_detune;
        for i in 0..n {
            let cents = unison_cents(i, n, spread);
            let freq = (base * cents_to_ratio(cents)).clamp(0.0, nyquist);
            self.phase_incs[i] = TAU * freq / self.sample_rate;
        }
    }

    /// `modulation` is an audio-rate signal in roughly [-1, 1]; converted to radians.
    pub fn tick(&mut self, modulation: f32) -> f32 {
        let env = self.env.tick();
        let n = self.n_voices;
        let mut acc = 0.0f32;
        let mut wsum = 0.0f32;
        let mod_phase = f64::from(modulation * TWO_PI);
        for i in 0..n {
            let g = unison_gain(i, n);
            let phase = self.phases[i] + mod_phase;
            acc += self.params.waveform.evaluate(phase, self.phase_incs[i]) * g;
            wsum += g;
            self.phases[i] = (self.phases[i] + self.phase_incs[i]) % TAU;
        }
        let s = if wsum > 1e-6 {
            acc / wsum.sqrt().max(1.0)
        } else {
            0.0
        };
        self.prev = self.last;
        self.last = s * env * self.params.level * self.vel_amp;
        self.last
    }
}

fn unison_cents(i: usize, n: usize, spread: f64) -> f64 {
    if n <= 1 {
        return 0.0;
    }
    if n == 7 {
        return SUPERSAW7_CENTS[i] * spread;
    }
    let x = (i as f64) / (n as f64 - 1.0) * 2.0 - 1.0;
    if x.abs() < 1e-12 {
        0.0
    } else {
        x.signum() * x.abs().powf(1.3) * spread
    }
}

fn unison_gain(i: usize, n: usize) -> f32 {
    if n <= 1 {
        return 1.0;
    }
    let mid = (n - 1) as f32 * 0.5;
    if (i as f32 - mid).abs() < 0.51 {
        1.0
    } else {
        0.71
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_op(waveform: Waveform, hz: f64, sr: f32, n: usize) -> Vec<f32> {
        let params = OperatorParams {
            waveform,
            ratio: 1.0,
            level: 1.0,
            attack: 0.0,
            decay: 0.0,
            sustain: 1.0,
            release: 0.0,
            vel_sens: 0.0,
            ..OperatorParams::default()
        };
        let mut op = Operator::new(params, sr);
        op.note_on(1.0);
        op.update_frequency(hz, 1.0);
        (0..n).map(|_| op.tick(0.0)).collect()
    }

    fn hf_energy(buf: &[f32]) -> f32 {
        let mut s = 0.0f32;
        for w in buf.windows(2) {
            let d = w[1] - w[0];
            s += d * d;
        }
        (s / buf.len() as f32).sqrt()
    }

    #[test]
    fn polyblep_saw_has_less_hf_than_naive_at_bass() {
        let sr = 22_050.0f32;
        let hz = 55.0f64;
        let n = 4_096usize;
        let blep = {
            let mut phase = 0.0f64;
            let inc = TAU * hz / f64::from(sr);
            (0..n)
                .map(|_| {
                    let s = polyblep_saw(phase, inc);
                    phase = (phase + inc) % TAU;
                    s
                })
                .collect::<Vec<_>>()
        };
        let naive = {
            let mut phase = 0.0f64;
            let inc = TAU * hz / f64::from(sr);
            (0..n)
                .map(|_| {
                    let s = naive_saw(phase);
                    phase = (phase + inc) % TAU;
                    s
                })
                .collect::<Vec<_>>()
        };
        let hf_b = hf_energy(&blep);
        let hf_n = hf_energy(&naive);
        assert!(
            hf_b < hf_n,
            "polyBLEP should be darker near Nyquist than naive (blep={hf_b}, naive={hf_n})"
        );
        assert!(blep.iter().any(|&x| x.abs() > 0.2), "saw is silent");
    }

    #[test]
    fn supersaw_is_not_a_sine() {
        let sr = 22_050.0;
        let n = 2_048;
        let saw = render_op(Waveform::SuperSaw, 110.0, sr, n);
        let sine = render_op(Waveform::Sine, 110.0, sr, n);
        let peak_s = saw.iter().fold(0.0f32, |a, &x| a.max(x.abs()));
        assert!(peak_s > 0.2, "super-saw silent (peak={peak_s})");
        let diff: f32 = saw.iter().zip(&sine).map(|(a, b)| (a - b).abs()).sum();
        assert!(
            diff > 50.0,
            "super-saw nearly identical to sine (diff={diff})"
        );
    }
}
