use std::sync::{atomic::Ordering, Arc};
use crossbeam_channel::Receiver;

use super::{voice::Voice, parameters::SynthParameters};
use crate::types::events::SynthEvent;

/// Core synthesis engine
/// Runs in real-time audio thread - must be lock-free and allocation-free
pub struct SynthEngine {
    voice: Voice,
    parameters: Arc<SynthParameters>,
    event_rx: Receiver<SynthEvent>,
}

impl SynthEngine {
    /// Create new synthesis engine with MIDI event receiver
    pub fn new(
        sample_rate: f32,
        parameters: Arc<SynthParameters>,
        event_rx: Receiver<SynthEvent>,
    ) -> Self {
        Self {
            voice: Voice::new(sample_rate),
            parameters,
            event_rx,
        }
    }

    /// Process audio callback - fills output buffer with samples
    /// This runs in real-time audio thread - must be fast and lock-free
    pub fn process(&mut self, output: &mut [f32]) {
        // Process all pending MIDI events (non-blocking)
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                SynthEvent::NoteOn { frequency, velocity: _ } => {
                    // For monophonic synth, just trigger the single voice
                    self.voice.note_on(frequency);
                }
                SynthEvent::NoteOff { note: _ } => {
                    // Release the voice
                    self.voice.note_off();
                }
            }
        }

        // Read ADSR parameters from atomics (non-blocking)
        let attack = self.parameters.attack.load(Ordering::Relaxed);
        let decay = self.parameters.decay.load(Ordering::Relaxed);
        let sustain = self.parameters.sustain.load(Ordering::Relaxed);
        let release = self.parameters.release.load(Ordering::Relaxed);
        self.voice.set_adsr(attack, decay, sustain, release);

        // Generate samples
        for sample in output.iter_mut() {
            *sample = self.voice.next_sample();
        }
    }
}
