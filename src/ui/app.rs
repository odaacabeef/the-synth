use std::sync::{atomic::Ordering, Arc};
use crate::instruments::poly16::SynthParameters;
use crate::config::{CVInstanceConfig, DrumInstanceConfig, SynthInstanceConfig};
use crate::instruments::drums::{DrumParameters, DrumType};
use crate::instruments::cv::CVParameters;

/// UI application state
/// Tracks all editable parameters and UI state
pub struct App {
    /// Currently selected parameter for editing (synths)
    pub selected_param: Parameter,
    /// Currently selected drum parameter for editing (drums)
    pub selected_drum_param: DrumParameter,
    /// Currently selected CV parameter for editing (CVs)
    pub selected_cv_param: CVParameter,
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
        parameters: DrumParameters,
        voice_state: Option<u8>,
    },
    CV {
        config: CVInstanceConfig,
        parameters: Arc<CVParameters>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrumParameter {
    // Kick parameters
    KickPitchStart,
    KickPitchEnd,
    KickPitchDecay,
    KickDecay,
    KickClick,
    // Snare parameters
    SnareToneFreq,
    SnareToneMix,
    SnareDecay,
    SnareSnap,
    // Hat parameters
    HatBrightness,
    HatDecay,
    HatMetallic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CVParameter {
    Transpose,
    Glide,
}

impl App {
    /// Create new app in multi-instance mode (config mode)
    pub fn new_multi_instance(
        all_parameters: Vec<Arc<SynthParameters>>,
        synth_configs: Vec<SynthInstanceConfig>,
        drum_parameters: Vec<DrumParameters>,
        drum_configs: Vec<DrumInstanceConfig>,
        cv_parameters: Vec<Arc<CVParameters>>,
        cv_configs: Vec<CVInstanceConfig>,
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
        for (params, config) in drum_parameters.into_iter().zip(drum_configs.into_iter()) {
            multi_instances.push(MultiInstance::Drum {
                config,
                parameters: params,
                voice_state: None,
            });
        }

        // Add CV instances
        for (params, config) in cv_parameters.into_iter().zip(cv_configs.into_iter()) {
            multi_instances.push(MultiInstance::CV {
                config,
                parameters: params,
                voice_state: None,
            });
        }

        // Initialize selected_drum_param based on first drum type (if any)
        let selected_drum_param = multi_instances
            .iter()
            .find_map(|inst| {
                if let MultiInstance::Drum { config, .. } = inst {
                    Some(match config.drum_type {
                        DrumType::Kick => DrumParameter::KickPitchStart,
                        DrumType::Snare => DrumParameter::SnareToneFreq,
                        DrumType::Hat => DrumParameter::HatBrightness,
                    })
                } else {
                    None
                }
            })
            .unwrap_or(DrumParameter::KickPitchStart); // Default to kick if no drums

        Self {
            selected_param: Parameter::Attack,
            selected_drum_param,
            selected_cv_param: CVParameter::Transpose,
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
            // Save current parameter index before switching
            let current_index = self.get_current_param_index();

            self.current_instance = (self.current_instance + 1) % self.multi_instances.len();

            // Restore parameter index (or max if out of bounds)
            self.set_param_by_index(current_index);
        }
    }

    /// Previous instance
    pub fn prev_instance(&mut self) {
        if !self.multi_instances.is_empty() {
            // Save current parameter index before switching
            let current_index = self.get_current_param_index();

            if self.current_instance == 0 {
                self.current_instance = self.multi_instances.len() - 1;
            } else {
                self.current_instance -= 1;
            }

            // Restore parameter index (or max if out of bounds)
            self.set_param_by_index(current_index);
        }
    }

    /// Jump to first instance
    pub fn jump_to_first(&mut self) {
        if !self.multi_instances.is_empty() {
            // Save current parameter index before switching
            let current_index = self.get_current_param_index();

            self.current_instance = 0;

            // Restore parameter index (or max if out of bounds)
            self.set_param_by_index(current_index);
        }
    }

    /// Jump to last instance
    pub fn jump_to_last(&mut self) {
        if !self.multi_instances.is_empty() {
            // Save current parameter index before switching
            let current_index = self.get_current_param_index();

            self.current_instance = self.multi_instances.len() - 1;

            // Restore parameter index (or max if out of bounds)
            self.set_param_by_index(current_index);
        }
    }

    /// Get the current parameter index (0-4)
    fn get_current_param_index(&self) -> usize {
        if let Some(instance) = self.multi_instances.get(self.current_instance) {
            match instance {
                MultiInstance::Synth { .. } => match self.selected_param {
                    Parameter::Attack => 0,
                    Parameter::Decay => 1,
                    Parameter::Sustain => 2,
                    Parameter::Release => 3,
                    Parameter::Waveform => 4,
                },
                MultiInstance::Drum { config, .. } => match config.drum_type {
                    DrumType::Kick => match self.selected_drum_param {
                        DrumParameter::KickPitchStart => 0,
                        DrumParameter::KickPitchEnd => 1,
                        DrumParameter::KickPitchDecay => 2,
                        DrumParameter::KickDecay => 3,
                        DrumParameter::KickClick => 4,
                        _ => 0,
                    },
                    DrumType::Snare => match self.selected_drum_param {
                        DrumParameter::SnareToneFreq => 0,
                        DrumParameter::SnareToneMix => 1,
                        DrumParameter::SnareDecay => 2,
                        DrumParameter::SnareSnap => 3,
                        _ => 0,
                    },
                    DrumType::Hat => match self.selected_drum_param {
                        DrumParameter::HatBrightness => 0,
                        DrumParameter::HatDecay => 1,
                        DrumParameter::HatMetallic => 2,
                        _ => 0,
                    },
                },
                MultiInstance::CV { .. } => match self.selected_cv_param {
                    CVParameter::Transpose => 0,
                    CVParameter::Glide => 1,
                },
            }
        } else {
            0
        }
    }

    /// Set parameter by index (0-4), clamping to max available for the instance
    fn set_param_by_index(&mut self, index: usize) {
        if let Some(instance) = self.multi_instances.get(self.current_instance) {
            match instance {
                MultiInstance::Synth { .. } => {
                    // Synths have 5 parameters (0-4)
                    self.selected_param = match index {
                        0 => Parameter::Attack,
                        1 => Parameter::Decay,
                        2 => Parameter::Sustain,
                        3 => Parameter::Release,
                        _ => Parameter::Waveform, // 4 or higher
                    };
                }
                MultiInstance::Drum { config, .. } => {
                    self.selected_drum_param = match config.drum_type {
                        DrumType::Kick => {
                            // Kick has 5 parameters (0-4)
                            match index {
                                0 => DrumParameter::KickPitchStart,
                                1 => DrumParameter::KickPitchEnd,
                                2 => DrumParameter::KickPitchDecay,
                                3 => DrumParameter::KickDecay,
                                _ => DrumParameter::KickClick, // 4 or higher
                            }
                        }
                        DrumType::Snare => {
                            // Snare has 4 parameters (0-3)
                            match index {
                                0 => DrumParameter::SnareToneFreq,
                                1 => DrumParameter::SnareToneMix,
                                2 => DrumParameter::SnareDecay,
                                _ => DrumParameter::SnareSnap, // 3 or higher
                            }
                        }
                        DrumType::Hat => {
                            // Hat has 3 parameters (0-2)
                            match index {
                                0 => DrumParameter::HatBrightness,
                                1 => DrumParameter::HatDecay,
                                _ => DrumParameter::HatMetallic, // 2 or higher
                            }
                        }
                    };
                }
                MultiInstance::CV { .. } => {
                    // CV has 2 parameters (0-1)
                    self.selected_cv_param = match index {
                        0 => CVParameter::Transpose,
                        _ => CVParameter::Glide, // 1 or higher
                    };
                }
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
                    MultiInstance::CV { voice_state: vs, .. } => {
                        // For CV, take the first active voice (monophonic)
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

    /// Cycle to next drum parameter (based on drum type)
    pub fn next_drum_parameter(&mut self, drum_type: DrumType) {
        self.selected_drum_param = match drum_type {
            DrumType::Kick => match self.selected_drum_param {
                DrumParameter::KickPitchStart => DrumParameter::KickPitchEnd,
                DrumParameter::KickPitchEnd => DrumParameter::KickPitchDecay,
                DrumParameter::KickPitchDecay => DrumParameter::KickDecay,
                DrumParameter::KickDecay => DrumParameter::KickClick,
                DrumParameter::KickClick => DrumParameter::KickPitchStart,
                _ => DrumParameter::KickPitchStart, // Default to first if not a kick param
            },
            DrumType::Snare => match self.selected_drum_param {
                DrumParameter::SnareToneFreq => DrumParameter::SnareToneMix,
                DrumParameter::SnareToneMix => DrumParameter::SnareDecay,
                DrumParameter::SnareDecay => DrumParameter::SnareSnap,
                DrumParameter::SnareSnap => DrumParameter::SnareToneFreq,
                _ => DrumParameter::SnareToneFreq, // Default to first if not a snare param
            },
            DrumType::Hat => match self.selected_drum_param {
                DrumParameter::HatBrightness => DrumParameter::HatDecay,
                DrumParameter::HatDecay => DrumParameter::HatMetallic,
                DrumParameter::HatMetallic => DrumParameter::HatBrightness,
                _ => DrumParameter::HatBrightness, // Default to first if not a hat param
            },
        };
    }

    /// Cycle to previous drum parameter (based on drum type)
    pub fn prev_drum_parameter(&mut self, drum_type: DrumType) {
        self.selected_drum_param = match drum_type {
            DrumType::Kick => match self.selected_drum_param {
                DrumParameter::KickPitchStart => DrumParameter::KickClick,
                DrumParameter::KickPitchEnd => DrumParameter::KickPitchStart,
                DrumParameter::KickPitchDecay => DrumParameter::KickPitchEnd,
                DrumParameter::KickDecay => DrumParameter::KickPitchDecay,
                DrumParameter::KickClick => DrumParameter::KickDecay,
                _ => DrumParameter::KickPitchStart, // Default to first if not a kick param
            },
            DrumType::Snare => match self.selected_drum_param {
                DrumParameter::SnareToneFreq => DrumParameter::SnareSnap,
                DrumParameter::SnareToneMix => DrumParameter::SnareToneFreq,
                DrumParameter::SnareDecay => DrumParameter::SnareToneMix,
                DrumParameter::SnareSnap => DrumParameter::SnareDecay,
                _ => DrumParameter::SnareToneFreq, // Default to first if not a snare param
            },
            DrumType::Hat => match self.selected_drum_param {
                DrumParameter::HatBrightness => DrumParameter::HatMetallic,
                DrumParameter::HatDecay => DrumParameter::HatBrightness,
                DrumParameter::HatMetallic => DrumParameter::HatDecay,
                _ => DrumParameter::HatBrightness, // Default to first if not a hat param
            },
        };
    }

    /// Cycle to next CV parameter
    pub fn next_cv_parameter(&mut self) {
        self.selected_cv_param = match self.selected_cv_param {
            CVParameter::Transpose => CVParameter::Glide,
            CVParameter::Glide => CVParameter::Transpose,
        };
    }

    /// Cycle to previous CV parameter
    pub fn prev_cv_parameter(&mut self) {
        self.selected_cv_param = match self.selected_cv_param {
            CVParameter::Transpose => CVParameter::Glide,
            CVParameter::Glide => CVParameter::Transpose,
        };
    }

    /// Increase selected parameter value
    pub fn increase_value(&mut self) {
        let selected = self.selected_param;
        let selected_drum_param = self.selected_drum_param;
        let selected_cv_param = self.selected_cv_param;

        if let Some(instance) = self.current_instance_mut() {
            match instance {
                MultiInstance::Synth { config, .. } => {
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
                MultiInstance::Drum { config, .. } => {
                    match config.drum_type {
                        DrumType::Kick => match selected_drum_param {
                            DrumParameter::KickPitchStart => {
                                config.kick_pitch_start = (config.kick_pitch_start + 5.0).min(300.0);
                            }
                            DrumParameter::KickPitchEnd => {
                                config.kick_pitch_end = (config.kick_pitch_end + 2.0).min(100.0);
                            }
                            DrumParameter::KickPitchDecay => {
                                config.kick_pitch_decay = (config.kick_pitch_decay + 0.01).min(0.2);
                            }
                            DrumParameter::KickDecay => {
                                config.kick_decay = (config.kick_decay + 0.05).min(1.0);
                            }
                            DrumParameter::KickClick => {
                                config.kick_click = (config.kick_click + 0.05).min(1.0);
                            }
                            _ => {}
                        },
                        DrumType::Snare => match selected_drum_param {
                            DrumParameter::SnareToneFreq => {
                                config.snare_tone_freq = (config.snare_tone_freq + 5.0).min(300.0);
                            }
                            DrumParameter::SnareToneMix => {
                                config.snare_tone_mix = (config.snare_tone_mix + 0.05).min(1.0);
                            }
                            DrumParameter::SnareDecay => {
                                config.snare_decay = (config.snare_decay + 0.01).min(0.5);
                            }
                            DrumParameter::SnareSnap => {
                                config.snare_snap = (config.snare_snap + 0.05).min(1.0);
                            }
                            _ => {}
                        },
                        DrumType::Hat => match selected_drum_param {
                            DrumParameter::HatBrightness => {
                                config.hat_brightness = (config.hat_brightness + 100.0).min(12000.0);
                            }
                            DrumParameter::HatDecay => {
                                config.hat_decay = (config.hat_decay + 0.01).min(0.5);
                            }
                            DrumParameter::HatMetallic => {
                                config.hat_metallic = (config.hat_metallic + 0.05).min(1.0);
                            }
                            _ => {}
                        },
                    }
                    self.sync_multi_instance_to_audio();
                }
                MultiInstance::CV { config, .. } => {
                    match selected_cv_param {
                        CVParameter::Transpose => {
                            config.transpose = (config.transpose + 1).min(24);
                        }
                        CVParameter::Glide => {
                            config.glide = (config.glide + 0.05).min(2.0);
                        }
                    }
                    self.sync_multi_instance_to_audio();
                }
            }
        }
    }

    /// Decrease selected parameter value
    pub fn decrease_value(&mut self) {
        let selected = self.selected_param;
        let selected_drum_param = self.selected_drum_param;
        let selected_cv_param = self.selected_cv_param;

        if let Some(instance) = self.current_instance_mut() {
            match instance {
                MultiInstance::Synth { config, .. } => {
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
                MultiInstance::Drum { config, .. } => {
                    match config.drum_type {
                        DrumType::Kick => match selected_drum_param {
                            DrumParameter::KickPitchStart => {
                                config.kick_pitch_start = (config.kick_pitch_start - 5.0).max(100.0);
                            }
                            DrumParameter::KickPitchEnd => {
                                config.kick_pitch_end = (config.kick_pitch_end - 2.0).max(30.0);
                            }
                            DrumParameter::KickPitchDecay => {
                                config.kick_pitch_decay = (config.kick_pitch_decay - 0.01).max(0.01);
                            }
                            DrumParameter::KickDecay => {
                                config.kick_decay = (config.kick_decay - 0.05).max(0.1);
                            }
                            DrumParameter::KickClick => {
                                config.kick_click = (config.kick_click - 0.05).max(0.0);
                            }
                            _ => {}
                        },
                        DrumType::Snare => match selected_drum_param {
                            DrumParameter::SnareToneFreq => {
                                config.snare_tone_freq = (config.snare_tone_freq - 5.0).max(150.0);
                            }
                            DrumParameter::SnareToneMix => {
                                config.snare_tone_mix = (config.snare_tone_mix - 0.05).max(0.0);
                            }
                            DrumParameter::SnareDecay => {
                                config.snare_decay = (config.snare_decay - 0.01).max(0.05);
                            }
                            DrumParameter::SnareSnap => {
                                config.snare_snap = (config.snare_snap - 0.05).max(0.0);
                            }
                            _ => {}
                        },
                        DrumType::Hat => match selected_drum_param {
                            DrumParameter::HatBrightness => {
                                config.hat_brightness = (config.hat_brightness - 100.0).max(5000.0);
                            }
                            DrumParameter::HatDecay => {
                                config.hat_decay = (config.hat_decay - 0.01).max(0.02);
                            }
                            DrumParameter::HatMetallic => {
                                config.hat_metallic = (config.hat_metallic - 0.05).max(0.0);
                            }
                            _ => {}
                        },
                    }
                    self.sync_multi_instance_to_audio();
                }
                MultiInstance::CV { config, .. } => {
                    match selected_cv_param {
                        CVParameter::Transpose => {
                            config.transpose = (config.transpose - 1).max(-24);
                        }
                        CVParameter::Glide => {
                            config.glide = (config.glide - 0.05).max(0.0);
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
            match instance {
                MultiInstance::Synth {
                    config,
                    parameters,
                    ..
                } => {
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
                MultiInstance::Drum {
                    config,
                    parameters,
                    ..
                } => {
                    // Sync drum parameters to audio thread
                    match parameters {
                        DrumParameters::Kick(kick_params) => {
                            kick_params.pitch_start.store(config.kick_pitch_start, Ordering::Relaxed);
                            kick_params.pitch_end.store(config.kick_pitch_end, Ordering::Relaxed);
                            kick_params.pitch_decay.store(config.kick_pitch_decay, Ordering::Relaxed);
                            kick_params.decay.store(config.kick_decay, Ordering::Relaxed);
                            kick_params.click.store(config.kick_click, Ordering::Relaxed);
                        }
                        DrumParameters::Snare(snare_params) => {
                            snare_params.tone_freq.store(config.snare_tone_freq, Ordering::Relaxed);
                            snare_params.tone_mix.store(config.snare_tone_mix, Ordering::Relaxed);
                            snare_params.decay.store(config.snare_decay, Ordering::Relaxed);
                            snare_params.snap.store(config.snare_snap, Ordering::Relaxed);
                        }
                        DrumParameters::Hat(hat_params) => {
                            hat_params.brightness.store(config.hat_brightness, Ordering::Relaxed);
                            hat_params.decay.store(config.hat_decay, Ordering::Relaxed);
                            hat_params.metallic.store(config.hat_metallic, Ordering::Relaxed);
                        }
                    }
                }
                MultiInstance::CV {
                    config,
                    parameters,
                    ..
                } => {
                    parameters.transpose.store(config.transpose, Ordering::Relaxed);
                    parameters.glide.store(config.glide, Ordering::Relaxed);
                }
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
