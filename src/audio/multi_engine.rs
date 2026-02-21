use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::Arc;

use crate::instruments::poly16::{SynthEngine, SynthParameters};
use crate::instruments::drums::{DrumEngine, DrumParameters};
use crate::instruments::cv::{CVEngine, CVParameters};
use crate::types::events::SynthEvent;

/// Specification for creating an engine instance
pub enum EngineSpec {
    Synth {
        params: Arc<SynthParameters>,
        midi_channel: u8,
    },
    Drum {
        trigger_note: u8,
        midi_channel: u8,
        parameters: DrumParameters,
    },
    CV {
        parameters: Arc<CVParameters>,
        midi_channel: u8,
        voice_count: usize,
        note_filter: Option<u8>,
    },
}

/// Engine type - can be either a synth, drum, or CV
pub enum EngineType {
    Synth(SynthEngine),
    Drum(DrumEngine),
    CV(CVEngine),
}

impl EngineType {
    /// Process audio for this engine (synth/drum only)
    fn process(&mut self, output: &mut [f32]) {
        match self {
            EngineType::Synth(e) => e.process(output),
            EngineType::Drum(e) => e.process(output),
            EngineType::CV(_) => panic!("CV engines must use process_cv"),
        }
    }

    /// Get voice states for this engine
    fn voice_states(&self) -> [Option<u8>; 16] {
        match self {
            EngineType::Synth(e) => e.voice_states(),
            EngineType::Drum(e) => e.voice_states(),
            EngineType::CV(e) => e.voice_states(),
        }
    }
}

/// Individual synthesizer instance within a multi-engine setup
pub struct SynthInstance {
    pub engine: EngineType,
    pub audio_channel: usize,
    /// None for synth/drum (single channel), Some(n) for CV with n pitch voices
    pub cv_voices: Option<usize>,
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
    /// * `instances` - Vector of (engine_spec, audio_channel) tuples
    /// * `main_event_rx` - Main MIDI event receiver (events will be broadcast to all engines)
    pub fn new(
        sample_rate: f32,
        instances: Vec<(EngineSpec, usize)>,
        main_event_rx: Receiver<SynthEvent>,
    ) -> Self {
        let mut synth_instances = Vec::new();
        let mut instance_event_txs = Vec::new();

        for (spec, audio_channel) in instances {
            // Create a dedicated event channel for this instance
            let (event_tx, event_rx) = unbounded();

            let (engine, cv_voices) = match spec {
                EngineSpec::Synth {
                    params,
                    midi_channel,
                } => {
                    let synth_engine = SynthEngine::new_with_channel(
                        sample_rate,
                        params,
                        event_rx,
                        midi_channel,
                    );
                    (EngineType::Synth(synth_engine), None)
                }
                EngineSpec::Drum {
                    trigger_note,
                    midi_channel,
                    parameters,
                } => {
                    let drum_engine = DrumEngine::new_with_parameters(
                        parameters,
                        trigger_note,
                        sample_rate,
                        midi_channel,
                        event_rx,
                    );
                    (EngineType::Drum(drum_engine), None)
                }
                EngineSpec::CV {
                    parameters,
                    midi_channel,
                    voice_count,
                    note_filter,
                } => {
                    let cv_engine = CVEngine::new(
                        sample_rate,
                        parameters,
                        event_rx,
                        midi_channel,
                        voice_count,
                        note_filter,
                    );
                    (EngineType::CV(cv_engine), Some(voice_count))
                }
            };

            synth_instances.push(SynthInstance {
                engine,
                audio_channel,
                cv_voices,
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
                let _ = event_tx.try_send(event);
            }
        }

        // Temporary buffers for engine output (grown as needed, reused across instances)
        let mut mono_buffer = vec![0.0f32; frames];
        let mut gate_buffer = vec![0.0f32; frames];
        // Pitch buffers for CV voices; grown lazily as we encounter CV instances
        let mut cv_pitch_buffers: Vec<Vec<f32>> = Vec::new();

        // Process each instance
        for instance in &mut self.instances {
            if let Some(voice_count) = instance.cv_voices {
                // CV multi-channel processing:
                // Gate -> audio_channel, pitch voices -> audio_channel+1, audio_channel+2, ...

                // Ensure we have enough pitch buffers
                while cv_pitch_buffers.len() < voice_count {
                    cv_pitch_buffers.push(vec![0.0f32; frames]);
                }

                if let EngineType::CV(engine) = &mut instance.engine {
                    engine.process_cv(
                        &mut gate_buffer[..frames],
                        &mut cv_pitch_buffers[..voice_count],
                    );
                }

                // Route gate to audio_channel
                let gate_ch = instance.audio_channel;
                if gate_ch < num_channels {
                    for frame_idx in 0..frames {
                        output[frame_idx * num_channels + gate_ch] += gate_buffer[frame_idx];
                    }
                }

                // Route pitch voices to audio_channel+1, audio_channel+2, ...
                for (v, pitch_buf) in cv_pitch_buffers[..voice_count].iter().enumerate() {
                    let pitch_ch = instance.audio_channel + 1 + v;
                    if pitch_ch < num_channels {
                        for frame_idx in 0..frames {
                            output[frame_idx * num_channels + pitch_ch] += pitch_buf[frame_idx];
                        }
                    }
                }
            } else {
                // Single-channel processing (synth/drum)
                mono_buffer.fill(0.0);
                instance.engine.process(&mut mono_buffer);

                let channel_idx = instance.audio_channel;
                if channel_idx < num_channels {
                    for frame_idx in 0..frames {
                        output[frame_idx * num_channels + channel_idx] += mono_buffer[frame_idx];
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::instruments::poly16::SynthParameters;

    #[test]
    fn test_multi_engine_creation() {
        let (_event_tx, event_rx) = unbounded();

        let params1 = Arc::new(SynthParameters::default());
        let params2 = Arc::new(SynthParameters::default());

        let instances = vec![
            (
                EngineSpec::Synth {
                    params: params1,
                    midi_channel: 0,
                },
                0,
            ), // Channel 0 -> Audio 0
            (
                EngineSpec::Synth {
                    params: params2,
                    midi_channel: 1,
                },
                1,
            ), // Channel 1 -> Audio 1
        ];

        let _multi = MultiEngineSynth::new(44100.0, instances, event_rx);

        // If it doesn't panic, creation succeeded
    }

    #[test]
    fn test_event_broadcasting() {
        let (event_tx, event_rx) = unbounded();

        let params1 = Arc::new(SynthParameters::default());
        let params2 = Arc::new(SynthParameters::default());

        let instances = vec![
            (
                EngineSpec::Synth {
                    params: params1,
                    midi_channel: 0,
                },
                0,
            ),
            (
                EngineSpec::Synth {
                    params: params2,
                    midi_channel: 1,
                },
                1,
            ),
        ];

        let mut multi = MultiEngineSynth::new(44100.0, instances, event_rx);

        // Send a note on event (A4 = note 69, 440 Hz)
        let _ = event_tx.try_send(SynthEvent::note_on(0, 69, 440.0, 0.8));

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
            (
                EngineSpec::Synth {
                    params: params.clone(),
                    midi_channel: 0,
                },
                0,
            ),
            (
                EngineSpec::Synth {
                    params: params.clone(),
                    midi_channel: 1,
                },
                1,
            ),
        ];

        let mut multi = MultiEngineSynth::new(44100.0, instances, event_rx);

        // Send note on channel 0 (A4 = note 69, 440 Hz)
        let _ = event_tx.try_send(SynthEvent::note_on(0, 69, 440.0, 0.8));

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
