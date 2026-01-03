use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::Arc;

use super::{engine::SynthEngine, parameters::SynthParameters};
use crate::types::events::SynthEvent;

/// Individual synthesizer instance within a multi-engine setup
pub struct SynthInstance {
    pub engine: SynthEngine,
    pub audio_channel: usize,
}

/// Multi-engine synthesizer
/// Manages multiple independent synth engines, each with its own channel routing
pub struct MultiEngineSynth {
    instances: Vec<SynthInstance>,
    main_event_rx: Receiver<SynthEvent>,
    instance_event_txs: Vec<Sender<SynthEvent>>,
}

impl MultiEngineSynth {
    /// Create a new multi-engine synthesizer
    ///
    /// # Arguments
    /// * `sample_rate` - Audio sample rate
    /// * `instances` - Vector of (parameters, midi_channel, audio_channel) tuples
    /// * `main_event_rx` - Main MIDI event receiver (events will be broadcast to all engines)
    pub fn new(
        sample_rate: f32,
        instances: Vec<(Arc<SynthParameters>, u8, usize)>,
        main_event_rx: Receiver<SynthEvent>,
    ) -> Self {
        let mut synth_instances = Vec::new();
        let mut instance_event_txs = Vec::new();

        for (parameters, midi_channel, audio_channel) in instances {
            // Create a dedicated event channel for this instance
            let (event_tx, event_rx) = unbounded();

            // Create the synth engine with channel filtering
            let engine = SynthEngine::new_with_channel(
                sample_rate,
                parameters,
                event_rx,
                midi_channel,
            );

            synth_instances.push(SynthInstance {
                engine,
                audio_channel,
            });

            instance_event_txs.push(event_tx);
        }

        Self {
            instances: synth_instances,
            main_event_rx,
            instance_event_txs,
        }
    }

    /// Get voice states for all instances
    /// Returns a vector of voice states in instance order
    pub fn all_voice_states(&self) -> Vec<[Option<u8>; 16]> {
        self.instances
            .iter()
            .map(|inst| inst.engine.voice_states())
            .collect()
    }

    /// Process audio for all engines and mix to multi-channel output
    ///
    /// # Arguments
    /// * `output` - Interleaved multi-channel output buffer
    /// * `num_channels` - Number of output channels
    pub fn process(&mut self, output: &mut [f32], num_channels: usize) {
        let frames = output.len() / num_channels;

        // Clear output buffer
        output.fill(0.0);

        // Broadcast all pending events to instance event channels
        while let Ok(event) = self.main_event_rx.try_recv() {
            for event_tx in &self.instance_event_txs {
                // Non-blocking send; if channel is full, skip (shouldn't happen in practice)
                let _ = event_tx.try_send(event);
            }
        }

        // Temporary buffer for each engine's mono output
        let mut engine_buffer = vec![0.0f32; frames];

        // Process each instance
        for instance in &mut self.instances {
            // Generate audio for this instance
            engine_buffer.fill(0.0);
            instance.engine.process(&mut engine_buffer);

            // Mix to the specified output channel
            let channel_idx = instance.audio_channel;
            if channel_idx < num_channels {
                for frame_idx in 0..frames {
                    let sample = engine_buffer[frame_idx];
                    output[frame_idx * num_channels + channel_idx] += sample;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::parameters::SynthParameters;

    #[test]
    fn test_multi_engine_creation() {
        let (event_tx, event_rx) = unbounded();

        let params1 = Arc::new(SynthParameters::default());
        let params2 = Arc::new(SynthParameters::default());

        let instances = vec![
            ("Bass".to_string(), params1, 0, 0),  // Channel 0 -> Audio 0
            ("Lead".to_string(), params2, 1, 1),  // Channel 1 -> Audio 1
        ];

        let multi = MultiEngineSynth::new(44100.0, instances, event_rx);

        assert_eq!(multi.instance_count(), 2);
        assert_eq!(multi.get_instance(0).unwrap().name, "Bass");
        assert_eq!(multi.get_instance(1).unwrap().name, "Lead");

        drop(event_tx); // Keep compiler happy
    }

    #[test]
    fn test_event_broadcasting() {
        let (event_tx, event_rx) = unbounded();

        let params1 = Arc::new(SynthParameters::default());
        let params2 = Arc::new(SynthParameters::default());

        let instances = vec![
            ("Synth1".to_string(), params1, 0, 0),
            ("Synth2".to_string(), params2, 1, 1),
        ];

        let mut multi = MultiEngineSynth::new(44100.0, instances, event_rx);

        // Send a note on event
        let _ = event_tx.try_send(SynthEvent::note_on(0, 440.0, 0.8));

        // Process a small buffer to trigger event broadcasting
        let mut output = vec![0.0f32; 256 * 2]; // 256 frames, 2 channels
        multi.process(&mut output, 2);

        // Events should have been broadcast to all instances
        // (Can't directly verify without exposing internals, but no panics = good)
    }

    #[test]
    fn test_channel_routing() {
        let (event_tx, event_rx) = unbounded();

        let params = Arc::new(SynthParameters::default());

        let instances = vec![
            ("Ch0".to_string(), params.clone(), 0, 0),
            ("Ch1".to_string(), params.clone(), 1, 1),
        ];

        let mut multi = MultiEngineSynth::new(44100.0, instances, event_rx);

        // Send note on channel 0
        let _ = event_tx.try_send(SynthEvent::note_on(0, 440.0, 0.8));

        // Process
        let mut output = vec![0.0f32; 512]; // 256 frames, 2 channels
        multi.process(&mut output, 2);

        // Channel 0 should have audio (non-zero), channel 1 should be silent
        let ch0_has_audio = output.iter().step_by(2).any(|&s| s.abs() > 0.001);
        let ch1_has_audio = output.iter().skip(1).step_by(2).any(|&s| s.abs() > 0.001);

        assert!(ch0_has_audio, "Channel 0 should have audio");
        assert!(!ch1_has_audio, "Channel 1 should be silent");
    }
}
