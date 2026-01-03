use std::sync::{atomic::Ordering, Arc};
use crossbeam_channel::Receiver;

use super::{voice_pool::VoicePool, parameters::SynthParameters};
use crate::types::events::SynthEvent;

/// Core synthesis engine
/// Runs in real-time audio thread - must be lock-free and allocation-free
pub struct SynthEngine {
    voice_pool: VoicePool,
    parameters: Arc<SynthParameters>,
    event_rx: Receiver<SynthEvent>,
    midi_channel_filter: u8, // 0-15 for specific channel, 255 for omni
}

impl SynthEngine {
    /// Create new synthesis engine with specific MIDI channel filter
    pub fn new_with_channel(
        sample_rate: f32,
        parameters: Arc<SynthParameters>,
        event_rx: Receiver<SynthEvent>,
        midi_channel_filter: u8,
    ) -> Self {
        Self {
            voice_pool: VoicePool::new(sample_rate),
            parameters,
            event_rx,
            midi_channel_filter,
        }
    }

    /// Get the state of all voices (note number or None for each voice)
    pub fn voice_states(&self) -> [Option<u8>; 16] {
        self.voice_pool.voice_states()
    }

    /// Check if this engine should process the given event based on channel filtering
    fn should_process_event(&self, event: &SynthEvent) -> bool {
        // Omni mode (255) accepts all channels
        if self.midi_channel_filter == 255 {
            return true;
        }

        // Check if event channel matches our filter
        match event.channel() {
            Some(ch) => ch == self.midi_channel_filter,
            None => true, // AllNotesOff with no channel affects all engines
        }
    }

    /// Process audio callback - fills output buffer with samples
    /// This runs in real-time audio thread - must be fast and lock-free
    pub fn process(&mut self, output: &mut [f32]) {
        // Process all pending MIDI events (non-blocking)
        while let Ok(event) = self.event_rx.try_recv() {
            // Filter by channel (255 = omni, accepts all)
            if !self.should_process_event(&event) {
                continue;
            }

            match event {
                SynthEvent::NoteOn { note, frequency, .. } => {
                    // Use the note number from the event for voice tracking
                    self.voice_pool.note_on(note, frequency);
                }
                SynthEvent::NoteOff { note, .. } => {
                    // Release the matching voice
                    self.voice_pool.note_off(note);
                }
                SynthEvent::AllNotesOff { .. } => {
                    // MIDI panic - release all voices
                    self.voice_pool.all_notes_off();
                }
            }
        }

        // Read ADSR parameters from atomics (non-blocking)
        let attack = self.parameters.attack.load(Ordering::Relaxed);
        let decay = self.parameters.decay.load(Ordering::Relaxed);
        let sustain = self.parameters.sustain.load(Ordering::Relaxed);
        let release = self.parameters.release.load(Ordering::Relaxed);
        self.voice_pool.set_adsr(attack, decay, sustain, release);

        // Read waveform parameter
        let waveform_u8 = self.parameters.waveform.load(Ordering::Relaxed);
        let waveform = crate::types::waveform::Waveform::from_u8(waveform_u8);
        self.voice_pool.set_waveform(waveform);

        // Process all voices and mix to output
        self.voice_pool.process(output);
    }
}
