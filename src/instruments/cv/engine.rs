use crossbeam_channel::Receiver;
use std::sync::{atomic::Ordering, Arc};

use super::{parameters::CVParameters, voice::CVVoice};
use crate::types::events::SynthEvent;

/// CV output engine - generates control voltages for modular synths
pub struct CVEngine {
    voices: Vec<CVVoice>,
    voice_notes: Vec<Option<u8>>, // active note per pitch voice
    voice_ages: Vec<u64>,         // assignment age for voice stealing
    age_counter: u64,
    gate_note_count: usize, // count of held notes (drives gate output)
    voice_count: usize,
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
        voice_count: usize,
    ) -> Self {
        let glide = parameters.glide.load(Ordering::Relaxed);

        let voices: Vec<CVVoice> = (0..voice_count)
            .map(|_| {
                let mut v = CVVoice::new(sample_rate);
                v.set_glide_time(glide);
                v
            })
            .collect();

        Self {
            voice_notes: vec![None; voice_count],
            voice_ages: vec![0; voice_count],
            age_counter: 0,
            gate_note_count: 0,
            voices,
            voice_count,
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

    fn handle_note_on(&mut self, note: u8) {
        self.gate_note_count = self.gate_note_count.saturating_add(1);

        if self.voice_count == 0 {
            return;
        }

        // If note already assigned to a voice, retrigger it
        if let Some(i) = self.voice_notes.iter().position(|&n| n == Some(note)) {
            self.voices[i].note_on(note);
            self.voice_ages[i] = self.age_counter;
            self.age_counter += 1;
            return;
        }

        // Find a free voice or steal the oldest
        let voice_idx = self
            .voice_notes
            .iter()
            .position(|n| n.is_none())
            .unwrap_or_else(|| {
                self.voice_ages
                    .iter()
                    .enumerate()
                    .min_by_key(|&(_, &age)| age)
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            });

        // Clear stolen voice before assigning
        if self.voice_notes[voice_idx].is_some() {
            self.voices[voice_idx].all_notes_off();
        }

        self.voice_notes[voice_idx] = Some(note);
        self.voice_ages[voice_idx] = self.age_counter;
        self.age_counter += 1;
        self.voices[voice_idx].note_on(note);
    }

    fn handle_note_off(&mut self, note: u8) {
        self.gate_note_count = self.gate_note_count.saturating_sub(1);

        if self.voice_count == 0 {
            return;
        }

        if let Some(i) = self.voice_notes.iter().position(|&n| n == Some(note)) {
            self.voices[i].note_off(note);
            self.voice_notes[i] = None;
        }
    }

    fn handle_all_notes_off(&mut self) {
        self.gate_note_count = 0;
        for (voice, note) in self.voices.iter_mut().zip(self.voice_notes.iter_mut()) {
            voice.all_notes_off();
            *note = None;
        }
    }

    /// Get voice states for UI - returns active note per voice slot
    pub fn voice_states(&self) -> [Option<u8>; 16] {
        let mut states = [None; 16];
        for (i, &note) in self.voice_notes.iter().enumerate().take(16) {
            states[i] = note;
        }
        states
    }

    /// Process audio callback - fills gate buffer and one pitch buffer per voice
    ///
    /// Gate is high (0.8) whenever any note is held, regardless of voice count.
    /// `pitch_outputs` must have exactly `voice_count` entries.
    pub fn process_cv(&mut self, gate_output: &mut [f32], pitch_outputs: &mut [Vec<f32>]) {
        let frames = gate_output.len();

        // Process all pending MIDI events
        while let Ok(event) = self.event_rx.try_recv() {
            if !self.should_process_event(&event) {
                continue;
            }

            match event {
                SynthEvent::NoteOn { note, .. } => self.handle_note_on(note),
                SynthEvent::NoteOff { note, .. } => self.handle_note_off(note),
                SynthEvent::AllNotesOff { .. } => self.handle_all_notes_off(),
            }
        }

        // Update parameters from UI thread
        let glide = self.parameters.glide.load(Ordering::Relaxed);
        let transpose = self.parameters.transpose.load(Ordering::Relaxed);
        for voice in &mut self.voices {
            voice.set_glide_time(glide);
            voice.set_transpose(transpose);
        }

        // Fill gate buffer (constant for the whole block)
        let gate_val = if self.gate_note_count > 0 { 0.8 } else { 0.0 };
        gate_output.fill(gate_val);

        // Fill pitch buffers
        for (voice, buf) in self.voices.iter_mut().zip(pitch_outputs.iter_mut()) {
            for s in &mut buf[..frames] {
                *s = voice.next_pitch_sample();
            }
        }
    }
}
