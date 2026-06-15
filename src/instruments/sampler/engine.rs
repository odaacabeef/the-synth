use std::sync::Arc;

use crossbeam_channel::Receiver;

use super::parameters::SamplerParameters;
use super::sample::SampleData;
use super::voice::SamplerVoice;
use crate::types::events::SynthEvent;

/// Maximum number of simultaneous voices a sampler instance can have.
/// Caps the voice-state array used by the UI.
const MAX_VOICES: usize = 16;

/// Sampler engine - plays one WAV file, triggered by MIDI notes.
///
/// `root` is the note that plays the sample at its recorded pitch. With a
/// `range` set, any note within it (which must surround `root`) retriggers
/// the sample repitched by `note - root` semitones; without a range, only
/// `root` triggers. Playback is one-shot: note-off is ignored.
pub struct SamplerEngine {
    sample: Arc<SampleData>,
    voices: Vec<SamplerVoice>,
    ages: Vec<u64>, // allocation age per voice, for voice stealing
    global_age: u64,
    root: u8,
    range: Option<(u8, u8)>,
    midi_channel_filter: u8, // 0-15, or 255 for omni
    parameters: Arc<SamplerParameters>,
    sample_rate: f32, // device sample rate
    event_rx: Receiver<SynthEvent>,
}

