use std::sync::{atomic::Ordering, Arc};
use super::{voice::Voice, parameters::SynthParameters};

/// Core synthesis engine
/// Runs in real-time audio thread - must be lock-free and allocation-free
pub struct SynthEngine {
    voice: Voice,
    parameters: Arc<SynthParameters>,
    sample_count: u64,
    note_triggered: bool,
}

impl SynthEngine {
    /// Create new synthesis engine
    pub fn new(sample_rate: f32, parameters: Arc<SynthParameters>) -> Self {
        Self {
            voice: Voice::new(sample_rate),
            parameters,
            sample_count: 0,
            note_triggered: false,
        }
    }

    /// Process audio callback - fills output buffer with samples
    /// This runs in real-time audio thread - must be fast and lock-free
    pub fn process(&mut self, output: &mut [f32]) {
        // Read ADSR parameters from atomics (non-blocking)
        let attack = self.parameters.attack.load(Ordering::Relaxed);
        let decay = self.parameters.decay.load(Ordering::Relaxed);
        let sustain = self.parameters.sustain.load(Ordering::Relaxed);
        let release = self.parameters.release.load(Ordering::Relaxed);
        self.voice.set_adsr(attack, decay, sustain, release);

        // Hardcoded test: Trigger a note at start, release after 0.5 seconds
        // This demonstrates the ADSR envelope working
        if !self.note_triggered && self.sample_count == 0 {
            let frequency = self.parameters.frequency.load(Ordering::Relaxed);
            self.voice.note_on(frequency);
            self.note_triggered = true;
        }

        // Release note after 0.5 seconds (for testing envelope)
        let sample_rate = output.len() as f32 / 0.01; // Approximate sample rate
        if self.note_triggered && self.sample_count == (sample_rate * 0.5) as u64 {
            self.voice.note_off();
        }

        // Generate samples
        for sample in output.iter_mut() {
            *sample = self.voice.next_sample();
            self.sample_count += 1;
        }
    }
}
