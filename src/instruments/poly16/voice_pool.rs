use super::voice::Voice;

/// Number of simultaneous voices supported
pub const MAX_VOICES: usize = 16;

/// Voice state for tracking and allocation
#[derive(Debug, Clone, Copy, PartialEq)]
enum VoiceState {
    Idle,
    Active { note: u8 },
}

/// Individual voice in the pool with state tracking
struct PoolVoice {
    voice: Voice,
    state: VoiceState,
    age: u64, // For voice stealing - higher = older
}

impl PoolVoice {
    fn new(sample_rate: f32) -> Self {
        Self {
            voice: Voice::new(sample_rate),
            state: VoiceState::Idle,
            age: 0,
        }
    }

    fn is_idle(&self) -> bool {
        matches!(self.state, VoiceState::Idle)
    }

    fn is_active(&self) -> bool {
        matches!(self.state, VoiceState::Active { .. })
    }

    fn is_releasing(&self) -> bool {
        // Voice is releasing if it's not idle but envelope is not active
        self.is_active() && !self.voice.is_active()
    }
}

/// Pool of voices for polyphonic synthesis
/// Pre-allocated array of voices with allocation/stealing logic
pub struct VoicePool {
    voices: [PoolVoice; MAX_VOICES],
    global_age: u64,
}

impl VoicePool {
    /// Create new voice pool with all voices pre-allocated
    pub fn new(sample_rate: f32) -> Self {
        Self {
            voices: [
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
                PoolVoice::new(sample_rate),
            ],
            global_age: 0,
        }
    }

    /// Trigger a note on - allocates or steals a voice
    pub fn note_on(&mut self, note: u8, frequency: f32) {
        // Find or allocate a voice
        let voice_idx = self.find_voice_for_note(note);

        // Configure and trigger the voice
        let pool_voice = &mut self.voices[voice_idx];
        pool_voice.voice.note_on(frequency);
        pool_voice.state = VoiceState::Active { note };
        pool_voice.age = self.global_age;
        self.global_age += 1;
    }

    /// Trigger a note off - finds matching voice(s)
    pub fn note_off(&mut self, note: u8) {
        // Release ALL voices playing this note
        // (handles cases where same note triggered multiple times)
        for pool_voice in &mut self.voices {
            if let VoiceState::Active { note: active_note } = pool_voice.state {
                if active_note == note {
                    pool_voice.voice.note_off();
                    // Keep state as Active until envelope is done
                }
            }
        }
    }

    /// Find best voice for a new note
    /// Priority: Idle > Releasing > Oldest
    fn find_voice_for_note(&mut self, _note: u8) -> usize {
        // 1. Try to find an idle voice
        if let Some(idx) = self.voices.iter().position(|v| v.is_idle()) {
            return idx;
        }

        // 2. Try to find a releasing voice
        if let Some(idx) = self.voices.iter().position(|v| v.is_releasing()) {
            return idx;
        }

        // 3. Steal the oldest active voice
        self.voices
            .iter()
            .enumerate()
            .min_by_key(|(_, v)| v.age)
            .map(|(idx, _)| idx)
            .unwrap_or(0)
    }

    /// Update voice states - mark voices as idle when envelope completes
    fn update_voice_states(&mut self) {
        for pool_voice in &mut self.voices {
            if pool_voice.is_active() && !pool_voice.voice.is_active() {
                pool_voice.state = VoiceState::Idle;
            }
        }
    }

    /// Set ADSR parameters for all voices
    pub fn set_adsr(&mut self, attack: f32, decay: f32, sustain: f32, release: f32) {
        for pool_voice in &mut self.voices {
            pool_voice.voice.set_adsr(attack, decay, sustain, release);
        }
    }

    /// Set waveform for all voices
    pub fn set_waveform(&mut self, waveform: crate::types::waveform::Waveform) {
        for pool_voice in &mut self.voices {
            pool_voice.voice.set_waveform(waveform);
        }
    }

    /// Release all active voices (MIDI panic)
    pub fn all_notes_off(&mut self) {
        for pool_voice in &mut self.voices {
            if pool_voice.is_active() {
                pool_voice.voice.note_off();
            }
        }
    }

    /// Process all voices and mix to output buffer
    pub fn process(&mut self, output: &mut [f32]) {
        // Update voice states first
        self.update_voice_states();

        // Clear output buffer
        output.fill(0.0);

        // Mix all voices
        for pool_voice in &mut self.voices {
            if !pool_voice.is_idle() {
                for sample in output.iter_mut() {
                    *sample += pool_voice.voice.next_sample();
                }
            }
        }

        // Scale down to prevent clipping (simple normalization)
        let scale = 1.0 / (MAX_VOICES as f32).sqrt();
        for sample in output.iter_mut() {
            *sample *= scale;
        }
    }

    /// Get the state of all voices (note number or None)
    pub fn voice_states(&self) -> [Option<u8>; MAX_VOICES] {
        let mut states = [None; MAX_VOICES];
        for (i, pool_voice) in self.voices.iter().enumerate() {
            states[i] = match pool_voice.state {
                VoiceState::Active { note } => Some(note),
                VoiceState::Idle => None,
            };
        }
        states
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voice_pool_creates() {
        let pool = VoicePool::new(44100.0);
        let states = pool.voice_states();
        assert!(states.iter().all(|s| s.is_none()));
    }

    #[test]
    fn test_voice_allocation() {
        let mut pool = VoicePool::new(44100.0);

        pool.note_on(60, 261.63); // Middle C
        let states = pool.voice_states();
        assert_eq!(states.iter().filter(|s| s.is_some()).count(), 1);

        pool.note_on(64, 329.63); // E
        let states = pool.voice_states();
        assert_eq!(states.iter().filter(|s| s.is_some()).count(), 2);
    }

    #[test]
    fn test_max_voices() {
        let mut pool = VoicePool::new(44100.0);

        // Play 10 notes (more than MAX_VOICES)
        for i in 0..10 {
            pool.note_on(60 + i, 440.0);
        }

        // Should never exceed MAX_VOICES
        let states = pool.voice_states();
        assert!(states.iter().filter(|s| s.is_some()).count() <= MAX_VOICES);
    }
}
