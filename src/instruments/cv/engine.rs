use crossbeam_channel::Receiver;
use std::sync::{atomic::Ordering, Arc};

use super::{parameters::CVParameters, voice::CVVoice};
use crate::types::events::SynthEvent;

/// CV output engine - generates control voltages for modular synths
pub struct CVEngine {
    voice: CVVoice,
    parameters: Arc<CVParameters>,
    event_rx: Receiver<SynthEvent>,
    midi_channel_filter: u8, // 0-15 for specific channel, 255 for omni
}

impl CVEngine {
    pub fn new(
        sample_rate: f32,
        parameters: Arc<CVParameters>,
        event_rx: Receiver<SynthEvent>,
        midi_channel_filter: u8,
    ) -> Self {
        let mut voice = CVVoice::new(sample_rate);

        // Initialize glide from parameters
        let glide = parameters.glide.load(Ordering::Relaxed);
        voice.set_glide_time(glide);

        Self {
            voice,
            parameters,
            event_rx,
            midi_channel_filter,
        }
    }

    /// Check if this engine should process the given event
    fn should_process_event(&self, event: &SynthEvent) -> bool {
        if self.midi_channel_filter == 255 {
            return true; // Omni
        }

        match event.channel() {
            Some(ch) => ch == self.midi_channel_filter,
            None => true, // AllNotesOff affects all
        }
    }

    /// Get voice states for UI (CV uses only first slot)
    pub fn voice_states(&self) -> [Option<u8>; 16] {
        let mut states = [None; 16];
        states[0] = self.voice.current_note();
        states
    }

    /// Process audio callback - fills pitch and gate buffers
    pub fn process_dual_channel(&mut self, pitch_output: &mut [f32], gate_output: &mut [f32]) {
        assert_eq!(pitch_output.len(), gate_output.len());

        // Process all pending MIDI events
        while let Ok(event) = self.event_rx.try_recv() {
            if !self.should_process_event(&event) {
                continue;
            }

            match event {
                SynthEvent::NoteOn { note, .. } => {
                    self.voice.note_on(note);
                }
                SynthEvent::NoteOff { note, .. } => {
                    self.voice.note_off(note);
                }
                SynthEvent::AllNotesOff { .. } => {
                    self.voice.all_notes_off();
                }
            }
        }

        // Update parameters from UI thread
        let glide = self.parameters.glide.load(Ordering::Relaxed);
        self.voice.set_glide_time(glide);

        let transpose = self.parameters.transpose.load(Ordering::Relaxed);
        self.voice.set_transpose(transpose);

        // Generate CV samples
        for i in 0..pitch_output.len() {
            pitch_output[i] = self.voice.next_pitch_sample();
            gate_output[i] = self.voice.next_gate_sample();
        }
    }

    /// Process single-channel (legacy compatibility - just outputs pitch CV)
    pub fn process(&mut self, output: &mut [f32]) {
        // Process events
        while let Ok(event) = self.event_rx.try_recv() {
            if !self.should_process_event(&event) {
                continue;
            }

            match event {
                SynthEvent::NoteOn { note, .. } => {
                    self.voice.note_on(note);
                }
                SynthEvent::NoteOff { note, .. } => {
                    self.voice.note_off(note);
                }
                SynthEvent::AllNotesOff { .. } => {
                    self.voice.all_notes_off();
                }
            }
        }

        // Update parameters
        let glide = self.parameters.glide.load(Ordering::Relaxed);
        self.voice.set_glide_time(glide);

        let transpose = self.parameters.transpose.load(Ordering::Relaxed);
        self.voice.set_transpose(transpose);

        // Generate pitch CV only
        for sample in output.iter_mut() {
            *sample = self.voice.next_pitch_sample();
        }
    }
}
