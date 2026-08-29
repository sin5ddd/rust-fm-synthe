use crate::error::{Error, Result};
use crate::preset::Preset;
use crate::voice::Voice;

/// Offline render settings. Frequency is already resolved (Hz).
#[derive(Clone, Debug)]
pub struct RenderParams {
    pub frequency_hz: f64,
    pub duration_secs: f64,
    pub velocity: f32,
    pub sample_rate: u32,
}

impl Default for RenderParams {
    fn default() -> Self {
        Self {
            frequency_hz: 130.81,
            duration_secs: 1.0,
            velocity: 0.9,
            sample_rate: 44_100,
        }
    }
}

impl RenderParams {
    pub fn validate(&self) -> Result<()> {
        if !(8_000..=192_000).contains(&self.sample_rate) {
            return Err(Error::InvalidParam {
                message: format!("sample_rate must be 8000-192000, got {}", self.sample_rate),
            });
        }
        if !(0.02..=60.0).contains(&self.duration_secs) {
            return Err(Error::InvalidParam {
                message: format!(
                    "duration must be 0.02-60 seconds, got {}",
                    self.duration_secs
                ),
            });
        }
        if !self.frequency_hz.is_finite() || self.frequency_hz <= 0.0 {
            return Err(Error::InvalidParam {
                message: format!("frequency must be > 0, got {}", self.frequency_hz),
            });
        }
        if !self.velocity.is_finite() {
            return Err(Error::InvalidParam {
                message: "velocity is not finite".into(),
            });
        }
        Ok(())
    }
}

/// Target peak after normalize, about -1 dBFS.
pub const TARGET_PEAK: f32 = 0.89125094;

/// Render a mono buffer. Peak-normalized so factory shots sit at a usable level.
pub fn render(preset: &Preset, params: &RenderParams) -> Result<Vec<f32>> {
    params.validate()?;
    let n = (params.duration_secs * f64::from(params.sample_rate)).round() as usize;
    if n == 0 {
        return Err(Error::InvalidParam {
            message: "render produced zero samples".into(),
        });
    }

    let mut voice = Voice::new(preset, params.sample_rate);
    voice.set_duration(params.duration_secs);
    voice.note_on(params.frequency_hz, params.velocity.clamp(0.0, 1.0));

    // One-shots (sustain ≈ 0): leave the gate open; the AD already dies.
    // Sustained patches: lift the gate so release fits in the buffer, but
    // never on sample 0 (that would release from amplitude 0 → silence).
    let release = f64::from(voice.max_release_secs());
    let hold = if voice.max_sustain() > 0.02 {
        let room = (params.duration_secs - release).max(params.duration_secs * 0.2);
        room.clamp(0.01, params.duration_secs)
    } else {
        params.duration_secs
    };
    let note_off_at = ((hold * f64::from(params.sample_rate)).round() as usize).clamp(1, n);

    let mut buf = vec![0.0f32; n];
    for (i, sample) in buf.iter_mut().enumerate() {
        if i == note_off_at {
            voice.note_off();
        }
        if voice.is_idle() {
            break;
        }
        let x = voice.tick();
        *sample = if x.is_finite() { x } else { 0.0 };
    }

    normalize_peak(&mut buf, TARGET_PEAK);
    Ok(buf)
}

pub fn normalize_peak(buf: &mut [f32], target: f32) {
    let peak = buf.iter().fold(0.0f32, |a, &x| a.max(x.abs()));
    if peak > 1e-8 {
        let g = target / peak;
        for x in buf.iter_mut() {
            *x *= g;
        }
    }
}

pub fn rms(buf: &[f32]) -> f32 {
    if buf.is_empty() {
        return 0.0;
    }
    let sum: f64 = buf.iter().map(|x| f64::from(*x) * f64::from(*x)).sum();
    (sum / buf.len() as f64).sqrt() as f32
}

