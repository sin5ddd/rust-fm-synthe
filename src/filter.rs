use crate::adsr::AdsrParams;
use serde::Deserialize;

/// Voice-level filter mode. One SVF, three taps (LP / BP / HP).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FilterType {
    #[default]
    Lowpass,
    Bandpass,
    Highpass,
}

/// Static filter + cutoff-ADSR patch data. All fields default so old TOML still parses.
#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct FilterParams {
    #[serde(rename = "type")]
    pub kind: FilterType,
    /// Base cutoff in Hz (before envelope).
    pub cutoff: f32,
    /// 0 = gentle (Q ≈ 0.7), 1 = high resonance. Clamped internally so it cannot scream.
    pub resonance: f32,
    /// Cutoff envelope depth in octaves. 0 = static cutoff. Negative closes the filter.
    pub env_amount: f32,
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Default for FilterParams {
    fn default() -> Self {
        Self {
            kind: FilterType::Lowpass,
            cutoff: 18_000.0,
            resonance: 0.0,
            env_amount: 0.0,
            attack: 0.0,
            decay: 0.0,
            sustain: 1.0,
            release: 0.05,
        }
    }
}

impl FilterParams {
    pub fn adsr(&self) -> AdsrParams {
        AdsrParams {
            attack: self.attack.max(0.0),
            decay: self.decay.max(0.0),
            sustain: self.sustain.clamp(0.0, 1.0),
            release: self.release.max(0.0),
        }
    }
}

/// Topology-preserving transform SVF (Cytomic / Zavalishin).
///
/// One state pair yields LP, BP, and HP. Coefficients are rebuilt each sample
/// so cutoff can be enveloped without zippering.
#[derive(Clone, Debug)]
pub struct Svf {
    sample_rate: f32,
    ic1eq: f32,
    ic2eq: f32,
}

impl Svf {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate: sample_rate.max(1.0),
            ic1eq: 0.0,
            ic2eq: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.ic1eq = 0.0;
        self.ic2eq = 0.0;
    }

    pub fn tick(&mut self, input: f32, cutoff_hz: f32, resonance: f32, kind: FilterType) -> f32 {
        let sr = self.sample_rate;
        let nyquist = sr * 0.49;
        let fc = cutoff_hz.clamp(20.0, nyquist);
        let q = resonance_to_q(resonance);
        // tan(π f/sr) is stable well below Nyquist; clamp g so a bad cutoff cannot explode.
        let g = (std::f32::consts::PI * fc / sr).tan().clamp(0.0, 8.0);
        let k = 1.0 / q;
        let a1 = 1.0 / (1.0 + g * (g + k));
        let a2 = g * a1;
        let a3 = g * a2;

        let v3 = input - self.ic2eq;
        let v1 = a1 * self.ic1eq + a2 * v3;
        let v2 = self.ic2eq + a2 * self.ic1eq + a3 * v3;
        self.ic1eq = 2.0 * v1 - self.ic1eq;
        self.ic2eq = 2.0 * v2 - self.ic2eq;

        if !self.ic1eq.is_finite() || !self.ic2eq.is_finite() {
            self.reset();
            return 0.0;
        }

        let y = match kind {
            FilterType::Lowpass => v2,
            // k*v1 ≈ unity peak at the cutoff (otherwise BP gets louder with Q).
            FilterType::Bandpass => v1 * k,
            FilterType::Highpass => input - k * v1 - v2,
        };
        if y.is_finite() {
            y
        } else {
            self.reset();
            0.0
        }
    }
}

/// Map a 0–1 patch knob to Q. Stays well below self-oscillation.
fn resonance_to_q(resonance: f32) -> f32 {
    let r = resonance.clamp(0.0, 1.0);
    (0.707 + r * 11.0).clamp(0.5, 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, sr: f32, n: usize) -> Vec<f32> {
        let w = std::f32::consts::TAU * freq / sr;
        (0..n).map(|i| (w * i as f32).sin()).collect()
    }

    fn rms(buf: &[f32]) -> f32 {
        if buf.is_empty() {
            return 0.0;
        }
        let s: f32 = buf.iter().map(|x| x * x).sum();
        (s / buf.len() as f32).sqrt()
    }

    fn run(kind: FilterType, cutoff: f32, res: f32, input: &[f32], sr: f32) -> Vec<f32> {
        let mut f = Svf::new(sr);
        input
            .iter()
            .map(|&x| f.tick(x, cutoff, res, kind))
            .collect()
    }

    #[test]
    fn lowpass_kills_high_sine() {
        let sr = 44_100.0;
        let n = 8_192;
        let high = sine(8_000.0, sr, n);
        let closed = run(FilterType::Lowpass, 250.0, 0.05, &high, sr);
        let open = run(FilterType::Lowpass, 16_000.0, 0.05, &high, sr);
        let rc = rms(&closed[2_000..]);
        let ro = rms(&open[2_000..]);
        assert!(
            rc < ro * 0.12,
            "lowpass did not attenuate 8 kHz (closed={rc}, open={ro})"
        );
    }

    #[test]
    fn highpass_kills_low_sine() {
        let sr = 44_100.0;
        let n = 8_192;
        let low = sine(80.0, sr, n);
        let closed = run(FilterType::Highpass, 2_500.0, 0.05, &low, sr);
        let open = run(FilterType::Highpass, 30.0, 0.05, &low, sr);
        let rc = rms(&closed[2_000..]);
        let ro = rms(&open[2_000..]);
        assert!(
            rc < ro * 0.12,
            "highpass did not attenuate 80 Hz (closed={rc}, open={ro})"
        );
    }

    #[test]
    fn bandpass_prefers_its_cutoff() {
        let sr = 44_100.0;
        let n = 8_192;
        let mid = sine(1_000.0, sr, n);
        let low = sine(80.0, sr, n);
        let y_mid = run(FilterType::Bandpass, 1_000.0, 0.35, &mid, sr);
        let y_low = run(FilterType::Bandpass, 1_000.0, 0.35, &low, sr);
        let rm = rms(&y_mid[2_000..]);
        let rl = rms(&y_low[2_000..]);
        assert!(
            rm > rl * 4.0,
            "bandpass at 1 kHz should pass 1 kHz much more than 80 Hz (mid={rm}, low={rl})"
        );
    }

    #[test]
    fn high_resonance_stays_finite() {
        let mut f = Svf::new(44_100.0);
        let mut x = 1.0f32;
        for i in 0..8_000 {
            let cutoff = 80.0 + (i as f32) * 2.0;
            let y = f.tick(x, cutoff, 1.0, FilterType::Lowpass);
            assert!(y.is_finite(), "NaN/Inf at sample {i}");
            x = if i % 64 == 0 { 1.0 } else { 0.0 };
        }
        let y_bp = f.tick(1.0, 400.0, 1.0, FilterType::Bandpass);
        let y_hp = f.tick(1.0, 400.0, 1.0, FilterType::Highpass);
        assert!(y_bp.is_finite() && y_hp.is_finite());
    }
}
