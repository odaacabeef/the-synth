/// Internal events sent from MIDI thread to audio thread
/// Must be simple and fast to construct/parse
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SynthEvent {
    /// Note on event with frequency and velocity
    NoteOn { frequency: f32, velocity: f32 },
    /// Note off event with note number
    NoteOff { note: u8 },
    /// All notes off (MIDI panic)
    AllNotesOff,
}

impl SynthEvent {
    /// Create a note on event
    pub fn note_on(frequency: f32, velocity: f32) -> Self {
        SynthEvent::NoteOn { frequency, velocity }
    }

    /// Create a note off event
    pub fn note_off(note: u8) -> Self {
        SynthEvent::NoteOff { note }
    }

    /// Create an all notes off event
    pub fn all_notes_off() -> Self {
        SynthEvent::AllNotesOff
    }
}
