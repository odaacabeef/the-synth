use crossbeam_channel::Receiver;

use crate::types::events::SynthEvent;

/// ES-5 gate encoder engine
///
/// Encodes up to 6 gate outputs into a stereo audio pair using the
/// Expert Sleepers ES-5 protocol. Each gate maps to an individual bit
/// in the upper byte of a 24-bit audio sample, 3 outputs per channel:
///   Left:  out1 = bit 16, out2 = bit 17, out3 = bit 18
///   Right: out4 = bit 16, out5 = bit 17, out6 = bit 18
///
/// The float sample value is the unsigned 24-bit integer divided by
/// 2^23. A set bit reads as a high gate (5V), an unset bit as 0V.
pub struct ES5Engine {
    gate_states: [bool; 6],
    trigger_notes: [Option<u8>; 6],
    output_count: usize,
    midi_channel_filter: u8,
    event_rx: Receiver<SynthEvent>,
}

impl ES5Engine {
    pub fn new(
        trigger_notes: &[u8],
        midi_channel_filter: u8,
        event_rx: Receiver<SynthEvent>,
    ) -> Self {
        let output_count = trigger_notes.len().min(6);
        let mut notes = [None; 6];
        for (i, &note) in trigger_notes.iter().take(6).enumerate() {
            notes[i] = Some(note);
        }

        Self {
            gate_states: [false; 6],
            trigger_notes: notes,
            output_count,
            midi_channel_filter,
            event_rx,
        }
    }

    fn should_process_event(&self, event: &SynthEvent) -> bool {
        if self.midi_channel_filter == 255 {
            return true;
        }
        match event.channel() {
            Some(ch) => ch == self.midi_channel_filter,
            None => true,
        }
    }

    pub fn voice_states(&self) -> [Option<u8>; 16] {
        let mut states = [None; 16];
        for i in 0..self.output_count {
            if self.gate_states[i] {
                states[i] = self.trigger_notes[i];
            }
        }
        states
    }

    #[cfg(test)]
    pub fn output_count(&self) -> usize {
        self.output_count
    }

    /// Process audio - fills left and right buffers with ES-5 encoded gate data
    pub fn process_dual_channel(&mut self, left_output: &mut [f32], right_output: &mut [f32]) {
        // Process all pending MIDI events
        while let Ok(event) = self.event_rx.try_recv() {
            if !self.should_process_event(&event) {
                continue;
            }

            match event {
                SynthEvent::NoteOn { note, .. } => {
                    for i in 0..self.output_count {
                        if self.trigger_notes[i] == Some(note) {
                            self.gate_states[i] = true;
                        }
                    }
                }
                SynthEvent::NoteOff { note, .. } => {
                    for i in 0..self.output_count {
                        if self.trigger_notes[i] == Some(note) {
                            self.gate_states[i] = false;
                        }
                    }
                }
                SynthEvent::AllNotesOff { .. } => {
                    self.gate_states = [false; 6];
                }
            }
        }

        // Encode gate states into ES-5 protocol.
        // Each gate output maps to an individual bit in the upper byte of a
        // 24-bit audio sample. The ES-5 decodes consecutive bits starting
        // from bit 16, with 3 outputs per channel (left and right):
        //   Left channel:  out1 = bit 16, out2 = bit 17, out3 = bit 18
        //   Right channel: out4 = bit 16, out5 = bit 17, out6 = bit 18
        // The float value is the unsigned 24-bit integer divided by 2^23.
        let mut left_bits: u32 = 0;
        if self.gate_states[0] { left_bits |= 1 << 16; }
        if self.gate_states[1] { left_bits |= 1 << 17; }
        if self.gate_states[2] { left_bits |= 1 << 18; }
        let mut right_bits: u32 = 0;
        if self.gate_states[3] { right_bits |= 1 << 16; }
        if self.gate_states[4] { right_bits |= 1 << 17; }
        if self.gate_states[5] { right_bits |= 1 << 18; }
        left_output.fill(left_bits as f32 / 8388608.0);
        right_output.fill(right_bits as f32 / 8388608.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;

    #[test]
    fn test_engine_creates() {
        let (_tx, rx) = unbounded();
        let engine = ES5Engine::new(&[36, 38, 42, 46, 48, 50], 9, rx);

        let states = engine.voice_states();
        assert_eq!(states[0], None);
        assert_eq!(engine.output_count(), 6);
    }

    #[test]
    fn test_engine_gate_on_off() {
        let (tx, rx) = unbounded();
        let mut engine = ES5Engine::new(&[36, 38], 9, rx);

        // Send note on for first gate
        tx.send(SynthEvent::note_on(9, 36, 65.4, 1.0)).unwrap();

        let mut left = vec![0.0f32; 64];
        let mut right = vec![0.0f32; 64];
        engine.process_dual_channel(&mut left, &mut right);

        // Gate 1 should be on
        let states = engine.voice_states();
        assert_eq!(states[0], Some(36));
        assert_eq!(states[1], None);

        // Left channel should have bit 16 set: (1 << 16) / 2^23
        let expected = (1u32 << 16) as f32 / 8388608.0;
        assert_eq!(left[0], expected);
        // Right channel should be zero
        assert_eq!(right[0], 0.0);

        // Send note off
        tx.send(SynthEvent::NoteOff { channel: 9, note: 36 }).unwrap();
        engine.process_dual_channel(&mut left, &mut right);

        let states = engine.voice_states();
        assert_eq!(states[0], None);
        assert_eq!(left[0], 0.0);
    }

    #[test]
    fn test_engine_multiple_gates() {
        let (tx, rx) = unbounded();
        let mut engine = ES5Engine::new(&[36, 38, 42, 46], 9, rx);

        // Turn on gates 1 and 2 (both on left channel, bits 16 and 17)
        tx.send(SynthEvent::note_on(9, 36, 65.4, 1.0)).unwrap();
        tx.send(SynthEvent::note_on(9, 38, 73.4, 1.0)).unwrap();

        let mut left = vec![0.0f32; 64];
        let mut right = vec![0.0f32; 64];
        engine.process_dual_channel(&mut left, &mut right);

        // Left should have bits 16 and 17 set
        let expected = ((1u32 << 16) | (1u32 << 17)) as f32 / 8388608.0;
        assert_eq!(left[0], expected);
        assert_eq!(right[0], 0.0);

        // Turn on gate 4 (right channel, bit 16)
        tx.send(SynthEvent::note_on(9, 46, 92.5, 1.0)).unwrap();
        engine.process_dual_channel(&mut left, &mut right);

        let expected_right = (1u32 << 16) as f32 / 8388608.0;
        assert_eq!(right[0], expected_right);
    }

    #[test]
    fn test_engine_ignores_wrong_channel() {
        let (tx, rx) = unbounded();
        let mut engine = ES5Engine::new(&[36], 9, rx);

        tx.send(SynthEvent::note_on(0, 36, 65.4, 1.0)).unwrap();

        let mut left = vec![0.0f32; 64];
        let mut right = vec![0.0f32; 64];
        engine.process_dual_channel(&mut left, &mut right);

        assert_eq!(engine.voice_states()[0], None);
    }

    #[test]
    fn test_engine_ignores_wrong_note() {
        let (tx, rx) = unbounded();
        let mut engine = ES5Engine::new(&[36], 9, rx);

        tx.send(SynthEvent::note_on(9, 38, 73.4, 1.0)).unwrap();

        let mut left = vec![0.0f32; 64];
        let mut right = vec![0.0f32; 64];
        engine.process_dual_channel(&mut left, &mut right);

        assert_eq!(engine.voice_states()[0], None);
    }
}
