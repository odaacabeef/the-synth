/// MIDI note to frequency conversion using equal temperament
/// A440 tuning: MIDI note 69 = 440 Hz

/// Convert MIDI note number to frequency in Hz
/// Uses equal temperament: f = 440 * 2^((n-69)/12)
pub fn midi_note_to_frequency(note: u8) -> f32 {
    const A4: f32 = 440.0;
    const A4_MIDI: i32 = 69;

    let semitones = note as i32 - A4_MIDI;
    A4 * 2.0_f32.powf(semitones as f32 / 12.0)
}

/// Convert MIDI velocity (0-127) to normalized amplitude (0.0-1.0)
pub fn midi_velocity_to_amplitude(velocity: u8) -> f32 {
    (velocity as f32 / 127.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_a4_conversion() {
        let freq = midi_note_to_frequency(69);
        assert!((freq - 440.0).abs() < 0.01);
    }

    #[test]
    fn test_c4_middle_c() {
        let freq = midi_note_to_frequency(60);
        assert!((freq - 261.63).abs() < 0.01); // Middle C
    }

    #[test]
    fn test_octave_doubling() {
        let a3 = midi_note_to_frequency(57); // A3
        let a4 = midi_note_to_frequency(69); // A4
        let a5 = midi_note_to_frequency(81); // A5

        assert!((a4 / a3 - 2.0).abs() < 0.01); // Octave doubles frequency
        assert!((a5 / a4 - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_velocity_conversion() {
        assert_eq!(midi_velocity_to_amplitude(0), 0.0);
        assert_eq!(midi_velocity_to_amplitude(127), 1.0);
        assert!((midi_velocity_to_amplitude(64) - 0.504).abs() < 0.01);
    }
}
