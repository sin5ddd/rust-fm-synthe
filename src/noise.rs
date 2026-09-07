use crate::adsr::{Adsr, AdsrParams};
use crate::filter::{FilterParams, FilterType, Svf};
use serde::Deserialize;

/// Fixed seed so factory one-shots are bit-stable across renders.
const RNG_SEED: u32 = 0xA341_316C;

/// Makeup so Paul Kellet economy pink sits near white in peak.
const PINK_GAIN: f32 = 0.33;

/// Noise color. Not an operator waveform; a parallel source on the voice.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum NoiseColor {
    #[default]
    White,
    Pink,
    Brown,
}

fn default_noise_filter() -> FilterParams {
    FilterParams {
        kind: FilterType::Highpass,
        cutoff: 20.0,
        resonance: 0.0,
        env_amount: 0.0,
        attack: 0.0,
        decay: 0.0,
        sustain: 1.0,
        release: 0.05,
    }
}

/// Parallel noise patch. Omit the table or set `level = 0` to skip generation.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct NoiseParams {
    #[serde(rename = "type")]
    pub kind: NoiseColor,
    pub level: f32,
    /// Seconds of silence before the envelope starts. 0 = immediate.
    pub delay: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    /// 0 = ignore velocity, 1 = fully follow it.
    pub vel_sens: f32,
    #[serde(default = "default_noise_filter")]
    pub filter: FilterParams,
}

impl Default for NoiseParams {
    fn default() -> Self {
        Self {
            kind: NoiseColor::White,
            level: 0.0,
            delay: 0.0,
            attack: 0.0,
            decay: 0.05,
            sustain: 0.0,
            release: 0.02,
            vel_sens: 0.0,
            filter: default_noise_filter(),
        }
    }
}

impl NoiseParams {
    pub fn is_active(&self) -> bool {
        self.level.abs() > 1e-8
    }

    pub fn adsr(&self) -> AdsrParams {
        AdsrParams {
            attack: self.attack.max(0.0),
            decay: self.decay.max(0.0),
            sustain: self.sustain.clamp(0.0, 1.0),
            release: self.release.max(0.0),
        }
    }
}

/// Runtime noise: xorshift32 → color → dedicated SVF → amplitude ADSR.
pub struct NoiseSource {
    params: NoiseParams,
    active: bool,
    rng: u32,
    pink: [f32; 3],
    brown: f32,
    brown_pole: f32,
    env: Adsr,
    filter: Svf,
    filter_env: Adsr,
    vel_amp: f32,
    delay_left: u32,
    sample_rate: f32,
}

impl NoiseSource {
    pub fn new(params: NoiseParams, sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let active = params.is_active();
        let env = Adsr::new(params.adsr(), sr);
        let filter_env = Adsr::new(params.filter.adsr(), sr);
        // ~25 Hz one-pole: white through it is ~6 dB/oct (brown) above the pole.
        let brown_pole = (-std::f32::consts::TAU * 25.0 / sr).exp();
        Self {
            params,
            active,
            rng: RNG_SEED,
            pink: [0.0; 3],
            brown: 0.0,
            brown_pole,
            env,
            filter: Svf::new(sr),
            filter_env,
            vel_amp: 1.0,
            delay_left: 0,
            sample_rate: sr,
        }
    }

    pub fn note_on(&mut self, velocity: f32) {
        if !self.active {
            return;
        }
        let vel = velocity.clamp(0.0, 1.0);
        let s = self.params.vel_sens.clamp(0.0, 1.0);
        self.vel_amp = (1.0 - s) + s * vel;
        self.rng = RNG_SEED;
        self.pink = [0.0; 3];
        self.brown = 0.0;
        self.filter.reset();
        let delay = self.params.delay.clamp(0.0, 5.0);
        self.delay_left = (delay * self.sample_rate).round() as u32;
        if self.delay_left == 0 {
            self.env.note_on();
            self.filter_env.note_on();
        }
    }

    pub fn note_off(&mut self) {
        if !self.active {
            return;
        }
        if self.delay_left > 0 {
            self.delay_left = 0;
            return;
        }
        self.env.note_off();
        self.filter_env.note_off();
    }

    pub fn is_idle(&self) -> bool {
        if !self.active {
            return true;
        }
        self.delay_left == 0 && self.env.is_idle()
    }

    pub fn release_secs(&self) -> f32 {
        if self.active {
            self.env.release_secs()
        } else {
            0.0
        }
    }

    pub fn sustain(&self) -> f32 {
        if self.active {
            self.env.sustain()
        } else {
            0.0
        }
    }

    pub fn tick(&mut self) -> f32 {
        if !self.active {
            return 0.0;
        }
        if self.delay_left > 0 {
            self.delay_left -= 1;
            if self.delay_left == 0 {
                self.env.note_on();
                self.filter_env.note_on();
            } else {
                return 0.0;
            }
        }
        let env = self.env.tick();
        let fenv = self.filter_env.tick();
        if env <= 1e-8 {
            return 0.0;
        }
        let white = self.next_white();
        let raw = match self.params.kind {
            NoiseColor::White => white,
            NoiseColor::Pink => self.tick_pink(white),
            NoiseColor::Brown => self.tick_brown(white),
        };
        let cutoff = self.params.filter.cutoff * 2f32.powf(fenv * self.params.filter.env_amount);
        let filtered = self.filter.tick(
            raw,
            cutoff,
            self.params.filter.resonance,
            self.params.filter.kind,
        );
        filtered * env * self.params.level * self.vel_amp
    }

