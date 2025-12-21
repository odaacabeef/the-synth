use crate::types::{events::SynthEvent, note::{midi_note_to_frequency, midi_velocity_to_amplitude}};

/// MIDI message types we care about
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MidiMessage {
    NoteOn { channel: u8, note: u8, velocity: u8 },
    NoteOff { channel: u8, note: u8, velocity: u8 },
    ControlChange { channel: u8, controller: u8, value: u8 },
    Unknown,
}

impl MidiMessage {
    /// Parse raw MIDI bytes into a message
    /// Handles standard MIDI protocol: [status, data1, data2]
    pub fn parse(bytes: &[u8]) -> Self {
        if bytes.is_empty() {
            return MidiMessage::Unknown;
        }

        let status = bytes[0];
        let message_type = status & 0xF0;
        let channel = status & 0x0F;

        match message_type {
            0x90 => {
                // Note On
                if bytes.len() >= 3 {
                    let note = bytes[1];
                    let velocity = bytes[2];

                    // MIDI spec: Note On with velocity 0 is actually Note Off
                    if velocity == 0 {
                        MidiMessage::NoteOff {
                            channel,
                            note,
                            velocity: 0,
                        }
                    } else {
                        MidiMessage::NoteOn {
                            channel,
                            note,
                            velocity,
                        }
                    }
                } else {
                    MidiMessage::Unknown
                }
            }
            0x80 => {
                // Note Off
                if bytes.len() >= 3 {
                    MidiMessage::NoteOff {
                        channel,
                        note: bytes[1],
                        velocity: bytes[2],
                    }
                } else {
                    MidiMessage::Unknown
                }
            }
            0xB0 => {
                // Control Change
                if bytes.len() >= 3 {
                    MidiMessage::ControlChange {
                        channel,
                        controller: bytes[1],
                        value: bytes[2],
                    }
                } else {
                    MidiMessage::Unknown
                }
            }
            _ => MidiMessage::Unknown,
        }
    }

    /// Convert MIDI message to synth event
    /// Filters by MIDI channel: 255 = omni (all channels), 0-15 = specific channel
    pub fn to_synth_event(&self, channel_filter: u8) -> Option<SynthEvent> {
        match self {
            MidiMessage::NoteOn { channel, note, velocity } => {
                // Filter by channel (255 = omni mode)
                if channel_filter != 255 && *channel != channel_filter {
                    return None;
                }
                let frequency = midi_note_to_frequency(*note);
                let amplitude = midi_velocity_to_amplitude(*velocity);
                Some(SynthEvent::note_on(frequency, amplitude))
            }
            MidiMessage::NoteOff { channel, note, .. } => {
                // Filter by channel (255 = omni mode)
                if channel_filter != 255 && *channel != channel_filter {
                    return None;
                }
                Some(SynthEvent::note_off(*note))
            }
            MidiMessage::ControlChange { channel, controller, .. } => {
                // Filter by channel (255 = omni mode)
                if channel_filter != 255 && *channel != channel_filter {
                    return None;
                }
                // CC 123 = All Notes Off (MIDI panic)
                if *controller == 123 {
                    Some(SynthEvent::all_notes_off())
                } else {
                    None
                }
            }
            MidiMessage::Unknown => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_note_on() {
        let bytes = [0x90, 60, 100]; // Note On, channel 0, middle C, velocity 100
        let msg = MidiMessage::parse(&bytes);
        assert_eq!(
            msg,
            MidiMessage::NoteOn {
                channel: 0,
                note: 60,
                velocity: 100
            }
        );
    }

    #[test]
    fn test_parse_note_off() {
        let bytes = [0x80, 60, 64]; // Note Off, channel 0, middle C
        let msg = MidiMessage::parse(&bytes);
        assert_eq!(
            msg,
            MidiMessage::NoteOff {
                channel: 0,
                note: 60,
                velocity: 64
            }
        );
    }

    #[test]
    fn test_note_on_velocity_zero_is_note_off() {
        let bytes = [0x90, 60, 0]; // Note On with velocity 0
        let msg = MidiMessage::parse(&bytes);
        assert!(matches!(msg, MidiMessage::NoteOff { .. }));
    }

    #[test]
    fn test_to_synth_event() {
        let msg = MidiMessage::NoteOn {
            channel: 0,
            note: 69, // A4
            velocity: 100,
        };
        let event = msg.to_synth_event(255).unwrap(); // 255 = omni mode
        if let SynthEvent::NoteOn { frequency, .. } = event {
            assert!((frequency - 440.0).abs() < 0.1);
        } else {
            panic!("Expected NoteOn event");
        }
    }
}
