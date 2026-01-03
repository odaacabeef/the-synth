/// Internal events sent from MIDI thread to audio thread
/// Must be simple and fast to construct/parse
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SynthEvent {
    /// Note on event with channel, frequency and velocity
    NoteOn { channel: u8, frequency: f32, velocity: f32 },
    /// Note off event with channel and note number
    NoteOff { channel: u8, note: u8 },
    /// All notes off (MIDI panic) - None for all channels
    AllNotesOff { channel: Option<u8> },
}

impl SynthEvent {
    /// Create a note on event
    pub fn note_on(channel: u8, frequency: f32, velocity: f32) -> Self {
        SynthEvent::NoteOn { channel, frequency, velocity }
    }

    /// Create a note off event
    pub fn note_off(channel: u8, note: u8) -> Self {
        SynthEvent::NoteOff { channel, note }
    }

    /// Create an all notes off event for a specific channel
    pub fn all_notes_off_channel(channel: u8) -> Self {
        SynthEvent::AllNotesOff { channel: Some(channel) }
    }

    /// Create an all notes off event for all channels
    pub fn all_notes_off_all() -> Self {
        SynthEvent::AllNotesOff { channel: None }
    }

    /// Get the channel for this event, if applicable
    pub fn channel(&self) -> Option<u8> {
        match self {
            SynthEvent::NoteOn { channel, .. } => Some(*channel),
            SynthEvent::NoteOff { channel, .. } => Some(*channel),
            SynthEvent::AllNotesOff { channel } => *channel,
        }
    }
}
