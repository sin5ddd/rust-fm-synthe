use crate::filter::{FilterType, Svf};
use serde::Deserialize;

const COMB_MAX: usize = 4096;
const RNG_SEED: u32 = 0xC0FF_EE11;
const GLIDE_SECS: f32 = 0.02;
const RHOTIC_GLIDE_SECS: f32 = 0.06;
const RHOTIC_F3_HZ: f32 = 1600.0;

/// Peterson-Barney female vowels. Absolute Hz; does not follow note pitch.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Vowel {
    #[default]
    A,
    E,
    I,
    O,
    U,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SegmentKind {
    #[default]
    Vowel,
    Plosive,
    Fricative,
    Breath,
    Nasal,
    /// Alveolar tap (Japanese ら). Voiced throughout; brief amplitude dip.
    Flap,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct CombParams {
    pub mix: f32,
    pub feedback: f32,
    pub damping: f32,
    pub ratio: f32,
}

impl Default for CombParams {
    fn default() -> Self {
        Self {
            mix: 0.0,
            feedback: 0.5,
            damping: 0.35,
            ratio: 1.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct VocalSegment {
    pub at: f32,
    pub dur: f32,
    pub kind: SegmentKind,
    pub vowel: Vowel,
    pub place_hz: f32,
    pub voiced: Option<f32>,
}

impl Default for VocalSegment {
    fn default() -> Self {
        Self {
            at: 0.0,
            dur: 0.05,
            kind: SegmentKind::Vowel,
            vowel: Vowel::A,
            place_hz: 0.0,
            voiced: None,
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(default)]
pub struct VocalParams {
    pub mix: f32,
    pub vowel: Vowel,
    pub formant_resonance: f32,
    /// Peak relative F1/F2/F3 wobble (0 = off). Typical sung 0.04–0.05.
    pub formant_wobble: f32,
    /// Wobble rate in Hz. 0 with wobble>0 uses 5.5.
    pub formant_wobble_hz: f32,
    pub comb: CombParams,
    pub segments: Vec<VocalSegment>,
}

impl Default for VocalParams {
    fn default() -> Self {
        Self {
            mix: 0.0,
            vowel: Vowel::A,
            formant_resonance: 0.55,
            formant_wobble: 0.0,
            formant_wobble_hz: 0.0,
            comb: CombParams::default(),
            segments: Vec::new(),
        }
    }
}

impl VocalParams {
    pub fn is_active(&self) -> bool {
        self.mix.abs() > 1e-8
    }
}

/// F1, F2, F3 in Hz (Peterson-Barney female). Pitch-independent.
pub fn vowel_hz(v: Vowel) -> (f32, f32, f32) {
    match v {
        Vowel::A => (850.0, 1220.0, 2810.0),
        Vowel::E => (610.0, 2330.0, 2990.0),
        Vowel::I => (310.0, 2790.0, 3310.0),
        Vowel::O => (590.0, 920.0, 2710.0),
        Vowel::U => (370.0, 950.0, 2670.0),
    }
}

fn place_default(kind: SegmentKind) -> f32 {
    match kind {
        SegmentKind::Plosive => 2000.0,
        SegmentKind::Fricative => 5500.0,
        SegmentKind::Breath => 2400.0,
        SegmentKind::Flap => 1800.0,
        SegmentKind::Vowel | SegmentKind::Nasal => 0.0,
    }
}

fn resolved_place(kind: SegmentKind, place_hz: f32) -> f32 {
    if place_hz > 0.0 {
        place_hz
    } else {
        place_default(kind)
    }
}

fn voiced_target(kind: SegmentKind, voiced: Option<f32>) -> f32 {
    if let Some(v) = voiced {
        v
    } else {
        match kind {
            SegmentKind::Vowel | SegmentKind::Nasal | SegmentKind::Plosive | SegmentKind::Flap => {
                1.0
            }
            SegmentKind::Fricative => 0.12,
            SegmentKind::Breath => 0.20,
        }
    }
}

fn scale_nasal(hz: (f32, f32, f32), kind: SegmentKind, place_hz: f32) -> (f32, f32, f32) {
    if kind != SegmentKind::Nasal {
        return hz;
    }
    let (f1, f2, f3) = hz;
    if place_hz < 2000.0 {
        (f1 * 0.55, f2 * 0.78, f3)
    } else {
        (f1 * 0.42, f2 * 0.65, f3)
    }
}

/// English /r/ and curled-tongue ら: F3 well below 2000 Hz.
fn apply_kind_formants(hz: (f32, f32, f32), kind: SegmentKind, place_hz: f32) -> (f32, f32, f32) {
    let hz = scale_nasal(hz, kind, place_hz);
    if kind != SegmentKind::Flap {
        return hz;
    }
    let (f1, f2, _) = hz;
    let f3 = if place_hz > 0.0 && place_hz < 2200.0 {
        place_hz
    } else {
        RHOTIC_F3_HZ
    };
    let f2 = f2.min((f3 - 200.0).max(900.0));
    (f1 * 0.88, f2, f3)
}

fn initial_formants(params: &VocalParams) -> (f32, f32, f32) {
    let mut chosen: Option<&VocalSegment> = None;
    for s in &params.segments {
        if s.at <= 0.0 {
            chosen = Some(s);
        }
    }
    match chosen {
        Some(s) => apply_kind_formants(vowel_hz(s.vowel), s.kind, s.place_hz),
        None => vowel_hz(params.vowel),
    }
}

fn lerp3(a: (f32, f32, f32), b: (f32, f32, f32), t: f32) -> (f32, f32, f32) {
    let t = t.clamp(0.0, 1.0);
    (
        a.0 + (b.0 - a.0) * t,
        a.1 + (b.1 - a.1) * t,
        a.2 + (b.2 - a.2) * t,
    )
}

struct ActiveSeg {
    at: f32,
    dur: f32,
    kind: SegmentKind,
    vowel: Vowel,
    place_hz: f32,
    voiced: Option<f32>,
}

pub struct VocalRuntime {
    params: VocalParams,
    comb_buf: Vec<f32>,
    comb_w: usize,
    comb_lp: f32,
    f1: Svf,
    f2: Svf,
    f3: Svf,
    burst: Svf,
    rng: u32,
    sample_rate: f32,
    time: f64,
    glide_from: (f32, f32, f32),
    glide_to: (f32, f32, f32),
    glide_left: f32,
    glide_secs: f32,
    prev_vowel: Vowel,
}

impl VocalRuntime {
    pub fn new(params: VocalParams, sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let hz = initial_formants(&params);
        Self {
            prev_vowel: params.vowel,
            params,
            comb_buf: vec![0.0; COMB_MAX],
            comb_w: 0,
            comb_lp: 0.0,
            f1: Svf::new(sr),
            f2: Svf::new(sr),
            f3: Svf::new(sr),
            burst: Svf::new(sr),
            rng: RNG_SEED,
            sample_rate: sr,
            time: 0.0,
            glide_from: hz,
            glide_to: hz,
            glide_left: 0.0,
            glide_secs: GLIDE_SECS,
        }
    }

    pub fn note_on(&mut self) {
        self.rng = RNG_SEED;
        self.comb_buf.fill(0.0);
        self.comb_w = 0;
        self.comb_lp = 0.0;
        self.f1.reset();
        self.f2.reset();
        self.f3.reset();
        self.burst.reset();
        self.time = 0.0;
        let hz = initial_formants(&self.params);
        self.glide_from = hz;
        self.glide_to = hz;
        self.glide_left = 0.0;
        self.glide_secs = GLIDE_SECS;
        self.prev_vowel = self.params.vowel;
    }

    pub fn tick(&mut self, input: f32, note_hz: f64, pitch_mult: f64) -> f32 {
        if !self.params.is_active() {
            return input;
        }

        let t = self.time;
        self.time += 1.0 / f64::from(self.sample_rate);
        let t32 = t as f32;

        let seg = self.active_seg(t32);
        let target = apply_kind_formants(vowel_hz(seg.vowel), seg.kind, seg.place_hz);
        if seg.vowel != self.prev_vowel || target != self.glide_to {
            self.glide_from = self.current_formants();
            self.glide_to = target;
            let f3_jump = (target.2 - self.glide_from.2).abs();
            self.glide_secs = if f3_jump > 400.0 {
                RHOTIC_GLIDE_SECS
            } else {
                GLIDE_SECS
            };
            self.glide_left = self.glide_secs;
            self.prev_vowel = seg.vowel;
        }
        let (mut f1_hz, mut f2_hz, mut f3_hz) = self.current_formants();
        let wob = self.params.formant_wobble.clamp(0.0, 0.15);
        if wob > 1e-8 {
            let hz = if self.params.formant_wobble_hz > 0.0 {
                self.params.formant_wobble_hz
            } else {
                5.5
            };
            let s = (std::f32::consts::TAU * hz * t32).sin();
            f1_hz *= 1.0 + wob * s;
            f2_hz *= 1.0 + wob * 1.15 * s;
            f3_hz *= 1.0 + wob * 0.7 * s;
        }
        let dt = 1.0 / self.sample_rate;
        if self.glide_left > 0.0 {
            self.glide_left = (self.glide_left - dt).max(0.0);
        }

        let voiced_tgt = voiced_target(seg.kind, seg.voiced);
        let u = if seg.dur <= 0.0 {
            1.0
        } else {
            ((t32 - seg.at) / seg.dur).clamp(0.0, 1.0)
        };
        let voiced_gain = if seg.kind == SegmentKind::Plosive {
            if u < 0.4 {
                0.0
            } else {
                (voiced_tgt * (u - 0.4) / 0.6).clamp(0.0, 1.0)
            }
        } else if seg.kind == SegmentKind::Flap {
            if u < 0.5 {
                (0.35 + 0.65 * (u / 0.5)) * voiced_tgt
            } else {
                voiced_tgt
            }
        } else {
            voiced_tgt
        };

        let in_burst = t32 >= seg.at && t32 < seg.at + seg.dur;
        let burst_env = if !in_burst {
            0.0
        } else {
            match seg.kind {
                SegmentKind::Plosive => {
                    if u < 0.4 {
                        0.0
                    } else {
                        (1.0 - (u - 0.4) / 0.6).max(0.0) * 0.9
                    }
                }
                SegmentKind::Fricative => 0.32,
                SegmentKind::Breath => 0.35,
                SegmentKind::Flap => (1.0 - u).max(0.0) * 0.12,
                SegmentKind::Vowel | SegmentKind::Nasal => 0.0,
            }
        };
        let burst_sig = if burst_env > 0.0 {
            let n = self.next_pm1();
            let cutoff = resolved_place(seg.kind, seg.place_hz);
            let bp = self.burst.tick(n, cutoff, 0.40, FilterType::Bandpass);
            bp * burst_env * 0.8
        } else {
            0.0
        };

        let mut src = input * voiced_gain + burst_sig;

        let comb_mix = self.params.comb.mix.clamp(0.0, 1.0);
        if comb_mix > 1e-8 {
            src = self.tick_comb(src, note_hz, pitch_mult, comb_mix);
        }

        let q_res = self.params.formant_resonance.clamp(0.0, 1.0);
        let y1 = self.f1.tick(src, f1_hz, q_res, FilterType::Bandpass);
        let y2 = self.f2.tick(src, f2_hz, q_res, FilterType::Bandpass);
        let y3 = self.f3.tick(src, f3_hz, q_res, FilterType::Bandpass);
        let wet = (y1 * 1.0 + y2 * 0.75 + y3 * 0.5) / 2.25;
        let mix = self.params.mix.clamp(0.0, 1.0);
        let out = src * (1.0 - mix) + wet * mix;
        if out.is_finite() {
            out
        } else {
            0.0
        }
    }

    fn current_formants(&self) -> (f32, f32, f32) {
        if self.glide_left <= 0.0 {
            self.glide_to
        } else {
            let dur = self.glide_secs.max(1e-4);
            let p = (1.0 - self.glide_left / dur).clamp(0.0, 1.0);
            lerp3(self.glide_from, self.glide_to, p)
        }
    }

    fn active_seg(&self, t: f32) -> ActiveSeg {
        let mut chosen: Option<&VocalSegment> = None;
        for s in &self.params.segments {
            if s.at <= t {
                chosen = Some(s);
            }
        }
        match chosen {
            Some(s) => ActiveSeg {
                at: s.at,
                dur: s.dur,
                kind: s.kind,
                vowel: s.vowel,
                place_hz: s.place_hz,
                voiced: s.voiced,
            },
            None => ActiveSeg {
                at: 0.0,
                dur: f32::INFINITY,
                kind: SegmentKind::Vowel,
                vowel: self.params.vowel,
                place_hz: 0.0,
                voiced: Some(1.0),
            },
        }
    }

    fn tick_comb(&mut self, src: f32, note_hz: f64, pitch_mult: f64, comb_mix: f32) -> f32 {
        let ratio = f64::from(self.params.comb.ratio).abs().max(1e-6);
        let f0 = (note_hz * pitch_mult * ratio).abs().max(1e-9);
        let delay = (f64::from(self.sample_rate) / f0).clamp(2.0, 4095.0);
        let i0 = delay.floor() as usize;
        let frac = (delay - i0 as f64) as f32;
        let n = COMB_MAX;
        let d0 = self.comb_buf[(self.comb_w + n - i0) % n];
        let d1 = self.comb_buf[(self.comb_w + n - i0 - 1) % n];
        let delayed = d0 * (1.0 - frac) + d1 * frac;

        let feedback = self.params.comb.feedback.clamp(0.0, 0.92);
        let damp = self.params.comb.damping.clamp(0.0, 1.0);
        self.comb_lp = damp * self.comb_lp + (1.0 - damp) * delayed;
        if !self.comb_lp.is_finite() {
            self.comb_lp = 0.0;
        }

        let mut comb_out = src + feedback * self.comb_lp;
        if !comb_out.is_finite() {
            comb_out = 0.0;
            self.comb_lp = 0.0;
            self.comb_buf.fill(0.0);
        }
        self.comb_buf[self.comb_w] = comb_out;
        self.comb_w = (self.comb_w + 1) % n;
        src * (1.0 - comb_mix) + comb_out * comb_mix
    }

    fn next_pm1(&mut self) -> f32 {
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        if (x as i32) < 0 {
            -1.0
        } else {
            1.0
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

    fn rms(buf: &[f32]) -> f32 {
        if buf.is_empty() {
            return 0.0;
        }
        let s: f32 = buf.iter().map(|x| x * x).sum();
        (s / buf.len() as f32).sqrt()
    }

    fn run(params: VocalParams, input: &[f32], note_hz: f64, pitch_mult: f64, sr: f32) -> Vec<f32> {
        let mut v = VocalRuntime::new(params, sr);
        v.note_on();
        input
            .iter()
            .map(|&x| v.tick(x, note_hz, pitch_mult))
            .collect()
    }

    #[test]
    fn bypass_is_identity() {
        let sr = 44_100.0;
        let input = sine(200.0, sr, 512);
        let y = run(VocalParams::default(), &input, 200.0, 1.0, sr);
        let err: f32 = input.iter().zip(&y).map(|(a, b)| (a - b).abs()).sum();
        assert!(err < 1e-9, "bypass error {err}");
    }

    #[test]
    fn flap_drops_f3_below_2000() {
        let a = vowel_hz(Vowel::A);
        let r = apply_kind_formants(a, SegmentKind::Flap, 1550.0);
        assert!(r.2 < 2000.0, "rhotic F3 {} should be well below 2000", r.2);
        assert!((r.2 - 1550.0).abs() < 1.0);
        assert!(r.2 < a.2 - 400.0);
        let def = apply_kind_formants(a, SegmentKind::Flap, 0.0);
        assert!((def.2 - RHOTIC_F3_HZ).abs() < 1.0);
    }

    #[test]
    fn formant_wobble_moves() {
        let sr = 44_100.0;
        let n = 4_096;
        let input = sine(850.0, sr, n);
        let base = VocalParams {
            mix: 1.0,
            comb: CombParams {
                mix: 0.0,
                ..CombParams::default()
            },
            vowel: Vowel::A,
            ..VocalParams::default()
        };
        let y0 = run(base.clone(), &input, 200.0, 1.0, sr);
        let wob = VocalParams {
            formant_wobble: 0.08,
            formant_wobble_hz: 5.5,
            ..base
        };
        let y1 = run(wob, &input, 200.0, 1.0, sr);
        let diff: f32 = y0.iter().zip(&y1).map(|(a, b)| (a - b).abs()).sum();
        assert!(
            diff > 1.0,
            "formant wobble did not change output (diff={diff})"
        );
    }

    #[test]
    fn formant_prefers_f2() {
        let sr = 44_100.0;
        let n = 8_192;
        let params = VocalParams {
            mix: 1.0,
            comb: CombParams {
                mix: 0.0,
                ..CombParams::default()
            },
            vowel: Vowel::A,
            ..VocalParams::default()
        };
        let mid = sine(1220.0, sr, n);
        let low = sine(80.0, sr, n);
        let y_mid = run(params.clone(), &mid, 200.0, 1.0, sr);
        let y_low = run(params, &low, 200.0, 1.0, sr);
        let rm = rms(&y_mid[n / 2..]);
        let rl = rms(&y_low[n / 2..]);
        assert!(
            rm > rl * 4.0,
            "F2=1220 should pass 1220 Hz much more than 80 Hz (mid={rm}, low={rl})"
        );
    }

    #[test]
    fn comb_reinforces_f0() {
        let sr = 44_100.0;
        let n = 8_192;
        let params = VocalParams {
            mix: 1.0,
            formant_resonance: 0.0,
            comb: CombParams {
                mix: 1.0,
                feedback: 0.85,
                damping: 0.0,
                ratio: 1.0,
            },
            vowel: Vowel::A,
            ..VocalParams::default()
        };
        let on = sine(200.0, sr, n);
        let off = sine(450.0, sr, n);
        let y_on = run(params.clone(), &on, 200.0, 1.0, sr);
        let y_off = run(params, &off, 200.0, 1.0, sr);
        let r_on = rms(&y_on[n / 2..]);
        let r_off = rms(&y_off[n / 2..]);
        assert!(
            r_on > r_off * 1.5,
            "comb at 200 Hz should prefer 200 over 450 (on={r_on}, off={r_off})"
        );
    }

    #[test]
    fn plosive_closure_then_burst() {
        let sr = 1000.0;
        let params = VocalParams {
            mix: 1.0,
            segments: vec![VocalSegment {
                at: 0.0,
                dur: 0.05,
                kind: SegmentKind::Plosive,
                vowel: Vowel::A,
                place_hz: 2000.0,
                voiced: None,
            }],
            ..VocalParams::default()
        };
        let input = vec![0.0; 51];
        let y = run(params, &input, 200.0, 1.0, sr);
        for (i, s) in y.iter().take(20).enumerate() {
            assert!(s.abs() < 1e-4, "closure leaked at {i}: {s}");
        }
        assert!(
            y[30..=50.min(y.len() - 1)].iter().any(|s| s.abs() > 0.01),
            "no burst after release"
        );
    }

    #[test]
    fn fricative_speaks_while_unvoiced() {
        let sr = 1000.0;
        let params = VocalParams {
            mix: 1.0,
            segments: vec![VocalSegment {
                at: 0.0,
                dur: 0.05,
                kind: SegmentKind::Fricative,
                vowel: Vowel::A,
                place_hz: 5500.0,
                voiced: None,
            }],
            ..VocalParams::default()
        };
        let input = vec![0.0; 40];
        let y = run(params, &input, 200.0, 1.0, sr);
        assert!(
            y.iter().any(|s| s.abs() > 0.01),
            "fricative produced silence"
        );
    }

    #[test]
    fn flap_stays_voiced() {
        let sr = 44_100.0;
        let n = (0.02 * sr) as usize;
        let params = VocalParams {
            mix: 1.0,
            comb: CombParams {
                mix: 0.0,
                ..CombParams::default()
            },
            segments: vec![VocalSegment {
                at: 0.0,
                dur: 0.04,
                kind: SegmentKind::Flap,
                vowel: Vowel::A,
                place_hz: 1800.0,
                voiced: None,
            }],
            ..VocalParams::default()
        };
        let w = std::f32::consts::TAU * 850.0 / sr;
        let input: Vec<f32> = (0..n).map(|i| (w * i as f32).sin()).collect();
        let y = run(params, &input, 200.0, 1.0, sr);
        let start = (0.005 * sr) as usize;
        assert!(
            y[start.min(y.len().saturating_sub(1))..]
                .iter()
                .any(|s| s.abs() > 0.01),
            "flap muted like a plosive"
        );
    }

    #[test]
    fn high_feedback_stays_finite() {
        let params = VocalParams {
            mix: 1.0,
            comb: CombParams {
                mix: 1.0,
                feedback: 0.92,
                damping: 0.35,
                ratio: 1.0,
            },
            ..VocalParams::default()
        };
        let mut v = VocalRuntime::new(params, 44_100.0);
        v.note_on();
        for i in 0..8_000 {
            let x = if i % 64 == 0 { 1.0 } else { 0.0 };
            let y = v.tick(x, 110.0, 1.0);
            assert!(y.is_finite(), "NaN/Inf at sample {i}");
        }
    }

    #[test]
    fn delay_shortens_when_pitch_rises() {
        let sr = 8_000.0;
        let n = 2_000;
        let params = VocalParams {
            mix: 1.0,
            formant_resonance: 0.0,
            comb: CombParams {
                mix: 1.0,
                feedback: 0.8,
                damping: 0.0,
                ratio: 1.0,
            },
            vowel: Vowel::A,
            ..VocalParams::default()
        };
        let mut impulse = vec![0.0; n];
        impulse[0] = 1.0;
        let y100 = run(params.clone(), &impulse, 100.0, 1.0, sr);
        let y200 = run(params, &impulse, 200.0, 1.0, sr);
        let peak_idx = |y: &[f32]| {
            y.iter()
                .enumerate()
                .skip(5)
                .max_by(|a, b| a.1.abs().partial_cmp(&b.1.abs()).unwrap())
                .map(|(i, _)| i)
                .unwrap()
        };
        let idx100 = peak_idx(&y100);
        let idx200 = peak_idx(&y200);
        assert!(idx200 > 4, "200 Hz peak too early ({idx200})");
        let rel = (idx100 as f32 - 2.0 * idx200 as f32).abs() / (2.0 * idx200 as f32).max(1.0);
        assert!(
            rel <= 0.15,
            "delay did not halve: idx100={idx100}, idx200={idx200}, rel={rel}"
        );
    }
}
