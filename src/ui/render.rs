use ratatui::{
    text::Line,
    widgets::Paragraph,
    Frame,
};

use super::app::{App, CVParameter, DrumParameter, MultiInstance, Parameter};
use crate::instruments::drums::DrumType;

/// Render the TUI
pub fn render(frame: &mut Frame, app: &App) {
    // Show help screen if toggled
    if app.show_help {
        render_synthesizer_help(frame);
    } else {
        render_multi_instance(frame, app);
    }
}

/// Convert MIDI note number to note name (e.g., 60 -> "C4")
fn midi_note_to_name(note: u8) -> String {
    const NOTE_NAMES: [&str; 12] = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
    let octave = (note / 12) as i32 - 1;
    let note_name = NOTE_NAMES[(note % 12) as usize];
    format!("{}{}", note_name, octave)
}

/// Render multi-instance screen with all instances side-by-side
fn render_multi_instance(frame: &mut Frame, app: &App) {
    if app.multi_instances.is_empty() {
        let paragraph = Paragraph::new(vec![Line::from("No instances configured")]);
        frame.render_widget(paragraph, frame.size());
        return;
    }

    // Build all lines by rendering instances horizontally
    let max_lines = 20; // Enough for header + parameters + voices
    let mut combined_lines: Vec<String> = vec![String::new(); max_lines];

    // Find the index where drums start (for divider placement)
    let first_drum_idx = app.multi_instances.iter().position(|inst| {
        matches!(inst, MultiInstance::Drum { .. })
    });

    for (idx, instance) in app.multi_instances.iter().enumerate() {
        let is_selected = idx == app.current_instance;
        let instance_lines = build_instance_lines(
            instance,
            is_selected,
            app.selected_param,
            app.selected_drum_param,
            app.selected_cv_param,
        );

        // Determine spacing: add divider before first drum
        let spacing = if idx == 0 {
            ""
        } else if Some(idx) == first_drum_idx {
            "  :" // Divider between poly16 and drums (2 spaces + :)
        } else {
            " " // Regular spacing
        };

        // Merge instance lines horizontally
        for (line_idx, line) in instance_lines.iter().enumerate() {
            if line_idx < max_lines {
                combined_lines[line_idx].push_str(spacing);
                combined_lines[line_idx].push_str(line);
            }
        }
    }

    // Remove trailing empty lines (including lines with only whitespace and divider)
    while combined_lines.last().map_or(false, |line| {
        let trimmed = line.trim();
        trimmed.is_empty() || trimmed == ":"
    }) {
        combined_lines.pop();
    }

    // Convert to ratatui Lines and render
    let lines: Vec<Line> = combined_lines
        .into_iter()
        .map(|s| Line::from(s))
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, frame.size());
}

/// Build lines for a single instrument instance (synth, drum, or CV)
fn build_instance_lines(
    instance: &MultiInstance,
    is_selected: bool,
    selected_param: Parameter,
    selected_drum_param: DrumParameter,
    selected_cv_param: CVParameter,
) -> Vec<String> {
    match instance {
        MultiInstance::Synth {
            config,
            voice_states,
            ..
        } => build_synth_lines(config, voice_states, is_selected, selected_param),
        MultiInstance::Drum {
            config,
            voice_state,
            last_trigger,
            ..
        } => build_drum_lines(config, *voice_state, *last_trigger, is_selected, selected_drum_param),
        MultiInstance::CV {
            config,
            voice_state,
            ..
        } => build_cv_lines(config, *voice_state, is_selected, selected_cv_param),
    }
}

