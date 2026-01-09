/// Monophonic CV voice with note priority stack and glide
pub struct CVVoice {
    sample_rate: f32,

    // Note stack for last-note priority (pre-allocated)
    note_stack: Vec<u8>,
    note_stack_capacity: usize,

    // Current pitch state
    current_pitch: f32, // Current CV pitch in volts
    target_pitch: f32,  // Target CV pitch in volts

    // Transpose in semitones
    transpose: i8,

    // Glide state
    glide_time: f32,       // Glide time in seconds
    glide_step: f32,       // Fixed step size per sample for linear glide
    glide_samples_left: f32, // Samples remaining in glide

    // Gate state
    gate_high: bool,
}

impl CVVoice {
    pub fn new(sample_rate: f32) -> Self {
        const MAX_NOTES: usize = 16; // Pre-allocate for max polyphony

        Self {
            sample_rate,
            note_stack: Vec::with_capacity(MAX_NOTES),
            note_stack_capacity: MAX_NOTES,
            current_pitch: 0.0,
            target_pitch: 0.0,
            transpose: 0,
            glide_time: 0.0,
            glide_step: 0.0,
            glide_samples_left: 0.0,
            gate_high: false,
        }
    }

    /// Convert MIDI note to CV voltage (1V/octave, C4 = 0V)
    /// Scaled for -10V to +10V audio interface range (normalized -1.0 to +1.0)
    fn note_to_voltage(note: u8, transpose: i8) -> f32 {
        let transposed_note = (note as i16 + transpose as i16).clamp(0, 127);
        (transposed_note as f32 - 60.0) / 120.0
    }

    /// Handle note on event
    pub fn note_on(&mut self, note: u8) {
        // Add to note stack if not already present
        if !self.note_stack.contains(&note) {
            if self.note_stack.len() < self.note_stack_capacity {
                self.note_stack.push(note);
            } else {
                // Stack full - replace oldest note (shouldn't happen with capacity 16)
                self.note_stack[0] = note;
            }
        }

        // Update target pitch to new note (with current transpose)
        self.target_pitch = Self::note_to_voltage(note, self.transpose);

        // If gate is off, jump immediately to target (no glide on first note)
        if !self.gate_high {
            self.current_pitch = self.target_pitch;
            self.glide_samples_left = 0.0;
        } else {
            // Calculate linear glide step for legato notes
            self.start_glide();
        }

        // Gate on
        self.gate_high = true;
    }

    /// Handle note off event
    pub fn note_off(&mut self, note: u8) {
        // Remove from note stack
        self.note_stack.retain(|&n| n != note);

        // If stack is empty, gate off
        if self.note_stack.is_empty() {
            self.gate_high = false;
            return;
        }

        // Otherwise, switch to most recent note in stack (last-note priority)
        if let Some(&last_note) = self.note_stack.last() {
            self.target_pitch = Self::note_to_voltage(last_note, self.transpose);
            self.start_glide();
        }
    }

    /// Handle all notes off
    pub fn all_notes_off(&mut self) {
        self.note_stack.clear();
        self.gate_high = false;
    }

    /// Set glide time
    pub fn set_glide_time(&mut self, time: f32) {
        self.glide_time = time;
    }

    /// Set transpose and update current note pitch if held
    pub fn set_transpose(&mut self, transpose: i8) {
        self.transpose = transpose;

        // If a note is currently held, recalculate its target pitch with new transpose
        if let Some(&current_note) = self.note_stack.last() {
            self.target_pitch = Self::note_to_voltage(current_note, self.transpose);
            // Optionally start a glide to the new transposed pitch
            self.start_glide();
        }
    }

    /// Start a new glide to the current target pitch
    fn start_glide(&mut self) {
        let distance = self.target_pitch - self.current_pitch;

        if self.glide_time <= 0.0 || distance.abs() < 0.0001 {
            // Instant glide or already at target
            self.current_pitch = self.target_pitch;
            self.glide_step = 0.0;
            self.glide_samples_left = 0.0;
        } else {
            // Calculate linear glide: distance / time_in_samples
            let total_samples = self.glide_time * self.sample_rate;
            self.glide_step = distance / total_samples;
            self.glide_samples_left = total_samples;
        }
    }

    /// Generate next pitch CV sample
    pub fn next_pitch_sample(&mut self) -> f32 {
        // Apply linear glide if in progress
        if self.glide_samples_left > 0.0 {
            self.current_pitch += self.glide_step;
            self.glide_samples_left -= 1.0;

            // Snap to target when done
            if self.glide_samples_left <= 0.0 {
                self.current_pitch = self.target_pitch;
            }
        }

        self.current_pitch
    }

    /// Generate next gate CV sample
    pub fn next_gate_sample(&self) -> f32 {
        if self.gate_high {
            0.8 // 8V scaled to -10V to +10V range (0.8 = 8V)
        } else {
            0.0
        }
    }

    /// Get current note (if any)
    pub fn current_note(&self) -> Option<u8> {
        self.note_stack.last().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_to_voltage() {
        assert_eq!(CVVoice::note_to_voltage(60, 0), 0.0); // C4 = 0V (normalized)
        assert!((CVVoice::note_to_voltage(72, 0) - 0.1).abs() < 0.001); // C5 = 1V = 0.1 normalized
        assert!((CVVoice::note_to_voltage(48, 0) + 0.1).abs() < 0.001); // C3 = -1V = -0.1 normalized
        assert!((CVVoice::note_to_voltage(69, 0) - 0.075).abs() < 0.001); // A4 = 0.75V = 0.075 normalized

        // Test transpose
        assert!((CVVoice::note_to_voltage(60, 12) - 0.1).abs() < 0.001); // C4 + 12 = C5 = 1V = 0.1 normalized
        assert!((CVVoice::note_to_voltage(60, -12) + 0.1).abs() < 0.001); // C4 - 12 = C3 = -1V = -0.1 normalized
    }

    #[test]
    fn test_note_priority() {
        let mut voice = CVVoice::new(44100.0);

        // First note
        voice.note_on(60); // C4
        assert_eq!(voice.current_note(), Some(60));

        // Second note (should switch)
        voice.note_on(64); // E4
        assert_eq!(voice.current_note(), Some(64));

        // Release second note (should return to first)
        voice.note_off(64);
        assert_eq!(voice.current_note(), Some(60));

        // Release first note (gate off)
        voice.note_off(60);
        assert_eq!(voice.current_note(), None);
    }

    #[test]
    fn test_glide() {
        let mut voice = CVVoice::new(44100.0);
        voice.set_glide_time(0.1); // 100ms glide

        voice.note_on(60); // C4 = 0V
        assert_eq!(voice.current_pitch, 0.0);

        voice.note_on(72); // C5 = 1V = 0.1 normalized
        assert!((voice.target_pitch - 0.1).abs() < 0.001);

        // Should glide linearly over 100ms (4410 samples at 44.1kHz)
        for _ in 0..4410 {
            voice.next_pitch_sample();
        }

        // Should reach target exactly with linear glide
        assert!((voice.current_pitch - 0.1).abs() < 0.0001);
    }

    #[test]
    fn test_gate() {
        let mut voice = CVVoice::new(44100.0);

        assert_eq!(voice.next_gate_sample(), 0.0); // Gate low

        voice.note_on(60);
        assert_eq!(voice.next_gate_sample(), 0.8); // Gate high (8V)

        voice.note_off(60);
        assert_eq!(voice.next_gate_sample(), 0.0); // Gate low
    }
}
