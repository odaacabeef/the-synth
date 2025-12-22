/// FreeVerb-style reverb implementation
/// Uses parallel comb filters and series all-pass filters

/// Single comb filter with damping
struct CombFilter {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
    damping: f32,
    filter_state: f32,
}

impl CombFilter {
    fn new(size: usize, feedback: f32, damping: f32) -> Self {
        Self {
            buffer: vec![0.0; size],
            index: 0,
            feedback,
            damping,
            filter_state: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let output = self.buffer[self.index];

        // One-pole lowpass filter for damping
        self.filter_state = output * (1.0 - self.damping) + self.filter_state * self.damping;

        // Feedback with damping
        self.buffer[self.index] = input + self.filter_state * self.feedback;

        // Advance circular buffer
        self.index = (self.index + 1) % self.buffer.len();

        output
    }

    fn set_damping(&mut self, damping: f32) {
        self.damping = damping.clamp(0.0, 1.0);
    }

    fn set_feedback(&mut self, feedback: f32) {
        self.feedback = feedback.clamp(0.0, 0.99);
    }

    fn clear(&mut self) {
        self.buffer.fill(0.0);
        self.filter_state = 0.0;
    }
}

/// All-pass filter for diffusion
struct AllPassFilter {
    buffer: Vec<f32>,
    index: usize,
    feedback: f32,
}

impl AllPassFilter {
    fn new(size: usize) -> Self {
        Self {
            buffer: vec![0.0; size],
            index: 0,
            feedback: 0.5,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let buffered = self.buffer[self.index];
        let output = -input + buffered;

        self.buffer[self.index] = input + buffered * self.feedback;

        self.index = (self.index + 1) % self.buffer.len();

        output
    }

    fn clear(&mut self) {
        self.buffer.fill(0.0);
    }
}

/// FreeVerb-style reverb
/// 8 parallel comb filters + 4 series all-pass filters
pub struct Reverb {
    // Parallel comb filters (different sizes for density)
    comb_filters: Vec<CombFilter>,

    // Series all-pass filters (for diffusion)
    allpass_filters: Vec<AllPassFilter>,

    // Mix parameters
    wet: f32,
    dry: f32,
    room_size: f32,
    damping: f32,
}

impl Reverb {
    /// Create new reverb with given sample rate
    pub fn new(sample_rate: f32) -> Self {
        // Comb filter delays (in samples, tuned for 44.1kHz)
        let scale = sample_rate / 44100.0;
        let comb_sizes = [
            (1557.0 * scale) as usize,
            (1617.0 * scale) as usize,
            (1491.0 * scale) as usize,
            (1422.0 * scale) as usize,
            (1277.0 * scale) as usize,
            (1356.0 * scale) as usize,
            (1188.0 * scale) as usize,
            (1116.0 * scale) as usize,
        ];

        // All-pass filter delays
        let allpass_sizes = [
            (225.0 * scale) as usize,
            (556.0 * scale) as usize,
            (441.0 * scale) as usize,
            (341.0 * scale) as usize,
        ];

        let initial_feedback = 0.84;
        let initial_damping = 0.2;

        let comb_filters: Vec<CombFilter> = comb_sizes
            .iter()
            .map(|&size| CombFilter::new(size, initial_feedback, initial_damping))
            .collect();

        let allpass_filters: Vec<AllPassFilter> = allpass_sizes
            .iter()
            .map(|&size| AllPassFilter::new(size))
            .collect();

        Self {
            comb_filters,
            allpass_filters,
            wet: 0.0,
            dry: 1.0,
            room_size: 0.5,
            damping: 0.5,
        }
    }

    /// Process single sample through reverb
    pub fn process(&mut self, input: f32) -> f32 {
        // Sum parallel comb filters
        let mut comb_sum = 0.0;
        for comb in &mut self.comb_filters {
            comb_sum += comb.process(input);
        }

        // Average the comb outputs
        let mut output = comb_sum / self.comb_filters.len() as f32;

        // Series all-pass filters for diffusion
        for allpass in &mut self.allpass_filters {
            output = allpass.process(output);
        }

        // Mix dry and wet signals
        self.dry * input + self.wet * output
    }

    /// Set wet/dry mix (0.0 = dry, 1.0 = wet)
    pub fn set_mix(&mut self, mix: f32) {
        let mix = mix.clamp(0.0, 1.0);
        self.wet = mix;
        self.dry = 1.0 - mix;
    }

    /// Set room size (0.0 = small, 1.0 = large)
    pub fn set_room_size(&mut self, size: f32) {
        self.room_size = size.clamp(0.0, 1.0);

        // Map room size to feedback (0.7 to 0.95)
        let feedback = 0.7 + self.room_size * 0.25;

        for comb in &mut self.comb_filters {
            comb.set_feedback(feedback);
        }
    }

    /// Set damping (0.0 = bright, 1.0 = dark)
    pub fn set_damping(&mut self, damping: f32) {
        self.damping = damping.clamp(0.0, 1.0);

        for comb in &mut self.comb_filters {
            comb.set_damping(self.damping);
        }
    }

    /// Get current wet/dry mix value
    #[allow(dead_code)]
    pub fn get_mix(&self) -> f32 {
        self.wet
    }

    /// Get current room size value
    #[allow(dead_code)]
    pub fn get_room_size(&self) -> f32 {
        self.room_size
    }

    /// Get current damping value
    #[allow(dead_code)]
    pub fn get_damping(&self) -> f32 {
        self.damping
    }

    /// Clear all delay buffers (reset reverb tail)
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        for comb in &mut self.comb_filters {
            comb.clear();
        }
        for allpass in &mut self.allpass_filters {
            allpass.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverb_creates() {
        let reverb = Reverb::new(44100.0);
        assert_eq!(reverb.get_mix(), 0.0);
    }

    #[test]
    fn test_reverb_processes() {
        let mut reverb = Reverb::new(44100.0);
        let output = reverb.process(1.0);
        assert!(output.is_finite());
    }

    #[test]
    fn test_reverb_parameters() {
        let mut reverb = Reverb::new(44100.0);

        reverb.set_mix(0.5);
        assert_eq!(reverb.get_mix(), 0.5);

        reverb.set_room_size(0.8);
        assert_eq!(reverb.get_room_size(), 0.8);

        reverb.set_damping(0.6);
        assert_eq!(reverb.get_damping(), 0.6);
    }
}