/// Build lines for a synth instance
fn build_synth_lines(
    config: &crate::config::SynthInstanceConfig,
    voice_states: &[Option<u8>; 16],
    is_selected: bool,
    selected_param: Parameter,
) -> Vec<String> {
    let mut lines = Vec::new();

    // MIDI channel string (1-indexed for display)
    let midi_ch_str = if config.midi_channel_filter() == 255 {
        "omni".to_string()
    } else {
        format!("{}", config.midi_channel_filter() + 1)
    };

    // Title line: m<midi>:a<audio>
    lines.push(format!("  m{}:a{}", midi_ch_str, config.audioch));
    lines.push(String::new()); // Blank line

    // ADSR parameters (only show cursor if this instance is selected)
    let cursor_attack = if is_selected && selected_param == Parameter::Attack {
        ">"
    } else {
        " "
    };
    lines.push(format!("{} Attack:  {:.3}s", cursor_attack, config.attack));

    let cursor_decay = if is_selected && selected_param == Parameter::Decay {
        ">"
    } else {
        " "
    };
    lines.push(format!("{} Decay:   {:.3}s", cursor_decay, config.decay));

    let cursor_sustain = if is_selected && selected_param == Parameter::Sustain {
        ">"
    } else {
        " "
    };
    lines.push(format!("{} Sustain: {:.2}", cursor_sustain, config.sustain));

    let cursor_release = if is_selected && selected_param == Parameter::Release {
        ">"
    } else {
        " "
    };
    lines.push(format!(
        "{} Release: {:.3}s",
        cursor_release, config.release
    ));

    lines.push(String::new()); // Blank line

    // Waveform
    let cursor_waveform = if is_selected && selected_param == Parameter::Waveform {
        ">"
    } else {
        " "
    };
    let waveform = config.waveform();
    let waveform_str = match waveform {
        crate::types::waveform::Waveform::Sine => "Sine",
        crate::types::waveform::Waveform::Triangle => "Triangle",
        crate::types::waveform::Waveform::Sawtooth => "Sawtooth",
        crate::types::waveform::Waveform::Square => "Square",
    };
    lines.push(format!("{} Wave: {}", cursor_waveform, waveform_str));

    lines.push(String::new()); // Blank line

    // Voice states (4 rows of 4 voices each)
    for row in 0..4 {
        let mut voice_line = String::from("  ");
        for col in 0..4 {
            if col > 0 {
                voice_line.push(' ');
            }
            let voice_idx = row * 4 + col;
            match voice_states[voice_idx] {
                Some(note) => {
                    let note_name = midi_note_to_name(note);
                    voice_line.push_str(&format!("{:3}", note_name));
                }
                None => voice_line.push_str("---"),
            }
        }
        lines.push(voice_line);
    }

    pad_lines(&mut lines);
    lines
}

/// Build lines for a drum instance
fn build_drum_lines(
    config: &crate::config::DrumInstanceConfig,
    voice_state: Option<u8>,
    last_trigger: Option<std::time::Instant>,
    is_selected: bool,
    selected_drum_param: DrumParameter,
) -> Vec<String> {
    let mut lines = Vec::new();

    // MIDI channel string (1-indexed for display)
    let midi_ch_str = if config.midi_channel_filter() == 255 {
        "omni".to_string()
    } else {
        format!("{}", config.midi_channel_filter() + 1)
    };

    // Title line: m<midi>:a<audio>
    lines.push(format!("  m{}:a{}", midi_ch_str, config.audioch));
    lines.push(String::new()); // Blank line

    // Parameters based on drum type
    match config.drum_type {
        DrumType::Kick => {
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::KickPitchStart, "PitchStart", format!("{}Hz", config.kick_pitch_start));
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::KickPitchEnd, "PitchEnd", format!("{}Hz", config.kick_pitch_end));
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::KickPitchDecay, "PitchDecay", format!("{:.3}s", config.kick_pitch_decay));
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::KickDecay, "Decay", format!("{:.3}s", config.kick_decay));
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::KickClick, "Click", format!("{:.2}", config.kick_click));
        }
        DrumType::Snare => {
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::SnareToneFreq, "ToneFreq", format!("{}Hz", config.snare_tone_freq));
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::SnareToneMix, "ToneMix", format!("{:.2}", config.snare_tone_mix));
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::SnareDecay, "Decay", format!("{:.3}s", config.snare_decay));
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::SnareSnap, "Snap", format!("{:.2}", config.snare_snap));
            lines.push(String::new()); // Blank line (snare has 4 params, need 5 lines)
        }
        DrumType::Hat => {
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::HatBrightness, "Brightness", format!("{}Hz", config.hat_brightness));
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::HatDecay, "Decay", format!("{:.3}s", config.hat_decay));
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::HatMetallic, "Metallic", format!("{:.2}", config.hat_metallic));
            lines.push(String::new()); // Blank line (hat has 3 params, need 5 lines)
            lines.push(String::new()); // Another blank
        }
    }

    lines.push(String::new()); // Blank line (line 8)
    lines.push(String::new()); // Blank line (line 9, where synth Wave is)

    // Voice state indicator on line 10 (matching synth voice states)
    // Show (X) if voice is active OR if triggered within last 80ms (for snappy visual feedback)
    let recently_triggered = last_trigger
        .map(|t| t.elapsed().as_millis() < 80)
        .unwrap_or(false);
    let state_indicator = if voice_state.is_some() || recently_triggered { "(X)" } else { "---" };
    lines.push(format!("  {}", state_indicator));

    lines.push(String::new()); // Blank line (line 11)
    lines.push(String::new()); // Blank line (line 12)

    // Type and note on line 13 (compact format, lowercase)
    let drum_type_str = config.drum_type.name().to_lowercase();
    let note_num = config.parse_note().unwrap_or(0);
    let note_name = midi_note_to_name(note_num);
    lines.push(format!("  {}: {} ({})", drum_type_str, note_name, note_num));

    // Add blank lines to match synth height (16 lines total)
    while lines.len() < 16 {
        lines.push(String::new());
    }

    pad_lines(&mut lines);
    lines
}