    fn next_white(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x as i32 as f32) * (1.0 / 2_147_483_648.0)
    }

    fn tick_pink(&mut self, white: f32) -> f32 {
        // Paul Kellet economy pink (musicdsp). Coefficients are for ~44.1 kHz.
        self.pink[0] = 0.99765 * self.pink[0] + white * 0.0990460;
        self.pink[1] = 0.96300 * self.pink[1] + white * 0.2965164;
        self.pink[2] = 0.57000 * self.pink[2] + white * 1.0526913;
        flush_denormal(&mut self.pink[0]);
        flush_denormal(&mut self.pink[1]);
        flush_denormal(&mut self.pink[2]);
        (self.pink[0] + self.pink[1] + self.pink[2] + white * 0.1848) * PINK_GAIN
    }

    fn tick_brown(&mut self, white: f32) -> f32 {
        let a = self.brown_pole;
        self.brown = a * self.brown + (1.0 - a) * white;
        flush_denormal(&mut self.brown);
        self.brown * 3.5
    }
}

fn flush_denormal(x: &mut f32) {
    if x.abs() < 1e-20 {
        *x = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hf_energy(buf: &[f32]) -> f32 {
        let mut s = 0.0f32;
        for w in buf.windows(2) {
            let d = w[1] - w[0];
            s += d * d;
        }
        (s / buf.len() as f32).sqrt()
    }

    fn rms(buf: &[f32]) -> f32 {
        if buf.is_empty() {
            return 0.0;
        }
        let s: f32 = buf.iter().map(|x| x * x).sum();
        (s / buf.len() as f32).sqrt()
    }

    fn render_noise(params: NoiseParams, sr: f32, n: usize) -> Vec<f32> {
        let mut src = NoiseSource::new(params, sr);
        src.note_on(1.0);
        (0..n).map(|_| src.tick()).collect()
    }

    fn color_params(kind: NoiseColor) -> NoiseParams {
        NoiseParams {
            kind,
            level: 1.0,
            attack: 0.0,
            decay: 0.0,
            sustain: 1.0,
            release: 0.0,
            vel_sens: 0.0,
            ..NoiseParams::default()
        }
    }

    #[test]
    fn omitted_noise_is_silent() {
        let buf = render_noise(NoiseParams::default(), 44_100.0, 2_048);
        assert!(buf.iter().all(|x| x.abs() < 1e-12), "level 0 leaked");
    }

    #[test]
    fn white_is_brighter_than_pink_than_brown() {
        let sr = 44_100.0;
        let n = 16_384;
        let skip = 2_048;
        let white = render_noise(color_params(NoiseColor::White), sr, n);
        let pink = render_noise(color_params(NoiseColor::Pink), sr, n);
        let brown = render_noise(color_params(NoiseColor::Brown), sr, n);
        let hw = hf_energy(&white[skip..]);
        let hp = hf_energy(&pink[skip..]);
        let hb = hf_energy(&brown[skip..]);
        assert!(rms(&white[skip..]) > 0.05, "white silent");
        assert!(rms(&pink[skip..]) > 0.02, "pink silent");
        assert!(rms(&brown[skip..]) > 0.02, "brown silent");
        assert!(
            hw > hp * 1.15,
            "white should be brighter than pink (w={hw}, p={hp})"
        );
        assert!(
            hp > hb * 1.15,
            "pink should be brighter than brown (p={hp}, b={hb})"
        );
    }

    #[test]
    fn delay_is_silent_then_speaks() {
        let sr = 1000.0f32;
        let params = NoiseParams {
            kind: NoiseColor::White,
            level: 1.0,
            delay: 0.02,
            attack: 0.0,
            decay: 0.0,
            sustain: 1.0,
            release: 0.0,
            vel_sens: 0.0,
            ..NoiseParams::default()
        };
        let mut src = NoiseSource::new(params, sr);
        src.note_on(1.0);
        let early: Vec<f32> = (0..15).map(|_| src.tick()).collect();
        assert!(
            early.iter().all(|x| x.abs() < 1e-9),
            "delay leaked ({early:?})"
        );
        let later: Vec<f32> = (0..30).map(|_| src.tick()).collect();
        assert!(
            later.iter().any(|x| x.abs() > 0.01),
            "delayed noise stayed silent"
        );
    }

    #[test]
    fn hp_is_brighter_than_lp_on_white() {
        let sr = 44_100.0;
        let n = 8_192;
        let skip = 2_000;
        let mut hp = color_params(NoiseColor::White);
        hp.filter.kind = FilterType::Highpass;
        hp.filter.cutoff = 5_000.0;
        let mut lp = color_params(NoiseColor::White);
        lp.filter.kind = FilterType::Lowpass;
        lp.filter.cutoff = 400.0;
        let bright = render_noise(hp, sr, n);
        let dark = render_noise(lp, sr, n);
        let hb = hf_energy(&bright[skip..]);
        let hd = hf_energy(&dark[skip..]);
        assert!(
            hb > hd * 2.0,
            "HP 5 kHz should be brighter than LP 400 Hz (hp={hb}, lp={hd})"
        );
    }

    #[test]
    fn bandpass_and_notch_stay_finite() {
        let sr = 44_100.0;
        let mut bp = color_params(NoiseColor::White);
        bp.filter.kind = FilterType::Bandpass;
        bp.filter.cutoff = 1_000.0;
        bp.filter.resonance = 0.8;
        let mut notch = color_params(NoiseColor::Pink);
        notch.filter.kind = FilterType::Notch;
        notch.filter.cutoff = 2_000.0;
        notch.filter.resonance = 0.6;
        for params in [bp, notch] {
            let buf = render_noise(params, sr, 4_096);
            assert!(buf.iter().all(|x| x.is_finite()), "NaN/Inf in noise filter");
            assert!(rms(&buf[512..]) > 0.005, "filtered noise silent");
        }
    }
}
