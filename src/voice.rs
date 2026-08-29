use crate::algorithm::Algorithm;
use crate::midi::semitones_to_ratio;
use crate::operator::Operator;
use crate::preset::Preset;
use std::f64::consts::TAU;

/// One-note FM voice. Offline renderer drives this sample-by-sample.
pub struct Voice {
    ops: [Operator; 4],
    algorithm: Algorithm,
    feedback: f32,
    feedback_op: usize,
    gain: f32,
    pitch_start: f64,
    pitch_end: f64,
    pitch_curve: f64,
    lfo_rate: f64,
    lfo_depth_cents: f64,
    mod_start: f32,
    mod_end: f32,
    note_hz: f64,
    sample_rate: f64,
    time: f64,
    duration: f64,
}

impl Voice {
    pub fn new(preset: &Preset, sample_rate: u32) -> Self {
        let sr = sample_rate as f32;
        let params = preset.operator_array();
        let ops = [
            Operator::new(params[0].clone(), sr),
            Operator::new(params[1].clone(), sr),
            Operator::new(params[2].clone(), sr),
            Operator::new(params[3].clone(), sr),
        ];
        let fb_op = usize::from(preset.feedback_op.saturating_sub(1)).min(3);
        Self {
            ops,
            algorithm: preset.algorithm,
            feedback: preset.feedback,
            feedback_op: fb_op,
            gain: preset.gain,
            pitch_start: preset.pitch.start_semitones,
            pitch_end: preset.pitch.end_semitones,
            pitch_curve: f64::from(preset.pitch.curve),
            lfo_rate: preset.lfo.rate_hz,
            lfo_depth_cents: preset.lfo.depth_cents,
            mod_start: preset.mod_sweep.start,
            mod_end: preset.mod_sweep.end,
            note_hz: 440.0,
            sample_rate: f64::from(sample_rate),
            time: 0.0,
            duration: 1.0,
        }
    }

    pub fn set_duration(&mut self, secs: f64) {
        self.duration = secs.max(1.0 / self.sample_rate);
    }

    pub fn note_on(&mut self, note_hz: f64, velocity: f32) {
        self.note_hz = note_hz.max(0.01);
        self.time = 0.0;
        for op in &mut self.ops {
            op.note_on(velocity);
        }
    }

    pub fn note_off(&mut self) {
        for op in &mut self.ops {
            op.note_off();
        }
    }

    pub fn is_idle(&self) -> bool {
        self.ops.iter().all(|op| op.is_idle())
    }

    pub fn max_release_secs(&self) -> f32 {
        self.ops
            .iter()
            .map(Operator::release_secs)
            .fold(0.0f32, f32::max)
    }

    pub fn max_sustain(&self) -> f32 {
        self.ops
            .iter()
            .map(Operator::sustain)
            .fold(0.0f32, f32::max)
    }

    pub fn tick(&mut self) -> f32 {
        let t = (self.time / self.duration).clamp(0.0, 1.0);
        let shaped = shape(t, self.pitch_curve);
        let pitch_st = self.pitch_start + (self.pitch_end - self.pitch_start) * shaped;
        let lfo = if self.lfo_depth_cents.abs() > 1e-6 && self.lfo_rate > 0.0 {
            (TAU * self.lfo_rate * self.time).sin() * self.lfo_depth_cents
        } else {
            0.0
        };
        let pitch_mult = semitones_to_ratio(pitch_st + lfo / 100.0);
        for op in &mut self.ops {
            op.update_frequency(self.note_hz, pitch_mult);
        }

        let mod_gain = self.mod_start + (self.mod_end - self.mod_start) * t as f32;
        let fb = self.ops[self.feedback_op].feedback_source() * self.feedback * mod_gain;
        let mix = tick_algorithm(
            &mut self.ops,
            self.algorithm,
            fb,
            mod_gain,
            self.feedback_op,
        );

        self.time += 1.0 / self.sample_rate;
        mix * self.gain
    }
}

fn shape(t: f64, curve: f64) -> f64 {
    if curve.abs() < 1e-6 {
        t
    } else {
        let expc = curve.exp();
        ((curve * t).exp() - 1.0) / (expc - 1.0)
    }
}

fn tick_algorithm(
    ops: &mut [Operator; 4],
    algorithm: Algorithm,
    feedback: f32,
    mod_gain: f32,
    feedback_op: usize,
) -> f32 {
    // Higher-numbered ops run first so stacks (4→3→2→1) see fresh modulators.
    let m = |x: f32| x * mod_gain;
    let fb_for = |idx: usize, fb: f32| if idx == feedback_op { fb } else { 0.0 };

    match algorithm {
        Algorithm::Serial => {
            let o4 = ops[3].tick(fb_for(3, feedback));
            let o3 = ops[2].tick(fb_for(2, feedback) + m(o4));
            let o2 = ops[1].tick(fb_for(1, feedback) + m(o3));
            ops[0].tick(fb_for(0, feedback) + m(o2))
        }
        Algorithm::ParallelMod => {
            let o4 = ops[3].tick(fb_for(3, feedback));
            let o3 = ops[2].tick(fb_for(2, feedback));
            let o2 = ops[1].tick(fb_for(1, feedback) + m(o4 + o3));
            ops[0].tick(fb_for(0, feedback) + m(o2))
        }
        Algorithm::DoubleMod => {
            let o4 = ops[3].tick(fb_for(3, feedback));
            let o3 = ops[2].tick(fb_for(2, feedback) + m(o4));
            let o2 = ops[1].tick(fb_for(1, feedback));
            ops[0].tick(fb_for(0, feedback) + m(o3 + o2))
        }
        Algorithm::SharedMod => {
            let o4 = ops[3].tick(fb_for(3, feedback));
            let o3 = ops[2].tick(fb_for(2, feedback) + m(o4));
            let o2 = ops[1].tick(fb_for(1, feedback) + m(o4));
            ops[0].tick(fb_for(0, feedback) + m(o3 + o2))
        }
        Algorithm::DualStack => {
            let o4 = ops[3].tick(fb_for(3, feedback));
            let o3 = ops[2].tick(fb_for(2, feedback) + m(o4));
            let o2 = ops[1].tick(fb_for(1, feedback));
            let o1 = ops[0].tick(fb_for(0, feedback) + m(o2));
            0.5 * (o3 + o1)
        }
        Algorithm::TripleCarrier => {
            let o4 = ops[3].tick(fb_for(3, feedback));
            let o3 = ops[2].tick(fb_for(2, feedback) + m(o4));
            let o2 = ops[1].tick(fb_for(1, feedback) + m(o4));
            let o1 = ops[0].tick(fb_for(0, feedback) + m(o4));
            (o3 + o2 + o1) / 3.0
        }
        Algorithm::StackPlusCarriers => {
            let o4 = ops[3].tick(fb_for(3, feedback));
            let o3 = ops[2].tick(fb_for(2, feedback) + m(o4));
            let o2 = ops[1].tick(fb_for(1, feedback));
            let o1 = ops[0].tick(fb_for(0, feedback));
            (o3 + o2 + o1) / 3.0
        }
        Algorithm::AllCarriers => {
            let o4 = ops[3].tick(fb_for(3, feedback));
            let o3 = ops[2].tick(fb_for(2, feedback));
            let o2 = ops[1].tick(fb_for(1, feedback));
            let o1 = ops[0].tick(fb_for(0, feedback));
            0.25 * (o4 + o3 + o2 + o1)
        }
    }
}