/// Helper to add a drum parameter line with cursor indicator
fn add_drum_param_line(
    lines: &mut Vec<String>,
    is_selected: bool,
    selected_drum_param: DrumParameter,
    param: DrumParameter,
    name: &str,
    value: String,
) {
    let cursor = if is_selected && selected_drum_param == param {
        ">"
    } else {
        " "
    };
    lines.push(format!("{} {}: {}", cursor, name, value));
}

/// Build lines for a CV instance
fn build_cv_lines(
    config: &crate::config::CVInstanceConfig,
    voice_state: Option<u8>,
    is_selected: bool,
    selected_cv_param: CVParameter,
) -> Vec<String> {
    let mut lines = Vec::new();

    // MIDI channel string
    let midi_ch_str = if config.midi_channel_filter() == 255 {
        "omni".to_string()
    } else {
        format!("{}", config.midi_channel_filter() + 1)
    };

    // Title: m<midi>:a<pitch>+<gate>
    lines.push(format!(
        "  m{}:a{}+{}",
        midi_ch_str,
        config.audioch,
        config.audioch + 1
    ));
    lines.push(String::new());

    // Transpose parameter
    let cursor_transpose = if is_selected && selected_cv_param == CVParameter::Transpose {
        ">"
    } else {
        " "
    };
    lines.push(format!("{} Transpose: {:+}", cursor_transpose, config.transpose));

    // Glide parameter
    let cursor_glide = if is_selected && selected_cv_param == CVParameter::Glide {
        ">"
    } else {
        " "
    };
    lines.push(format!("{} Glide: {:.3}s", cursor_glide, config.glide));

    // Blank lines for spacing
    for _ in 0..5 {
        lines.push(String::new());
    }

    // Current note and voltage display
    if let Some(note) = voice_state {
        let note_name = midi_note_to_name(note);
        let voltage = (note as f32 - 60.0) / 12.0;
        lines.push(format!("  {} ({:+.3}V)", note_name, voltage));
    } else {
        lines.push(format!("  --- (0.000V)"));
    }

    // Pad to match synth height
    while lines.len() < 16 {
        lines.push(String::new());
    }

    pad_lines(&mut lines);
    lines
}

/// Pad all lines to same width for alignment
fn pad_lines(lines: &mut [String]) {
    let max_width = lines.iter().map(|s| s.len()).max().unwrap_or(0);
    lines.iter_mut().for_each(|line| {
        while line.len() < max_width {
            line.push(' ');
        }
    });
}

/// Render synthesizer help screen
fn render_synthesizer_help(frame: &mut Frame) {
    let help_text = vec![
        Line::from(""),
        Line::from("Controls"),
        Line::from(""),
        Line::from("  h/l, ←/→            Switch between synth instances"),
        Line::from("  j/k, ↑/↓            Move cursor between parameters"),
        Line::from("  H/L, tab/shift+tab  Adjust the selected parameter value"),
        Line::from("  0, $                Jump to first/last instance"),
        Line::from("  ?                   Toggle this help screen"),
        Line::from("  q, ctrl+c           Quit application"),
    ];

    let paragraph = Paragraph::new(help_text);
    frame.render_widget(paragraph, frame.size());
}
