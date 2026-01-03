use crossbeam_channel::Receiver;

use super::{types::DrumType, voice::DrumVoice};
use crate::types::events::SynthEvent;

/// Drum engine - manages drum voices and note mapping
/// Each drum engine responds to a specific MIDI note
pub struct DrumEngine {
    voice: DrumVoice,
    trigger_note: u8,        // MIDI note that triggers this drum
    midi_channel_filter: u8, // MIDI channel filter (0-15 or 255 for omni)
    event_rx: Receiver<SynthEvent>,
}

impl DrumEngine {
    /// Create new drum engine
    ///
    /// # Arguments
    /// * `drum_type` - Type of drum (Kick, Snare, Hat)
    /// * `trigger_note` - MIDI note number that triggers this drum (0-127)
    /// * `sample_rate` - Audio sample rate in Hz
    /// * `midi_channel_filter` - MIDI channel to listen to (0-15, or 255 for omni)
    /// * `event_rx` - Channel receiver for MIDI events
    pub fn new(
        drum_type: DrumType,
        trigger_note: u8,
        sample_rate: f32,
        midi_channel_filter: u8,
        event_rx: Receiver<SynthEvent>,
    ) -> Self {
        Self {
            voice: DrumVoice::new(drum_type, sample_rate),
            trigger_note,
            midi_channel_filter,
            event_rx,
        }
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
            None => true, // AllNotesOff affects all
        }
    }

    /// Get voice states (for UI compatibility - drums don't have polyphonic voices)
    pub fn voice_states(&self) -> [Option<u8>; 16] {
        // Drums are monophonic, so we only use the first slot
        let mut states = [None; 16];
        if self.voice.is_active() {
            states[0] = Some(self.trigger_note);
        }
        states
    }

    /// Process audio callback - fills output buffer with samples
    /// This runs in real-time audio thread - must be fast and lock-free
    pub fn process(&mut self, output: &mut [f32]) {
        // Process all pending MIDI events
        while let Ok(event) = self.event_rx.try_recv() {
            if !self.should_process_event(&event) {
                continue;
            }

            match event {
                SynthEvent::NoteOn { note, .. } => {
                    // Check if this note matches our trigger note
                    if note == self.trigger_note {
                        self.voice.trigger();
                    }
                }
                SynthEvent::NoteOff { .. } => {
                    // Drums ignore note off (one-shot)
                }
                SynthEvent::AllNotesOff { .. } => {
                    // Could implement immediate muting here if desired
                    // For now, let drums naturally finish
                }
            }
        }

        // Generate audio
        output.fill(0.0);
        if self.voice.is_active() {
            for sample in output.iter_mut() {
                *sample = self.voice.next_sample();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;

    #[test]
    fn test_drum_engine_creates() {
        let (_tx, rx) = unbounded();
        let engine = DrumEngine::new(DrumType::Kick, 36, 44100.0, 9, rx);

        let states = engine.voice_states();
        assert_eq!(states[0], None); // Not active initially
    }

    #[test]
    fn test_drum_engine_triggers_on_correct_note() {
        let (tx, rx) = unbounded();
        let mut engine = DrumEngine::new(DrumType::Kick, 36, 44100.0, 9, rx);

        // Send note on for MIDI note 36 (C2 = 65.40639 Hz)
        tx.send(SynthEvent::note_on(9, 36, 65.40639, 1.0)).unwrap();

        // Process a small buffer (engine will pull events during processing)
        let mut buffer = vec![0.0f32; 512];
        engine.process(&mut buffer);

        // Check that audio was generated (buffer should have non-zero samples)
        let has_audio = buffer.iter().any(|&s| s.abs() > 0.0001);
        assert!(has_audio, "Drum should generate audio after trigger");
    }

    #[test]
    fn test_drum_engine_ignores_wrong_note() {
        let (tx, rx) = unbounded();
        let mut engine = DrumEngine::new(DrumType::Kick, 36, 44100.0, 9, rx);

        // Send note on for different note (MIDI note 38 = D1)
        let _ = tx.send(SynthEvent::note_on(9, 38, 38.89, 1.0));

        // Process
        let mut buffer = vec![0.0f32; 64];
        engine.process(&mut buffer);

        // Voice should NOT be active
        assert!(!engine.voice.is_active());
    }

    #[test]
    fn test_drum_engine_ignores_wrong_channel() {
        let (tx, rx) = unbounded();
        let mut engine = DrumEngine::new(DrumType::Kick, 36, 44100.0, 9, rx); // Channel 9

        // Send note on correct note but wrong channel
        let _ = tx.send(SynthEvent::note_on(0, 36, 32.7, 1.0)); // Channel 0

        // Process
        let mut buffer = vec![0.0f32; 64];
        engine.process(&mut buffer);

        // Voice should NOT be active
        assert!(!engine.voice.is_active());
    }
}
