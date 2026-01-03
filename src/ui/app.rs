use std::sync::{atomic::Ordering, Arc};
use crate::audio::parameters::SynthParameters;
use crate::config::{DrumInstanceConfig, SynthInstanceConfig};

/// UI application state
/// Tracks all editable parameters and UI state
pub struct App {
    /// Currently selected parameter for editing
    pub selected_param: Parameter,
    /// Multiple synth instances
    pub multi_instances: Vec<MultiInstance>,
    /// Currently selected instance index
    pub current_instance: usize,
    /// Whether to quit the application
    pub should_quit: bool,
    /// Whether to show help screen
    pub show_help: bool,
}

/// Multi-instance instrument data - can be either Synth or Drum
pub enum MultiInstance {
    Synth {
        config: SynthInstanceConfig,
        parameters: Arc<SynthParameters>,
        voice_states: [Option<u8>; 16],
    },
    Drum {
        config: DrumInstanceConfig,
        voice_state: Option<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parameter {
    Attack,
    Decay,
    Sustain,
    Release,
    Waveform,
}

impl App {
    /// Create new app in multi-instance mode (config mode)
    pub fn new_multi_instance(
        all_parameters: Vec<Arc<SynthParameters>>,
        synth_configs: Vec<SynthInstanceConfig>,
        drum_configs: Vec<DrumInstanceConfig>,
    ) -> Self {
        let mut multi_instances = Vec::new();

        // Add synth instances
        for (params, config) in all_parameters.into_iter().zip(synth_configs.into_iter()) {
            multi_instances.push(MultiInstance::Synth {
                config,
                parameters: params,
                voice_states: [None; 16],
            });
        }

        // Add drum instances
        for config in drum_configs {
            multi_instances.push(MultiInstance::Drum {
                config,
                voice_state: None,
            });
        }

        Self {
            selected_param: Parameter::Attack,
            multi_instances,
            current_instance: 0,
            should_quit: false,
            show_help: false,
        }
    }

    /// Get current instance mut (multi mode)
    fn current_instance_mut(&mut self) -> Option<&mut MultiInstance> {
        self.multi_instances.get_mut(self.current_instance)
    }

    /// Next instance
    pub fn next_instance(&mut self) {
        if !self.multi_instances.is_empty() {
            self.current_instance = (self.current_instance + 1) % self.multi_instances.len();
        }
    }

    /// Previous instance
    pub fn prev_instance(&mut self) {
        if !self.multi_instances.is_empty() {
            if self.current_instance == 0 {
                self.current_instance = self.multi_instances.len() - 1;
            } else {
                self.current_instance -= 1;
            }
        }
    }

    /// Update multi-instance voice states
    pub fn update_multi_voice_states(&mut self, states: Vec<[Option<u8>; 16]>) {
        for (idx, voice_states) in states.into_iter().enumerate() {
            if let Some(instance) = self.multi_instances.get_mut(idx) {
                match instance {
                    MultiInstance::Synth {
                        voice_states: vs, ..
                    } => {
                        *vs = voice_states;
                    }
                    MultiInstance::Drum { voice_state: vs, .. } => {
                        // For drums, take the first active voice (if any)
                        *vs = voice_states.iter().find(|v| v.is_some()).copied().flatten();
                    }
                }
            }
        }
    }

    /// Cycle to next parameter
    pub fn next_parameter(&mut self) {
        self.selected_param = match self.selected_param {
            Parameter::Attack => Parameter::Decay,
            Parameter::Decay => Parameter::Sustain,
            Parameter::Sustain => Parameter::Release,
            Parameter::Release => Parameter::Waveform,
            Parameter::Waveform => Parameter::Attack,
        };
    }

    /// Cycle to previous parameter
    pub fn prev_parameter(&mut self) {
        self.selected_param = match self.selected_param {
            Parameter::Attack => Parameter::Waveform,
            Parameter::Decay => Parameter::Attack,
            Parameter::Sustain => Parameter::Decay,
            Parameter::Release => Parameter::Sustain,
            Parameter::Waveform => Parameter::Release,
        };
    }

    /// Increase selected parameter value
    pub fn increase_value(&mut self) {
        let selected = self.selected_param;
        if let Some(instance) = self.current_instance_mut() {
            // Only synths have adjustable parameters
            if let MultiInstance::Synth {
                config,
                ..
            } = instance
            {
                match selected {
                    Parameter::Attack => {
                        config.attack = (config.attack + 0.01).min(2.0);
                    }
                    Parameter::Decay => {
                        config.decay = (config.decay + 0.01).min(2.0);
                    }
                    Parameter::Sustain => {
                        config.sustain = (config.sustain + 0.05).min(1.0);
                    }
                    Parameter::Release => {
                        config.release = (config.release + 0.05).min(5.0);
                    }
                    Parameter::Waveform => {
                        config.wave = match config.wave {
                            crate::config::WaveformSpec::Sine => {
                                crate::config::WaveformSpec::Triangle
                            }
                            crate::config::WaveformSpec::Triangle => {
                                crate::config::WaveformSpec::Sawtooth
                            }
                            crate::config::WaveformSpec::Sawtooth => {
                                crate::config::WaveformSpec::Square
                            }
                            crate::config::WaveformSpec::Square => {
                                crate::config::WaveformSpec::Sine
                            }
                        };
                    }
                }
                self.sync_multi_instance_to_audio();
            }
        }
    }

    /// Decrease selected parameter value
    pub fn decrease_value(&mut self) {
        let selected = self.selected_param;
        if let Some(instance) = self.current_instance_mut() {
            // Only synths have adjustable parameters
            if let MultiInstance::Synth {
                config,
                ..
            } = instance
            {
                match selected {
                    Parameter::Attack => {
                        config.attack = (config.attack - 0.01).max(0.001);
                    }
                    Parameter::Decay => {
                        config.decay = (config.decay - 0.01).max(0.001);
                    }
                    Parameter::Sustain => {
                        config.sustain = (config.sustain - 0.05).max(0.0);
                    }
                    Parameter::Release => {
                        config.release = (config.release - 0.05).max(0.001);
                    }
                    Parameter::Waveform => {
                        config.wave = match config.wave {
                            crate::config::WaveformSpec::Sine => {
                                crate::config::WaveformSpec::Square
                            }
                            crate::config::WaveformSpec::Triangle => {
                                crate::config::WaveformSpec::Sine
                            }
                            crate::config::WaveformSpec::Sawtooth => {
                                crate::config::WaveformSpec::Triangle
                            }
                            crate::config::WaveformSpec::Square => {
                                crate::config::WaveformSpec::Sawtooth
                            }
                        };
                    }
                }
                self.sync_multi_instance_to_audio();
            }
        }
    }

    /// Sync multi-instance parameters to audio thread
    fn sync_multi_instance_to_audio(&self) {
        if let Some(instance) = self.multi_instances.get(self.current_instance) {
            // Only synths have parameters to sync
            if let MultiInstance::Synth {
                config,
                parameters,
                ..
            } = instance
            {
                parameters
                    .attack
                    .store(config.attack, Ordering::Relaxed);
                parameters.decay.store(config.decay, Ordering::Relaxed);
                parameters
                    .sustain
                    .store(config.sustain, Ordering::Relaxed);
                parameters
                    .release
                    .store(config.release, Ordering::Relaxed);

                let waveform = config.waveform();
                parameters
                    .waveform
                    .store(waveform.to_u8(), Ordering::Relaxed);
            }
        }
    }

    /// Mark app for quit
    pub fn quit(&mut self) {
        self.should_quit = true;
    }

    /// Toggle help screen visibility
    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }
}