pub fn peak(buf: &[f32]) -> f32 {
    buf.iter().fold(0.0f32, |a, &x| a.max(x.abs()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preset::load_factory;

    #[test]
    fn engine_is_not_silent() {
        let preset = load_factory("sub-bass").unwrap();
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: 55.0,
                duration_secs: 0.4,
                velocity: 1.0,
                sample_rate: 44_100,
            },
        )
        .unwrap();
        assert!(buf.iter().all(|x| x.is_finite()));
        assert!(peak(&buf) > 0.5, "peak {}", peak(&buf));
        assert!(rms(&buf) > 0.02, "rms {}", rms(&buf));
        assert!(!buf.iter().all(|&x| x == 0.0));
    }

    #[test]
    fn short_sustained_note_is_audible() {
        let preset = load_factory("growl-bass").unwrap();
        let buf = render(
            &preset,
            &RenderParams {
                frequency_hz: 110.0,
                duration_secs: 0.2,
                velocity: 0.9,
                sample_rate: 22_050,
            },
        )
        .unwrap();
        assert!(rms(&buf) > 0.02, "rms {}", rms(&buf));
    }

    #[test]
    fn different_algos_differ() {
        let mut a = load_factory("stab-pluck").unwrap();
        let mut b = a.clone();
        a.algorithm = crate::algorithm::Algorithm::Serial;
        b.algorithm = crate::algorithm::Algorithm::AllCarriers;
        let params = RenderParams {
            frequency_hz: 220.0,
            duration_secs: 0.2,
            velocity: 0.8,
            sample_rate: 22_050,
        };
        let xa = render(&a, &params).unwrap();
        let xb = render(&b, &params).unwrap();
        let diff: f32 = xa.iter().zip(&xb).map(|(l, r)| (l - r).abs()).sum();
        assert!(diff > 1.0, "algorithms produced nearly identical audio");
    }

    fn brightness(buf: &[f32]) -> f32 {
        if buf.len() < 2 {
            return 0.0;
        }
        let mut s = 0.0f32;
        for w in buf.windows(2) {
            let d = w[1] - w[0];
            s += d * d;
        }
        (s / buf.len() as f32).sqrt()
    }

    #[test]
    fn supersaw_render_is_audible_and_not_a_sine() {
        let mut saw = load_factory("sub-bass").unwrap();
        let mut sine = saw.clone();
        for op in &mut saw.operators {
            op.waveform = crate::Waveform::SuperSaw;
        }
        for op in &mut sine.operators {
            op.waveform = crate::Waveform::Sine;
        }
        let params = RenderParams {
            frequency_hz: 55.0,
            duration_secs: 0.35,
            velocity: 1.0,
            sample_rate: 22_050,
        };
        let xa = render(&saw, &params).unwrap();
        let xb = render(&sine, &params).unwrap();
        assert!(xa.iter().all(|x| x.is_finite()));
        assert!(peak(&xa) > 0.4, "super-saw peak {}", peak(&xa));
        assert!(rms(&xa) > 0.02, "super-saw rms {}", rms(&xa));
        let diff: f32 = xa.iter().zip(&xb).map(|(l, r)| (l - r).abs()).sum();
        assert!(
            diff > 2.0,
            "super-saw render nearly identical to sine (diff={diff})"
        );
    }

    #[test]
    fn lowpass_low_cutoff_attenuates_highs() {
        let mut closed = load_factory("zap").unwrap();
        let mut open = closed.clone();
        closed.filter.kind = crate::FilterType::Lowpass;
        closed.filter.cutoff = 220.0;
        closed.filter.resonance = 0.1;
        closed.filter.env_amount = 0.0;
        open.filter.kind = crate::FilterType::Lowpass;
        open.filter.cutoff = 16_000.0;
        open.filter.resonance = 0.1;
        open.filter.env_amount = 0.0;
        let params = RenderParams {
            frequency_hz: 110.0,
            duration_secs: 0.25,
            velocity: 0.9,
            sample_rate: 22_050,
        };
        let dark = render(&closed, &params).unwrap();
        let bright = render(&open, &params).unwrap();
        let b_dark = brightness(&dark);
        let b_bright = brightness(&bright);
        assert!(
            b_dark < b_bright * 0.55,
            "low cutoff should be darker (closed={b_dark}, open={b_bright})"
        );
    }

    #[test]
    fn factory_filter_modes_parse() {
        let saw = load_factory("supersaw-bass").unwrap();
        assert!(saw
            .operators
            .iter()
            .any(|o| o.waveform == crate::Waveform::SuperSaw));
        let pluck = load_factory("filter-pluck").unwrap();
        assert_eq!(pluck.filter.kind, crate::FilterType::Lowpass);
        assert!(pluck.filter.env_amount > 1.0);
        let bp = load_factory("bp-growl").unwrap();
        assert_eq!(bp.filter.kind, crate::FilterType::Bandpass);
        let hp = load_factory("hp-air").unwrap();
        assert_eq!(hp.filter.kind, crate::FilterType::Highpass);
    }
}
