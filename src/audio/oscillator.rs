use std::f32::consts::PI;

/// Basic oscillator with phase accumulation
/// Generates sine wave for Phase 1 (other waveforms in Phase 2)
pub struct Oscillator {
    /// Current phase position (0.0 to 1.0)
    phase: f32,
    /// Phase increment per sample (frequency / sample_rate)
    phase_delta: f32,
    /// Current frequency in Hz
    frequency: f32,
    /// Sample rate in Hz
    sample_rate: f32,
}

impl Oscillator {
    /// Create a new oscillator with given sample rate
    pub fn new(sample_rate: f32) -> Self {
        let mut osc = Self {
            phase: 0.0,
            phase_delta: 0.0,
            frequency: 440.0,
            sample_rate,
        };
        osc.update_phase_delta();
        osc
    }

    /// Set the oscillator frequency
    pub fn set_frequency(&mut self, freq: f32) {
        self.frequency = freq;
        self.update_phase_delta();
    }

    /// Update phase delta based on current frequency
    fn update_phase_delta(&mut self) {
        self.phase_delta = self.frequency / self.sample_rate;
    }

    /// Generate next sample and advance phase
    pub fn next_sample(&mut self) -> f32 {
        // Generate sine wave from phase
        let output = (self.phase * 2.0 * PI).sin();

        // Advance phase and wrap around
        self.phase += self.phase_delta;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        output
    }

    /// Reset phase to zero
    pub fn reset(&mut self) {
        self.phase = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oscillator_creates() {
        let osc = Oscillator::new(44100.0);
        assert_eq!(osc.frequency, 440.0);
    }

    #[test]
    fn test_frequency_update() {
        let mut osc = Oscillator::new(44100.0);
        osc.set_frequency(880.0);
        assert_eq!(osc.frequency, 880.0);
    }
}
