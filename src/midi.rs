/// Equal-tempered MIDI note to frequency. MIDI 69 = A4 = 440 Hz.
pub fn midi_to_hz(note: u8) -> f64 {
    440.0 * 2f64.powf((f64::from(note) - 69.0) / 12.0)
}

/// Frequency to the nearest MIDI note number (clamped to 0–127).
pub fn hz_to_midi(hz: f64) -> u8 {
    if hz <= 0.0 {
        return 0;
    }
    let n = 69.0 + 12.0 * (hz / 440.0).log2();
    n.round().clamp(0.0, 127.0) as u8
}

/// Cents to a frequency multiplier.
pub fn cents_to_ratio(cents: f64) -> f64 {
    2f64.powf(cents / 1200.0)
}

/// Semitones to a frequency multiplier.
pub fn semitones_to_ratio(semitones: f64) -> f64 {
    2f64.powf(semitones / 12.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_is_440() {
        assert!((midi_to_hz(69) - 440.0).abs() < 1e-9);
    }

    #[test]
    fn c4_is_middle_c() {
        let hz = midi_to_hz(60);
        assert!((hz - 261.625565).abs() < 1e-4);
    }

    #[test]
    fn octave_doubles() {
        assert!((midi_to_hz(81) / midi_to_hz(69) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn cents_1200_is_octave() {
        assert!((cents_to_ratio(1200.0) - 2.0).abs() < 1e-12);
    }
}
