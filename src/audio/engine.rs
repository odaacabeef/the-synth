use std::sync::{atomic::Ordering, Arc};
use crossbeam_channel::Receiver;

use super::{voice_pool::VoicePool, parameters::SynthParameters};
use crate::dsp::reverb::Reverb;
use crate::types::events::SynthEvent;

/// Core synthesis engine
/// Runs in real-time audio thread - must be lock-free and allocation-free
pub struct SynthEngine {
    voice_pool: VoicePool,
    reverb: Reverb,
    parameters: Arc<SynthParameters>,
    event_rx: Receiver<SynthEvent>,
}

impl SynthEngine {
    /// Create new synthesis engine with MIDI event receiver and voice pool
    pub fn new(
        sample_rate: f32,
        parameters: Arc<SynthParameters>,
        event_rx: Receiver<SynthEvent>,
    ) -> Self {
        Self {
            voice_pool: VoicePool::new(sample_rate),
            reverb: Reverb::new(sample_rate),
            parameters,
            event_rx,
        }
    }

    /// Get the state of all voices (note number or None for each voice)
    pub fn voice_states(&self) -> [Option<u8>; 16] {
        self.voice_pool.voice_states()
    }

    /// Process audio callback - fills output buffer with samples
    /// This runs in real-time audio thread - must be fast and lock-free
    pub fn process(&mut self, output: &mut [f32]) {
        // Process all pending MIDI events (non-blocking)
        while let Ok(event) = self.event_rx.try_recv() {
            match event {
                SynthEvent::NoteOn { frequency, velocity: _ } => {
                    // Extract note number from the event
                    // We need the note number to match Note Off events
                    // Calculate it back from frequency (rough approximation)
                    let note = frequency_to_midi_note(frequency);
                    self.voice_pool.note_on(note, frequency);
                }
                SynthEvent::NoteOff { note } => {
                    // Release the matching voice
                    self.voice_pool.note_off(note);
                }
                SynthEvent::AllNotesOff => {
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

        // Read reverb parameters
        let reverb_mix = self.parameters.reverb_mix.load(Ordering::Relaxed);
        let reverb_room_size = self.parameters.reverb_room_size.load(Ordering::Relaxed);
        let reverb_damping = self.parameters.reverb_damping.load(Ordering::Relaxed);

        // Update reverb parameters
        self.reverb.set_mix(reverb_mix);
        self.reverb.set_room_size(reverb_room_size);
        self.reverb.set_damping(reverb_damping);

        // Process all voices and mix to output
        self.voice_pool.process(output);

        // Apply reverb to each sample
        for sample in output.iter_mut() {
            *sample = self.reverb.process(*sample);
        }
    }
}

/// Convert frequency back to MIDI note (approximate)
fn frequency_to_midi_note(frequency: f32) -> u8 {
    const A4: f32 = 440.0;
    const A4_MIDI: i32 = 69;

    let semitones = 12.0 * (frequency / A4).log2();
    (A4_MIDI + semitones.round() as i32).clamp(0, 127) as u8
}
