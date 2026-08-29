/// Linear attack, exponential decay/release. Times are in seconds.
#[derive(Clone, Debug)]
pub struct AdsrParams {
    pub attack: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
}

impl Default for AdsrParams {
    fn default() -> Self {
        Self {
            attack: 0.005,
            decay: 0.2,
            sustain: 0.0,
            release: 0.05,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Attack,
    Decay,
    Sustain,
    Release,
    Idle,
}

/// Per-voice envelope runner.
#[derive(Clone, Debug)]
pub struct Adsr {
    params: AdsrParams,
    sample_rate: f32,
    stage: Stage,
    value: f32,
    attack_inc: f32,
    decay_coeff: f32,
    release_coeff: f32,
}

impl Adsr {
    pub fn new(params: AdsrParams, sample_rate: f32) -> Self {
        let mut env = Self {
            params,
            sample_rate,
            stage: Stage::Idle,
            value: 0.0,
            attack_inc: 0.0,
            decay_coeff: 0.0,
            release_coeff: 0.0,
        };
        env.recompute();
        env
    }

    fn recompute(&mut self) {
        let sr = self.sample_rate.max(1.0);
        self.attack_inc = inc_from_time(1.0, self.params.attack, sr);
        self.decay_coeff = exp_coeff(self.params.decay, sr);
        self.release_coeff = exp_coeff(self.params.release, sr);
    }

    pub fn note_on(&mut self) {
        if self.params.attack <= 1.0 / self.sample_rate {
            self.value = 1.0;
            self.stage = if self.params.sustain >= 0.999 {
                Stage::Sustain
            } else {
                Stage::Decay
            };
        } else {
            self.stage = Stage::Attack;
        }
    }

    pub fn note_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
        }
    }

    pub fn is_idle(&self) -> bool {
        self.stage == Stage::Idle
    }

    pub fn release_secs(&self) -> f32 {
        self.params.release
    }

    pub fn sustain(&self) -> f32 {
        self.params.sustain
    }

    pub fn tick(&mut self) -> f32 {
        match self.stage {
            Stage::Idle => {
                self.value = 0.0;
            }
            Stage::Attack => {
                self.value += self.attack_inc;
                if self.value >= 1.0 {
                    self.value = 1.0;
                    self.stage = Stage::Decay;
                }
            }
            Stage::Decay => {
                let sustain = self.params.sustain.clamp(0.0, 1.0);
                self.value = sustain + (self.value - sustain) * self.decay_coeff;
                if (self.value - sustain).abs() < 1e-4 {
                    self.value = sustain;
                    self.stage = if sustain <= 1e-5 {
                        Stage::Idle
                    } else {
                        Stage::Sustain
                    };
                }
            }
            Stage::Sustain => {
                self.value = self.params.sustain.clamp(0.0, 1.0);
            }
            Stage::Release => {
                self.value *= self.release_coeff;
                if self.value < 1e-5 {
                    self.value = 0.0;
                    self.stage = Stage::Idle;
                }
            }
        }
        self.value
    }
}

fn inc_from_time(distance: f32, secs: f32, sample_rate: f32) -> f32 {
    let samples = (secs.max(0.0) * sample_rate).max(1.0);
    distance / samples
}

/// Coefficient so that the envelope falls to ~-80 dB in `secs`.
fn exp_coeff(secs: f32, sample_rate: f32) -> f32 {
    let samples = (secs.max(0.0) * sample_rate).max(1.0);
    // 1e-4 ≈ -80 dB residual
    (-9.2f32 / samples).exp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attack_reaches_peak() {
        let mut env = Adsr::new(
            AdsrParams {
                attack: 0.01,
                decay: 0.01,
                sustain: 0.5,
                release: 0.01,
            },
            1000.0,
        );
        env.note_on();
        let mut peak = 0.0f32;
        for _ in 0..30 {
            peak = peak.max(env.tick());
        }
        assert!(peak >= 0.99, "peak={peak}");
    }

    #[test]
    fn release_goes_idle() {
        let mut env = Adsr::new(
            AdsrParams {
                attack: 0.0,
                decay: 0.0,
                sustain: 1.0,
                release: 0.02,
            },
            2000.0,
        );
        env.note_on();
        let _ = env.tick();
        env.note_off();
        for _ in 0..200 {
            let _ = env.tick();
        }
        assert!(env.is_idle());
        assert!(env.tick() < 1e-4);
    }
}
