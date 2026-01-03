/// Simple one-pole low-pass filter
/// Efficient real-time filtering using exponential smoothing
pub struct OnePoleFilter {
    cutoff: f32,
    coefficient: f32,
    previous_output: f32,
    sample_rate: f32,
}

impl OnePoleFilter {
    /// Create new low-pass filter
    ///
    /// # Arguments
    /// * `sample_rate` - Audio sample rate in Hz
    /// * `cutoff_hz` - Filter cutoff frequency in Hz
    pub fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        let mut filter = Self {
            cutoff: cutoff_hz,
            coefficient: 0.0,
            previous_output: 0.0,
            sample_rate,
        };
        filter.update_coefficient();
        filter
    }

    /// Set filter cutoff frequency
    pub fn set_cutoff(&mut self, cutoff_hz: f32) {
        self.cutoff = cutoff_hz;
        self.update_coefficient();
    }

    /// Update filter coefficient based on cutoff frequency
    fn update_coefficient(&mut self) {
        let omega = 2.0 * std::f32::consts::PI * self.cutoff / self.sample_rate;
        self.coefficient = 1.0 - (-omega).exp();
    }

    /// Process one sample through the low-pass filter
    pub fn process(&mut self, input: f32) -> f32 {
        self.previous_output += self.coefficient * (input - self.previous_output);
        self.previous_output
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.previous_output = 0.0;
    }
}

/// High-pass filter implemented as input minus low-pass
pub struct HighPassFilter {
    low_pass: OnePoleFilter,
}

impl HighPassFilter {
    /// Create new high-pass filter
    ///
    /// # Arguments
    /// * `sample_rate` - Audio sample rate in Hz
    /// * `cutoff_hz` - Filter cutoff frequency in Hz
    pub fn new(sample_rate: f32, cutoff_hz: f32) -> Self {
        Self {
            low_pass: OnePoleFilter::new(sample_rate, cutoff_hz),
        }
    }

    /// Set filter cutoff frequency
    pub fn set_cutoff(&mut self, cutoff_hz: f32) {
        self.low_pass.set_cutoff(cutoff_hz);
    }

    /// Process one sample through the high-pass filter
    pub fn process(&mut self, input: f32) -> f32 {
        let low = self.low_pass.process(input);
        input - low // High = Input - Low
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.low_pass.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lowpass_dc() {
        let mut filter = OnePoleFilter::new(44100.0, 1000.0);

        // Feed DC signal (1.0)
        for _ in 0..100 {
            filter.process(1.0);
        }

        // Should converge to 1.0
        let output = filter.process(1.0);
        assert!((output - 1.0).abs() < 0.01, "DC response should be ~1.0");
    }

    #[test]
    fn test_lowpass_attenuates() {
        let mut filter = OnePoleFilter::new(44100.0, 100.0);

        // Process alternating signal (simulates high frequency)
        let mut outputs = Vec::new();
        for i in 0..10 {
            let input = if i % 2 == 0 { 1.0 } else { -1.0 };
            outputs.push(filter.process(input));
        }

        // Later outputs should be smaller in magnitude (attenuated)
        let last_output = outputs.last().unwrap().abs();
        assert!(last_output < 0.5, "High frequency should be attenuated");
    }

    #[test]
    fn test_highpass_blocks_dc() {
        let mut filter = HighPassFilter::new(44100.0, 1000.0);

        // Feed DC signal
        for _ in 0..100 {
            filter.process(1.0);
        }

        // Should converge to ~0.0 (DC blocked)
        let output = filter.process(1.0);
        assert!(output.abs() < 0.1, "DC should be blocked: {}", output);
    }

    #[test]
    fn test_filter_reset() {
        let mut filter = OnePoleFilter::new(44100.0, 1000.0);

        // Process some samples
        for _ in 0..10 {
            filter.process(1.0);
        }

        // Reset
        filter.reset();

        // State should be cleared
        assert_eq!(filter.previous_output, 0.0);
    }
}
