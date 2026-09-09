use serde::Deserialize;

const CHORUS_MAX: usize = 2048;
const DELAY_MAX: usize = 65536;
const REVERB_COMB_MAX: usize = 8192;
const REVERB_AP_MAX: usize = 2048;
const ACTIVE_EPS: f32 = 1e-8;
const HARMONY_MAX: usize = 8192;
const HARMONY_VOICES: usize = 4;

const COMB_TIMES: [f32; 4] = [0.0297, 0.0371, 0.0411, 0.0437];
const AP_TIMES: [f32; 2] = [0.005, 0.0017];
const AP_COEFF: f32 = 0.5;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChorusInterval {
    #[default]
    Unison,
    OctaveUp,
    OctaveDown,
    Fifth,
    Fourth,
}

impl ChorusInterval {
    fn ratio(self) -> f32 {
        match self {
            Self::Unison => 1.0,
            Self::OctaveUp => 2.0,
            Self::OctaveDown => 0.5,
            Self::Fifth => 1.5,
            Self::Fourth => 4.0 / 3.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct OverdriveParams {
    pub mix: f32,
    pub drive: f32,
}

impl Default for OverdriveParams {
    fn default() -> Self {
        Self {
            mix: 0.0,
            drive: 0.25,
        }
    }
}

impl OverdriveParams {
    pub fn is_active(&self) -> bool {
        self.mix.abs() > ACTIVE_EPS
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ChorusParams {
    pub mix: f32,
    pub rate_hz: f32,
    pub depth_ms: f32,
    pub delay_ms: f32,
    pub feedback: f32,
    /// Extra pitches on the unison chorus. Omit or `[]` = unison only.
    /// `unison` / `octave-up` / `octave-down` / `fifth` / `fourth`. Max 4.
    pub intervals: Vec<ChorusInterval>,
}

impl Default for ChorusParams {
    fn default() -> Self {
        Self {
            mix: 0.0,
            rate_hz: 0.85,
            depth_ms: 5.0,
            delay_ms: 14.0,
            feedback: 0.15,
            intervals: Vec::new(),
        }
    }
}

impl ChorusParams {
    pub fn is_active(&self) -> bool {
        self.mix.abs() > ACTIVE_EPS
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct DelayParams {
    pub mix: f32,
    pub time_ms: f32,
    pub feedback: f32,
    pub damping: f32,
}

impl Default for DelayParams {
    fn default() -> Self {
        Self {
            mix: 0.0,
            time_ms: 280.0,
            feedback: 0.28,
            damping: 0.35,
        }
    }
}

impl DelayParams {
    pub fn is_active(&self) -> bool {
        self.mix.abs() > ACTIVE_EPS
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct ReverbParams {
    pub mix: f32,
    pub room: f32,
    pub damp: f32,
}

impl Default for ReverbParams {
    fn default() -> Self {
        Self {
            mix: 0.0,
            room: 0.40,
            damp: 0.45,
        }
    }
}

impl ReverbParams {
    pub fn is_active(&self) -> bool {
        self.mix.abs() > ACTIVE_EPS
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct FxParams {
    pub overdrive: OverdriveParams,
    pub chorus: ChorusParams,
    pub delay: DelayParams,
    pub reverb: ReverbParams,
}

impl Default for FxParams {
    fn default() -> Self {
        Self {
            overdrive: OverdriveParams::default(),
            chorus: ChorusParams::default(),
            delay: DelayParams::default(),
            reverb: ReverbParams::default(),
        }
    }
}

impl FxParams {
    pub fn is_active(&self) -> bool {
        self.overdrive.is_active()
            || self.chorus.is_active()
            || self.delay.is_active()
            || self.reverb.is_active()
    }
}

struct ChorusVoice {
    buf: Vec<f32>,
    w: usize,
    phase: f32,
}

struct HarmonyVoice {
    buf: Vec<f32>,
    w: usize,
    pos: f32,
    phase: f32,
}

struct DelayLine {
    buf: Vec<f32>,
    w: usize,
    lp: f32,
}

struct ReverbComb {
    buf: Vec<f32>,
    w: usize,
    lp: f32,
}

struct Allpass {
    buf: Vec<f32>,
    w: usize,
}

pub(crate) struct FxChain {
    params: FxParams,
    sample_rate: f32,
    chorus: [ChorusVoice; 2],
    harmony: [HarmonyVoice; HARMONY_VOICES],
    delay: DelayLine,
    combs: [ReverbComb; 4],
    allpasses: [Allpass; 2],
}

fn read_delay(buf: &[f32], w: usize, delay: f32, max: usize) -> f32 {
    let delay = f64::from(delay);
    let i0 = delay.floor() as usize;
    let frac = (delay - i0 as f64) as f32;
    let d0 = buf[(w + max - i0) % max];
    let d1 = buf[(w + max - i0 - 1) % max];
    d0 * (1.0 - frac) + d1 * frac
}

fn frac_at(buf: &[f32], pos: f32) -> f32 {
    let n = buf.len();
    let nf = n as f32;
    let p = pos.rem_euclid(nf);
    let i0 = (p as usize).min(n - 1);
    let i1 = if i0 + 1 < n { i0 + 1 } else { 0 };
    let f = p - i0 as f32;
    buf[i0] * (1.0 - f) + buf[i1] * f
}

fn tick_harmony(v: &mut HarmonyVoice, input: f32, ratio: f32) -> f32 {
    let n = v.buf.len();
    let nf = n as f32;
    v.buf[v.w] = if input.is_finite() { input } else { 0.0 };
    let p0 = v.pos;
    let p1 = p0 + nf * 0.5;
    let y0 = frac_at(&v.buf, p0);
    let y1 = frac_at(&v.buf, p1);
    let x = (v.w as f32 - p0).rem_euclid(nf) / nf;
    let g0 = 0.5 - 0.5 * (std::f32::consts::TAU * x).cos();
    let y = y0 * g0 + y1 * (1.0 - g0);
    v.w = (v.w + 1) % n;
    v.pos += ratio;
    if v.pos >= nf {
        v.pos %= nf;
    }
    if y.is_finite() {
        y
    } else {
        v.buf.fill(0.0);
        v.w = 0;
        v.pos = 0.0;
        0.0
    }
}

impl FxChain {
    pub fn new(params: FxParams, sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        Self {
            params,
            sample_rate: sr,
            chorus: [
                ChorusVoice {
                    buf: vec![0.0; CHORUS_MAX],
                    w: 0,
                    phase: 0.0,
                },
                ChorusVoice {
                    buf: vec![0.0; CHORUS_MAX],
                    w: 0,
                    phase: 0.5 * std::f32::consts::PI,
                },
            ],
            harmony: std::array::from_fn(|i| HarmonyVoice {
                buf: vec![0.0; HARMONY_MAX],
                w: 0,
                pos: 0.0,
                phase: i as f32 * 0.5 * std::f32::consts::PI,
            }),
            delay: DelayLine {
                buf: vec![0.0; DELAY_MAX],
                w: 0,
                lp: 0.0,
            },
            combs: [
                ReverbComb {
                    buf: vec![0.0; REVERB_COMB_MAX],
                    w: 0,
                    lp: 0.0,
                },
                ReverbComb {
                    buf: vec![0.0; REVERB_COMB_MAX],
                    w: 0,
                    lp: 0.0,
                },
                ReverbComb {
                    buf: vec![0.0; REVERB_COMB_MAX],
                    w: 0,
                    lp: 0.0,
                },
                ReverbComb {
                    buf: vec![0.0; REVERB_COMB_MAX],
                    w: 0,
                    lp: 0.0,
                },
            ],
            allpasses: [
                Allpass {
                    buf: vec![0.0; REVERB_AP_MAX],
                    w: 0,
                },
                Allpass {
                    buf: vec![0.0; REVERB_AP_MAX],
                    w: 0,
                },
            ],
        }
    }

    pub fn reset(&mut self) {
        self.reset_chorus();
        self.reset_delay();
        self.reset_reverb();
    }

    pub fn is_active(&self) -> bool {
        self.params.is_active()
    }

    pub fn tick(&mut self, input: f32) -> f32 {
        if !self.is_active() {
            return input;
        }
        let x = self.tick_overdrive(input);
        let x = self.tick_chorus(x);
        let x = self.tick_delay(x);
        self.tick_reverb(x)
    }

    fn reset_chorus(&mut self) {
        self.chorus[0].buf.fill(0.0);
        self.chorus[0].w = 0;
        self.chorus[0].phase = 0.0;
        self.chorus[1].buf.fill(0.0);
        self.chorus[1].w = 0;
        self.chorus[1].phase = 0.5 * std::f32::consts::PI;
        for (i, h) in self.harmony.iter_mut().enumerate() {
            h.buf.fill(0.0);
            h.w = 0;
            h.pos = 0.0;
            h.phase = i as f32 * 0.5 * std::f32::consts::PI;
        }
    }

    fn reset_delay(&mut self) {
        self.delay.buf.fill(0.0);
        self.delay.w = 0;
        self.delay.lp = 0.0;
    }

    fn reset_reverb(&mut self) {
        for c in &mut self.combs {
            c.buf.fill(0.0);
            c.w = 0;
            c.lp = 0.0;
        }
        for a in &mut self.allpasses {
            a.buf.fill(0.0);
            a.w = 0;
        }
    }

    fn tick_overdrive(&mut self, input: f32) -> f32 {
        let mix = self.params.overdrive.mix.clamp(0.0, 1.0);
        if mix <= ACTIVE_EPS {
            return input;
        }
        let drive = self.params.overdrive.drive.clamp(0.0, 1.0);
        let g = 1.0 + drive * 8.0;
        let y = (input * g).clamp(-8.0, 8.0).tanh();
        let out = input * (1.0 - mix) + y * mix;
        if out.is_finite() {
            out
        } else {
            0.0
        }
    }

    fn tick_chorus(&mut self, input: f32) -> f32 {
        let mix = self.params.chorus.mix.clamp(0.0, 1.0);
        if mix <= ACTIVE_EPS {
            return input;
        }
        let rate = self.params.chorus.rate_hz.max(0.0);
        let depth_ms = self.params.chorus.depth_ms.clamp(0.0, 8.0);
        let delay_ms = self.params.chorus.delay_ms.clamp(8.0, 30.0);
        let feedback = self.params.chorus.feedback.clamp(0.0, 0.7);
        let sr = self.sample_rate;
        let rates = [rate, rate * 1.37];
        let max_d = (CHORUS_MAX - 2) as f32;
        let mut acc = 0.0;
        for (i, voice) in self.chorus.iter_mut().enumerate() {
            let dms = delay_ms + depth_ms * voice.phase.sin();
            let ds = (sr * dms / 1000.0).clamp(1.5, max_d);
            let delayed = read_delay(&voice.buf, voice.w, ds, CHORUS_MAX);
            let written = input + feedback * delayed;
            voice.buf[voice.w] = if written.is_finite() { written } else { 0.0 };
            voice.w = (voice.w + 1) % CHORUS_MAX;
            voice.phase += std::f32::consts::TAU * rates[i] / sr;
            if voice.phase > std::f32::consts::TAU {
                voice.phase %= std::f32::consts::TAU;
            }
            acc += delayed;
        }
        let unison = 0.5 * acc;
        let mut harm = 0.0;
        let mut n_h = 0u32;
        let rate_step = std::f32::consts::TAU * rate / sr;
        for (i, iv) in self
            .params
            .chorus
            .intervals
            .iter()
            .copied()
            .take(HARMONY_VOICES)
            .enumerate()
        {
            let ratio = iv.ratio();
            if (ratio - 1.0).abs() <= 1e-6 {
                continue;
            }
            let h = &mut self.harmony[i];
            h.phase += rate_step;
            if h.phase > std::f32::consts::TAU {
                h.phase %= std::f32::consts::TAU;
            }
            let detune = 1.0 + 0.003 * h.phase.sin();
            harm += tick_harmony(h, input, ratio * detune);
            n_h += 1;
        }
        let chorus_out = if n_h == 0 {
            unison
        } else {
            unison * 0.7 + (harm / n_h as f32) * 0.3
        };
        let out = input * (1.0 - mix) + chorus_out * mix;
        if out.is_finite() {
            out
        } else {
            self.reset_chorus();
            0.0
        }
    }

    fn tick_delay(&mut self, input: f32) -> f32 {
        let mix = self.params.delay.mix.clamp(0.0, 1.0);
        if mix <= ACTIVE_EPS {
            return input;
        }
        let time_ms = self.params.delay.time_ms.clamp(20.0, 1200.0);
        let feedback = self.params.delay.feedback.clamp(0.0, 0.85);
        let damp = self.params.delay.damping.clamp(0.0, 1.0);
        let max_d = (DELAY_MAX - 2) as f32;
        let ds = (self.sample_rate * time_ms / 1000.0).clamp(1.5, max_d);
        let delayed = read_delay(&self.delay.buf, self.delay.w, ds, DELAY_MAX);
        self.delay.lp = damp * self.delay.lp + (1.0 - damp) * delayed;
        if !self.delay.lp.is_finite() {
            self.delay.lp = 0.0;
        }
        let written = input + feedback * self.delay.lp;
        if !written.is_finite() {
            self.reset_delay();
            return 0.0;
        }
        self.delay.buf[self.delay.w] = written;
        self.delay.w = (self.delay.w + 1) % DELAY_MAX;
        let out = input * (1.0 - mix) + delayed * mix;
        if out.is_finite() {
            out
        } else {
            self.reset_delay();
            0.0
        }
    }

    fn tick_reverb(&mut self, input: f32) -> f32 {
        let mix = self.params.reverb.mix.clamp(0.0, 1.0);
        if mix <= ACTIVE_EPS {
            return input;
        }
        let room = self.params.reverb.room.clamp(0.0, 1.0);
        let damp = self.params.reverb.damp.clamp(0.0, 1.0);
        let room_scale = 0.85 + room * 0.30;
        let feedback = 0.28 + room * 0.55;
        let sr = self.sample_rate;

        let mut sum = 0.0;
        for (i, comb) in self.combs.iter_mut().enumerate() {
            let d = (sr * COMB_TIMES[i] * room_scale)
                .round()
                .clamp(1.0, (REVERB_COMB_MAX - 1) as f32) as usize;
            let delayed = comb.buf[(comb.w + REVERB_COMB_MAX - d) % REVERB_COMB_MAX];
            comb.lp = damp * comb.lp + (1.0 - damp) * delayed;
            if !comb.lp.is_finite() {
                comb.lp = 0.0;
            }
            let written = input + feedback * comb.lp;
            if !written.is_finite() {
                self.reset_reverb();
                return 0.0;
            }
            comb.buf[comb.w] = written;
            comb.w = (comb.w + 1) % REVERB_COMB_MAX;
            sum += delayed;
        }

        let mut x = sum * 0.25;
        for (i, ap) in self.allpasses.iter_mut().enumerate() {
            let d = (sr * AP_TIMES[i] * room_scale)
                .round()
                .clamp(1.0, (REVERB_AP_MAX - 1) as f32) as usize;
            let delayed = ap.buf[(ap.w + REVERB_AP_MAX - d) % REVERB_AP_MAX];
            let v = x + AP_COEFF * delayed;
            if !v.is_finite() {
                self.reset_reverb();
                return 0.0;
            }
            ap.buf[ap.w] = v;
            ap.w = (ap.w + 1) % REVERB_AP_MAX;
            x = delayed - AP_COEFF * v;
        }

        let out = input * (1.0 - mix) + x * mix;
        if out.is_finite() {
            out
        } else {
            self.reset_reverb();
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(freq: f32, sr: f32, n: usize) -> Vec<f32> {
        let w = std::f32::consts::TAU * freq / sr;
        (0..n).map(|i| (w * i as f32).sin()).collect()
    }

    fn run(params: FxParams, input: &[f32], sr: f32) -> Vec<f32> {
        let mut fx = FxChain::new(params, sr);
        input.iter().map(|&x| fx.tick(x)).collect()
    }

    #[test]
    fn bypass_is_identity() {
        let sr = 44_100.0;
        let input = sine(200.0, sr, 512);
        let y = run(FxParams::default(), &input, sr);
        let err: f32 = input.iter().zip(&y).map(|(a, b)| (a - b).abs()).sum();
        assert!(err < 1e-9, "bypass error {err}");
    }

    #[test]
    fn chorus_differs_from_dry() {
        let sr = 44_100.0;
        let input = sine(220.0, sr, 4096);
        let params = FxParams {
            chorus: ChorusParams {
                mix: 1.0,
                rate_hz: 0.85,
                depth_ms: 5.0,
                delay_ms: 14.0,
                feedback: 0.15,
                ..ChorusParams::default()
            },
            ..FxParams::default()
        };
        let y = run(params, &input, sr);
        let err: f32 = input.iter().zip(&y).map(|(a, b)| (a - b).abs()).sum();
        assert!(err > 10.0, "chorus should move the signal, err={err}");
    }

    #[test]
    fn chorus_octave_differs_from_unison() {
        let sr = 44_100.0;
        let input = sine(220.0, sr, 4096);
        let unison = FxParams {
            chorus: ChorusParams {
                mix: 1.0,
                ..ChorusParams::default()
            },
            ..FxParams::default()
        };
        let oct = FxParams {
            chorus: ChorusParams {
                mix: 1.0,
                intervals: vec![ChorusInterval::OctaveUp, ChorusInterval::Fifth],
                ..ChorusParams::default()
            },
            ..FxParams::default()
        };
        let a = run(unison, &input, sr);
        let b = run(oct, &input, sr);
        let err: f32 = a.iter().zip(&b).map(|(x, y)| (x - y).abs()).sum();
        assert!(
            err > 10.0,
            "harmony voices should change the chorus, err={err}"
        );
    }

    #[test]
    fn delay_echo_at_time() {
        let sr = 1000.0;
        let mut input = vec![0.0f32; 200];
        input[0] = 1.0;
        let params = FxParams {
            delay: DelayParams {
                mix: 1.0,
                time_ms: 50.0,
                feedback: 0.0,
                damping: 0.0,
            },
            ..FxParams::default()
        };
        let y = run(params, &input, sr);
        let (idx, _) = y
            .iter()
            .enumerate()
            .skip(3)
            .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
            .unwrap();
        assert!(
            (idx as i32 - 50).abs() <= 3,
            "echo peak at {idx}, expected 50±3"
        );
    }

    #[test]
    fn overdrive_compresses_peak() {
        let input: Vec<f32> = (0..64)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let params = FxParams {
            overdrive: OverdriveParams {
                mix: 1.0,
                drive: 1.0,
            },
            ..FxParams::default()
        };
        let y = run(params, &input, 44_100.0);
        let peak = y.iter().fold(0.0f32, |a, &x| a.max(x.abs()));
        // g=9 → tanh(8)≈1; linear would be 9. Bound 0.95 assumed a milder tanh arg.
        assert!(
            peak < 1.0,
            "tanh peak {peak} should saturate below linear 9"
        );
        assert!(peak > 0.9, "drive=1 should be in saturation, got {peak}");
    }

    #[test]
    fn reverb_tail_after_impulse() {
        let sr = 44_100.0;
        let mut input = vec![0.0f32; 8000];
        input[0] = 1.0;
        let params = FxParams {
            reverb: ReverbParams {
                mix: 1.0,
                room: 0.5,
                damp: 0.45,
            },
            ..FxParams::default()
        };
        let y = run(params, &input, sr);
        let tail = y[2000..8000].iter().any(|&x| x.abs() > 1e-4);
        assert!(tail, "reverb should still ring between 2000..8000");
    }

    #[test]
    fn high_drive_stays_finite() {
        let sr = 44_100.0;
        let input: Vec<f32> = (0..8000)
            .map(|i| if i % 64 == 0 { 1.0 } else { 0.0 })
            .collect();
        let params = FxParams {
            overdrive: OverdriveParams {
                mix: 1.0,
                drive: 1.0,
            },
            chorus: ChorusParams {
                mix: 1.0,
                intervals: vec![ChorusInterval::OctaveUp, ChorusInterval::Fifth],
                ..ChorusParams::default()
            },
            delay: DelayParams {
                mix: 1.0,
                time_ms: 100.0,
                feedback: 0.8,
                damping: 0.35,
            },
            reverb: ReverbParams {
                mix: 1.0,
                room: 0.9,
                damp: 0.45,
            },
        };
        let y = run(params, &input, sr);
        assert!(
            y.iter().all(|x| x.is_finite()),
            "fx chain produced non-finite"
        );
    }
}
