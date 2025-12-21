use std::sync::{atomic::Ordering, Arc};
use super::{oscillator::Oscillator, parameters::SynthParameters};

/// Core synthesis engine
/// Runs in real-time audio thread - must be lock-free and allocation-free
pub struct SynthEngine {
    oscillator: Oscillator,
    parameters: Arc<SynthParameters>,
    sample_rate: f32,
}

impl SynthEngine {
    /// Create new synthesis engine
    pub fn new(sample_rate: f32, parameters: Arc<SynthParameters>) -> Self {
        Self {
            oscillator: Oscillator::new(sample_rate),
            parameters,
            sample_rate,
        }
    }

    /// Process audio callback - fills output buffer with samples
    /// This runs in real-time audio thread - must be fast and lock-free
    pub fn process(&mut self, output: &mut [f32]) {
        // Read current frequency from atomic parameters (non-blocking)
        let frequency = self.parameters.frequency.load(Ordering::Relaxed);
        self.oscillator.set_frequency(frequency);

        // Generate samples
        for sample in output.iter_mut() {
            *sample = self.oscillator.next_sample() * 0.2; // Reduce volume to 20%
        }
    }
}