impl SamplerEngine {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sample: Arc<SampleData>,
        parameters: Arc<SamplerParameters>,
        root: u8,
        range: Option<(u8, u8)>,
        midi_channel_filter: u8,
        voices: usize,
        sample_rate: f32,
        event_rx: Receiver<SynthEvent>,
    ) -> Self {
        let voice_count = voices.clamp(1, MAX_VOICES);
        let mut voice_vec = Vec::with_capacity(voice_count);
        let mut ages = Vec::with_capacity(voice_count);
        for _ in 0..voice_count {
            voice_vec.push(SamplerVoice::new());
            ages.push(0);
        }

        Self {
            sample,
            voices: voice_vec,
            ages,
            global_age: 0,
            root,
            range,
            midi_channel_filter,
            parameters,
            sample_rate,
            event_rx,
        }
    }

    /// Check if this engine should process the given event based on channel filtering
    fn should_process_event(&self, event: &SynthEvent) -> bool {
        if self.midi_channel_filter == 255 {
            return true;
        }
        match event.channel() {
            Some(ch) => ch == self.midi_channel_filter,
            None => true, // AllNotesOff affects all
        }
    }

    /// Does this note trigger the sample?
    fn note_in_range(&self, note: u8) -> bool {
        match self.range {
            Some((lo, hi)) => note >= lo && note <= hi,
            None => note == self.root,
        }
    }

    /// Pick a voice for a new note: prefer an idle voice, else steal the oldest.
    fn alloc_voice(&self) -> usize {
        if let Some(idx) = self.voices.iter().position(|v| !v.is_active()) {
            return idx;
        }
        self.ages
            .iter()
            .enumerate()
            .min_by_key(|&(_, &age)| age)
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    /// Start a voice playing the sample at the pitch implied by `note`.
    fn trigger_note(&mut self, note: u8) {
        use std::sync::atomic::Ordering;

        let file_sr = self.sample.sample_rate;
        let len = self.sample.samples.len();
        if len < 2 {
            return; // nothing playable
        }

        let gain_db = self.parameters.gain_db.load(Ordering::Relaxed);
        let pitch = self.parameters.pitch.load(Ordering::Relaxed);
        let start = self.parameters.start.load(Ordering::Relaxed).clamp(0.0, 1.0);
        let attack = self.parameters.attack.load(Ordering::Relaxed).max(0.0);
        let release = self.parameters.release.load(Ordering::Relaxed).max(0.0);

        // Playback rate unifies sample-rate conversion and pitch shifting.
        let semitones = (note as f32 - self.root as f32) + pitch;
        let pitch_ratio = 2f32.powf(semitones / 12.0);
        let rate = (file_sr / self.sample_rate * pitch_ratio) as f64;

        let gain = 10f32.powf(gain_db / 20.0);
        let start_pos = start as f64 * (len as f64 - 1.0);
        let attack_samples = attack * self.sample_rate;
        let release_samples = release * self.sample_rate;

        let idx = self.alloc_voice();
        let sample = Arc::clone(&self.sample);
        self.voices[idx].trigger(
            sample,
            start_pos,
            rate,
            gain,
            attack_samples,
            release_samples,
            note,
        );
        self.ages[idx] = self.global_age;
        self.global_age += 1;
    }

    /// Get voice states (note numbers) for the UI.
    pub fn voice_states(&self) -> [Option<u8>; 16] {
        let mut states = [None; 16];
        for (i, voice) in self.voices.iter().enumerate().take(16) {
            if voice.is_active() {
                states[i] = voice.note();
            }
        }
        states
    }

    /// Process audio callback - fills output buffer with samples.
    /// Runs in the real-time audio thread: fast and lock-free.
    pub fn process(&mut self, output: &mut [f32]) {
        while let Ok(event) = self.event_rx.try_recv() {
            if !self.should_process_event(&event) {
                continue;
            }

            match event {
                SynthEvent::NoteOn { note, .. } => {
                    if self.note_in_range(note) {
                        self.trigger_note(note);
                    }
                }
                SynthEvent::NoteOff { .. } => {
                    // One-shot playback: note-off is ignored.
                }
                SynthEvent::AllNotesOff { .. } => {
                    for voice in &mut self.voices {
                        voice.stop();
                    }
                }
            }
        }

        // Mix active voices. No auto-normalization: per-sample gain and
        // external mixing are the level controls, consistent with the rest
        // of the project.
        output.fill(0.0);
        for voice in &mut self.voices {
            if voice.is_active() {
                for sample in output.iter_mut() {
                    *sample += voice.next_sample();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::{unbounded, Receiver};

    fn make_engine(
        root: u8,
        range: Option<(u8, u8)>,
        voices: usize,
        channel: u8,
        rx: Receiver<SynthEvent>,
    ) -> SamplerEngine {
        let sample = Arc::new(SampleData {
            samples: vec![0.5; 4410],
            sample_rate: 44100.0,
        });
        let params = Arc::new(SamplerParameters::new());
        SamplerEngine::new(sample, params, root, range, channel, voices, 44100.0, rx)
    }

    #[test]
    fn test_triggers_on_root() {
        let (tx, rx) = unbounded();
        let mut engine = make_engine(60, None, 1, 9, rx);
        tx.send(SynthEvent::note_on(9, 60, 261.6, 1.0)).unwrap();

        let mut buf = vec![0.0f32; 512];
        engine.process(&mut buf);
        assert!(buf.iter().any(|s| s.abs() > 0.0001));
    }

    #[test]
    fn test_ignores_out_of_range_note_without_range() {
        let (tx, rx) = unbounded();
        let mut engine = make_engine(60, None, 1, 9, rx);
        tx.send(SynthEvent::note_on(9, 62, 293.7, 1.0)).unwrap();

        let mut buf = vec![0.0f32; 512];
        engine.process(&mut buf);
        assert!(buf.iter().all(|s| s.abs() < 0.0001));
    }

    #[test]
    fn test_range_allows_span() {
        let (tx, rx) = unbounded();
        let mut engine = make_engine(60, Some((48, 72)), 1, 9, rx);
        tx.send(SynthEvent::note_on(9, 67, 392.0, 1.0)).unwrap();

        let mut buf = vec![0.0f32; 512];
        engine.process(&mut buf);
        assert!(buf.iter().any(|s| s.abs() > 0.0001));
    }

    #[test]
    fn test_ignores_wrong_channel() {
        let (tx, rx) = unbounded();
        let mut engine = make_engine(60, None, 1, 9, rx);
        tx.send(SynthEvent::note_on(0, 60, 261.6, 1.0)).unwrap();

        let mut buf = vec![0.0f32; 512];
        engine.process(&mut buf);
        assert!(buf.iter().all(|s| s.abs() < 0.0001));
    }

    #[test]
    fn test_polyphony_two_voices() {
        let (tx, rx) = unbounded();
        let mut engine = make_engine(60, Some((48, 72)), 4, 9, rx);
        tx.send(SynthEvent::note_on(9, 60, 261.6, 1.0)).unwrap();
        tx.send(SynthEvent::note_on(9, 64, 329.6, 1.0)).unwrap();

        let mut buf = vec![0.0f32; 64];
        engine.process(&mut buf);

        let active = engine.voice_states().iter().filter(|s| s.is_some()).count();
        assert_eq!(active, 2);
    }

    #[test]
    fn test_monophonic_retrigger_single_voice() {
        let (tx, rx) = unbounded();
        let mut engine = make_engine(60, Some((48, 72)), 1, 9, rx);
        tx.send(SynthEvent::note_on(9, 60, 261.6, 1.0)).unwrap();
        tx.send(SynthEvent::note_on(9, 64, 329.6, 1.0)).unwrap();

        let mut buf = vec![0.0f32; 64];
        engine.process(&mut buf);

        // Only one voice exists, so the second note steals it.
        let states = engine.voice_states();
        let active = states.iter().filter(|s| s.is_some()).count();
        assert_eq!(active, 1);
        assert_eq!(states[0], Some(64));
    }
}
