use crate::types::waveform::Waveform;
use std::sync::{atomic::Ordering, Arc};
use crate::audio::parameters::SynthParameters;
use crate::config::SynthInstanceConfig;

/// Application mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    /// Single synthesizer (legacy mode)
    Single,
    /// Multiple synthesizers (config mode)
    Multi,
}

/// UI application state
/// Tracks all editable parameters and UI state
pub struct App {
    /// Application mode
    pub mode: AppMode,

    /// === Single mode fields ===
    /// ADSR Attack time (0.001 to 2.0 seconds)
    pub attack: f32,
    /// ADSR Decay time (0.001 to 2.0 seconds)
    pub decay: f32,
    /// ADSR Sustain level (0.0 to 1.0)
    pub sustain: f32,
    /// ADSR Release time (0.001 to 5.0 seconds)
    pub release: f32,
    /// Current waveform
    pub waveform: Waveform,
    /// MIDI channel filter (None = omni/all channels, Some(0-15) = specific channel)
    pub midi_channel: Option<u8>,
    /// Currently selected parameter for editing
    pub selected_param: Parameter,
    /// Voice states: note number for each of 16 voices (None if idle)
    pub voice_states: [Option<u8>; 16],
    /// Reference to shared parameters (single mode)
    pub parameters: Arc<SynthParameters>,

    /// === Multi mode fields ===
    /// Multiple synth instances (config mode)
    pub multi_instances: Vec<MultiInstance>,
    /// Currently selected instance index
    pub current_instance: usize,

    /// === Common fields ===
    /// Whether to quit the application
    pub should_quit: bool,
    /// Whether to show help screen
    pub show_help: bool,
}

/// Multi-instance synth data
pub struct MultiInstance {
    pub name: String,
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
    /// Create new app in single mode (legacy)
    pub fn new(parameters: Arc<SynthParameters>, midi_channel: Option<u8>) -> Self {
        Self {
            mode: AppMode::Single,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.4,
            release: 0.1,
            waveform: Waveform::Sine,
            midi_channel,
            selected_param: Parameter::Attack,
            voice_states: [None; 16],
            parameters,
            multi_instances: Vec::new(),
            current_instance: 0,
            should_quit: false,
            show_help: false,
        }
    }

    /// Create new app in multi-instance mode (config mode)
    pub fn new_multi_instance(
        all_parameters: Vec<Arc<SynthParameters>>,
        configs: Vec<SynthInstanceConfig>,
    ) -> Self {
        let multi_instances = all_parameters
            .into_iter()
            .zip(configs.into_iter())
            .map(|(params, config)| MultiInstance {
                name: config.name.clone(),
                config,
                parameters: params,
                voice_states: [None; 16],
            })
            .collect();

        Self {
            mode: AppMode::Multi,
            // Single mode fields (unused in multi mode)
            attack: 0.0,
            decay: 0.0,
            sustain: 0.0,
            release: 0.0,
            waveform: Waveform::Sine,
            midi_channel: None,
            selected_param: Parameter::Attack,
            voice_states: [None; 16],
            parameters: Arc::new(SynthParameters::default()),
            // Multi mode fields
            multi_instances,
            current_instance: 0,
            // Common fields
            should_quit: false,
            show_help: false,
        }
    }

    /// Get current instance (multi mode)
    pub fn current_instance(&self) -> Option<&MultiInstance> {
        if self.mode == AppMode::Multi {
            self.multi_instances.get(self.current_instance)
        } else {
            None
        }
    }

    /// Get current instance mut (multi mode)
    fn current_instance_mut(&mut self) -> Option<&mut MultiInstance> {
        if self.mode == AppMode::Multi {
            self.multi_instances.get_mut(self.current_instance)
        } else {
            None
        }
    }

    /// Next instance (multi mode)
    pub fn next_instance(&mut self) {
        if self.mode == AppMode::Multi && !self.multi_instances.is_empty() {
            self.current_instance = (self.current_instance + 1) % self.multi_instances.len();
        }
    }

    /// Previous instance (multi mode)
    pub fn prev_instance(&mut self) {
        if self.mode == AppMode::Multi && !self.multi_instances.is_empty() {
            if self.current_instance == 0 {
                self.current_instance = self.multi_instances.len() - 1;
            } else {
                self.current_instance -= 1;
            }
        }
    }

    /// Update multi-instance voice states
    pub fn update_multi_voice_states(&mut self, states: Vec<[Option<u8>; 16]>) {
        if self.mode == AppMode::Multi {
            for (idx, voice_states) in states.into_iter().enumerate() {
                if let Some(instance) = self.multi_instances.get_mut(idx) {
                    instance.voice_states = voice_states;
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
        match self.mode {
            AppMode::Single => {
                match self.selected_param {
                    Parameter::Attack => {
                        self.attack = (self.attack + 0.01).min(2.0);
                        self.sync_to_audio();
                    }
                    Parameter::Decay => {
                        self.decay = (self.decay + 0.01).min(2.0);
                        self.sync_to_audio();
                    }
                    Parameter::Sustain => {
                        self.sustain = (self.sustain + 0.05).min(1.0);
                        self.sync_to_audio();
                    }
                    Parameter::Release => {
                        self.release = (self.release + 0.05).min(5.0);
                        self.sync_to_audio();
                    }
                    Parameter::Waveform => {
                        self.waveform = match self.waveform {
                            Waveform::Sine => Waveform::Triangle,
                            Waveform::Triangle => Waveform::Sawtooth,
                            Waveform::Sawtooth => Waveform::Square,
                            Waveform::Square => Waveform::Sine,
                        };
                        self.sync_to_audio();
                    }
                }
            }
            AppMode::Multi => {
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
                            instance.config.waveform = match instance.config.waveform {
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
        }
    }

    /// Decrease selected parameter value
    pub fn decrease_value(&mut self) {
        match self.mode {
            AppMode::Single => {
                match self.selected_param {
                    Parameter::Attack => {
                        self.attack = (self.attack - 0.01).max(0.001);
                        self.sync_to_audio();
                    }
                    Parameter::Decay => {
                        self.decay = (self.decay - 0.01).max(0.001);
                        self.sync_to_audio();
                    }
                    Parameter::Sustain => {
                        self.sustain = (self.sustain - 0.05).max(0.0);
                        self.sync_to_audio();
                    }
                    Parameter::Release => {
                        self.release = (self.release - 0.05).max(0.001);
                        self.sync_to_audio();
                    }
                    Parameter::Waveform => {
                        self.waveform = match self.waveform {
                            Waveform::Sine => Waveform::Square,
                            Waveform::Triangle => Waveform::Sine,
                            Waveform::Sawtooth => Waveform::Triangle,
                            Waveform::Square => Waveform::Sawtooth,
                        };
                        self.sync_to_audio();
                    }
                }
            }
            AppMode::Multi => {
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
                            instance.config.waveform = match instance.config.waveform {
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

    /// Sync UI parameters to audio thread (via atomics)
    fn sync_to_audio(&self) {
        self.parameters.attack.store(self.attack, Ordering::Relaxed);
        self.parameters.decay.store(self.decay, Ordering::Relaxed);
        self.parameters.sustain.store(self.sustain, Ordering::Relaxed);
        self.parameters.release.store(self.release, Ordering::Relaxed);
        self.parameters.waveform.store(self.waveform.to_u8(), Ordering::Relaxed);

        // Convert Option<u8> to u8: None = 255 (omni), Some(ch) = ch
        let channel_value = self.midi_channel.unwrap_or(255);
        self.parameters.midi_channel.store(channel_value, Ordering::Relaxed);
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
