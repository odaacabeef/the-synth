use std::sync::{atomic::Ordering, Arc};
use crate::audio::parameters::SynthParameters;
use crate::config::SynthInstanceConfig;

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

/// Multi-instance synth data
pub struct MultiInstance {
    pub config: SynthInstanceConfig,
    pub parameters: Arc<SynthParameters>,
    pub voice_states: [Option<u8>; 16],
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
        configs: Vec<SynthInstanceConfig>,
    ) -> Self {
        let multi_instances = all_parameters
            .into_iter()
            .zip(configs.into_iter())
            .map(|(params, config)| MultiInstance {
                config,
                parameters: params,
                voice_states: [None; 16],
            })
            .collect();

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
                instance.voice_states = voice_states;
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
            match selected {
                Parameter::Attack => {
                    instance.config.attack = (instance.config.attack + 0.01).min(2.0);
                }
                Parameter::Decay => {
                    instance.config.decay = (instance.config.decay + 0.01).min(2.0);
                }
                Parameter::Sustain => {
                    instance.config.sustain = (instance.config.sustain + 0.05).min(1.0);
                }
                Parameter::Release => {
                    instance.config.release = (instance.config.release + 0.05).min(5.0);
                }
                Parameter::Waveform => {
                    instance.config.wave = match instance.config.wave {
                        crate::config::WaveformSpec::Sine => crate::config::WaveformSpec::Triangle,
                        crate::config::WaveformSpec::Triangle => crate::config::WaveformSpec::Sawtooth,
                        crate::config::WaveformSpec::Sawtooth => crate::config::WaveformSpec::Square,
                        crate::config::WaveformSpec::Square => crate::config::WaveformSpec::Sine,
                    };
                }
            }
            self.sync_multi_instance_to_audio();
        }
    }

    /// Decrease selected parameter value
    pub fn decrease_value(&mut self) {
        let selected = self.selected_param;
        if let Some(instance) = self.current_instance_mut() {
            match selected {
                Parameter::Attack => {
                    instance.config.attack = (instance.config.attack - 0.01).max(0.001);
                }
                Parameter::Decay => {
                    instance.config.decay = (instance.config.decay - 0.01).max(0.001);
                }
                Parameter::Sustain => {
                    instance.config.sustain = (instance.config.sustain - 0.05).max(0.0);
                }
                Parameter::Release => {
                    instance.config.release = (instance.config.release - 0.05).max(0.001);
                }
                Parameter::Waveform => {
                    instance.config.wave = match instance.config.wave {
                        crate::config::WaveformSpec::Sine => crate::config::WaveformSpec::Square,
                        crate::config::WaveformSpec::Triangle => crate::config::WaveformSpec::Sine,
                        crate::config::WaveformSpec::Sawtooth => crate::config::WaveformSpec::Triangle,
                        crate::config::WaveformSpec::Square => crate::config::WaveformSpec::Sawtooth,
                    };
                }
            }
            self.sync_multi_instance_to_audio();
        }
    }

    /// Sync multi-instance parameters to audio thread
    fn sync_multi_instance_to_audio(&self) {
        if let Some(instance) = self.multi_instances.get(self.current_instance) {
            let params = &instance.parameters;
            params.attack.store(instance.config.attack, Ordering::Relaxed);
            params.decay.store(instance.config.decay, Ordering::Relaxed);
            params.sustain.store(instance.config.sustain, Ordering::Relaxed);
            params.release.store(instance.config.release, Ordering::Relaxed);

            let waveform = instance.config.waveform();
            params.waveform.store(waveform.to_u8(), Ordering::Relaxed);
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
