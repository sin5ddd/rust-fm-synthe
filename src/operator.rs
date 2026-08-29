use crate::adsr::{Adsr, AdsrParams};
use crate::midi::cents_to_ratio;
use serde::Deserialize;
use std::f64::consts::TAU;

const TWO_PI: f32 = std::f32::consts::TAU;

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
}

impl Waveform {
    pub fn evaluate(self, phase: f64) -> f32 {
        let s = phase.sin() as f32;
        match self {
            Waveform::Sine => s,
            Waveform::HalfSine => s.max(0.0),
            Waveform::AbsSine => s.abs(),
            Waveform::Pulse => {
                if s >= 0.0 {
                    1.0
                } else {
                    -1.0
                }
            }
        }
    }
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
}

/// Runtime operator: phase accumulator + envelope.
#[derive(Clone, Debug)]
pub struct Operator {
    params: OperatorParams,
    env: Adsr,
    phase: f64,
    phase_inc: f64,
    last: f32,
    prev: f32,
    sample_rate: f64,
    vel_amp: f32,
}

impl Operator {
    pub fn new(params: OperatorParams, sample_rate: f32) -> Self {
        let env = Adsr::new(params.adsr(), sample_rate);
        Self {
            params,
            env,
            phase: 0.0,
            phase_inc: 0.0,
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
        self.phase = 0.0;
        self.last = 0.0;
        self.prev = 0.0;
        self.env.note_on();
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
        let freq = match self.params.freq_mode {
            FreqMode::Ratio => {
                note_hz * self.params.ratio * cents_to_ratio(self.params.detune_cents) * pitch_mult
            }
            FreqMode::Fixed => self.params.fixed_hz * cents_to_ratio(self.params.detune_cents),
        };
        let nyquist = self.sample_rate * 0.49;
        let freq = freq.clamp(0.0, nyquist);
        self.phase_inc = TAU * freq / self.sample_rate;
    }

    /// `modulation` is an audio-rate signal in roughly [-1, 1]; converted to radians.
    pub fn tick(&mut self, modulation: f32) -> f32 {
        let env = self.env.tick();
        let phase = self.phase + f64::from(modulation * TWO_PI);
        let s = self.params.waveform.evaluate(phase);
        self.prev = self.last;
        self.last = s * env * self.params.level * self.vel_amp;
        self.phase = (self.phase + self.phase_inc) % TAU;
        self.last
    }
}
