use ratatui::{
    text::Line,
    widgets::Paragraph,
    Frame,
};

use super::app::{App, CVParameter, DrumParameter, MultiInstance, Parameter, SamplerParameter};
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

    // Convert to ratatui Lines and render
    let lines: Vec<Line> = build_screen_lines(app, 20)
        .into_iter()
        .map(|s| Line::from(s))
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, frame.size());
}

/// Assemble the full multi-instance screen as plain text rows.
///
/// Each instance is rendered to a fixed-width, fixed-height column block; the
/// blocks are concatenated side by side with ":" dividers before the first
/// drum and first sampler groups.
fn build_screen_lines(app: &App, max_lines: usize) -> Vec<String> {
    // A ":" divider is drawn before the first drum and the first sampler,
    // separating those groups from the instruments to their left.
    let first_drum_idx = app
        .multi_instances
        .iter()
        .position(|inst| matches!(inst, MultiInstance::Drum { .. }));
    let first_sampler_idx = app
        .multi_instances
        .iter()
        .position(|inst| matches!(inst, MultiInstance::Sampler { .. }));

    // Build each instance's column block, plus whether a divider precedes it.
    let mut blocks: Vec<Vec<String>> = Vec::with_capacity(app.multi_instances.len());
    let mut divider_before: Vec<bool> = Vec::with_capacity(app.multi_instances.len());
    for (idx, instance) in app.multi_instances.iter().enumerate() {
        let is_selected = idx == app.current_instance;
        blocks.push(build_instance_lines(
            instance,
            is_selected,
            app.selected_param,
            app.selected_drum_param,
            app.selected_cv_param,
            app.selected_sampler_param,
        ));
        divider_before.push(Some(idx) == first_drum_idx || Some(idx) == first_sampler_idx);
    }

    combine_columns(&blocks, &divider_before, max_lines)
}

/// Merge per-instance column blocks into horizontal rows.
///
/// Each block is already a fixed-width column (every instance builder pads its
/// lines to a common width and the same number of rows), so concatenating row
/// by row yields a rectangular layout with vertically-aligned dividers. A "  :"
/// divider is inserted before any instance flagged in `divider_before` (except
/// the first, which never has a leading divider). Trailing rows that carry only
/// spacing and dividers are trimmed.
fn combine_columns(blocks: &[Vec<String>], divider_before: &[bool], max_lines: usize) -> Vec<String> {
    let mut combined: Vec<String> = vec![String::new(); max_lines];

    for (i, block) in blocks.iter().enumerate() {
        let spacing = if i == 0 {
            ""
        } else if divider_before.get(i).copied().unwrap_or(false) {
            "  :"
        } else {
            " "
        };
        for (line_idx, line) in block.iter().enumerate() {
            if line_idx < max_lines {
                combined[line_idx].push_str(spacing);
                combined[line_idx].push_str(line);
            }
        }
    }

    while combined
        .last()
        .map_or(false, |line| line.chars().all(|c| c == ' ' || c == ':'))
    {
        combined.pop();
    }

    combined
}

/// Build lines for a single instrument instance (synth, drum, or CV)
fn build_instance_lines(
    instance: &MultiInstance,
    is_selected: bool,
    selected_param: Parameter,
    selected_drum_param: DrumParameter,
    selected_cv_param: CVParameter,
    selected_sampler_param: SamplerParameter,
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
            voice_states,
            last_trigger,
            ..
        } => build_cv_lines(config, voice_states, *last_trigger, is_selected, selected_cv_param),
        MultiInstance::ES5 {
            config,
            voice_states,
            last_trigger,
            ..
        } => build_es5_lines(config, voice_states, *last_trigger),
        MultiInstance::Sampler {
            config,
            voice_states,
            last_trigger,
            ..
        } => build_sampler_lines(config, voice_states, *last_trigger, is_selected, selected_sampler_param),
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

    // Pad to match other instruments (16 lines total) so dividers align
    while lines.len() < 16 {
        lines.push(String::new());
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
            // Longest param name: "PitchStart" or "PitchDecay" = 10 chars, +1 for colon = 11
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::KickPitchStart, "PitchStart", format!("{}Hz", config.kick_pitch_start), 11);
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::KickPitchEnd, "PitchEnd", format!("{}Hz", config.kick_pitch_end), 11);
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::KickPitchDecay, "PitchDecay", format!("{:.3}s", config.kick_pitch_decay), 11);
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::KickDecay, "Decay", format!("{:.3}s", config.kick_decay), 11);
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::KickClick, "Click", format!("{:.2}", config.kick_click), 11);
        }
        DrumType::Snare => {
            // Longest param name: "ToneFreq" = 8 chars, +1 for colon = 9
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::SnareToneFreq, "ToneFreq", format!("{}Hz", config.snare_tone_freq), 9);
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::SnareToneMix, "ToneMix", format!("{:.2}", config.snare_tone_mix), 9);
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::SnareDecay, "Decay", format!("{:.3}s", config.snare_decay), 9);
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::SnareSnap, "Snap", format!("{:.2}", config.snare_snap), 9);
            lines.push(String::new()); // Blank line (snare has 4 params, need 5 lines)
        }
        DrumType::Hat => {
            // Longest param name: "Brightness" = 10 chars, +1 for colon = 11
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::HatBrightness, "Brightness", format!("{}Hz", config.hat_brightness), 11);
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::HatDecay, "Decay", format!("{:.3}s", config.hat_decay), 11);
            add_drum_param_line(&mut lines, is_selected, selected_drum_param, DrumParameter::HatMetallic, "Metallic", format!("{:.2}", config.hat_metallic), 11);
            lines.push(String::new()); // Blank line (hat has 3 params, need 5 lines)
            lines.push(String::new()); // Another blank
        }
    }

    lines.push(String::new()); // Blank line (line 8)
    lines.push(String::new()); // Blank line (line 9, where synth Wave is)

    // Voice state indicator on line 10 (matching synth voice states)
    // Show +++ if voice is active OR if triggered within last 80ms (for snappy visual feedback)
    let recently_triggered = last_trigger
        .map(|t| t.elapsed().as_millis() < 80)
        .unwrap_or(false);
    let state_indicator = if voice_state.is_some() || recently_triggered { "+++" } else { "---" };
    lines.push(format!("  {}", state_indicator));

    lines.push(String::new()); // Blank line (line 11)
    lines.push(String::new()); // Blank line (line 12)

    // Type and note on line 13 (compact format, lowercase)
    let drum_type_str = config.drum_type.name().to_lowercase();
    let note_num = config.parse_note().unwrap_or(0);
    lines.push(format!("  {}: {} ({})", drum_type_str, config.note, note_num));

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
    width: usize,
) {
    let cursor = if is_selected && selected_drum_param == param {
        ">"
    } else {
        " "
    };
    // Left-align parameter name with colon, then pad to specified width
    // Width should be longest_name_length + 1 (for colon) to give 1 space before values
    let label = format!("{}:", name);
    lines.push(format!("{} {:<width$} {}", cursor, label, value, width = width));
}

/// Build lines for a CV instance
fn build_cv_lines(
    config: &crate::config::CVInstanceConfig,
    voice_states: &[Option<u8>; 16],
    last_trigger: Option<std::time::Instant>,
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

    let title = match config.voices {
        0 => format!("  m{}:a{}", midi_ch_str, config.audioch),
        n => format!("  m{}:a{}-{}", midi_ch_str, config.audioch, config.audioch + n),
    };
    lines.push(title);

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

    // Note filter
    let note_str = config.note.as_deref().unwrap_or("-");
    lines.push(format!("  Note: {}", note_str));

    lines.push(String::new());

    // Voice display
    if config.voices == 0 {
        let recently_triggered = last_trigger
            .map(|t| t.elapsed().as_millis() < 80)
            .unwrap_or(false);
        let indicator = if voice_states[0].is_some() || recently_triggered { "+++" } else { "---" };
        lines.push(format!("  {}", indicator));
    } else if config.voices == 1 {
        let note = voice_states[0];
        push_cv_voice_line(&mut lines, note, None);
    } else {
        for v in 0..config.voices {
            let note = voice_states.get(v).copied().flatten();
            push_cv_voice_line(&mut lines, note, Some(v + 1));
        }
    }

    // Pad to consistent height
    while lines.len() < 16 {
        lines.push(String::new());
    }

    pad_lines(&mut lines);

    // Enforce minimum width: widest possible voice line is "  1: C#5  +1.667V"
    let min_width = "  1: C#5  +1.667V".len();
    let current_width = lines.first().map(|l| l.len()).unwrap_or(0);
    if current_width < min_width {
        let extra = min_width - current_width;
        for line in lines.iter_mut() {
            line.extend(std::iter::repeat(' ').take(extra));
        }
    }

    lines
}

fn push_cv_voice_line(lines: &mut Vec<String>, note: Option<u8>, index: Option<usize>) {
    let prefix = match index {
        Some(i) => format!("  {}:", i),
        None => String::from(" "),
    };
    match note {
        Some(n) => {
            let note_name = midi_note_to_name(n);
            let voltage = (n as f32 - 60.0) / 12.0;
            lines.push(format!("{} {:<3}  {:+.3}V", prefix, note_name, voltage));
        }
        None => {
            lines.push(format!("{} ---", prefix));
        }
    }
}

/// Build lines for an ES-5 gate encoder instance
fn build_es5_lines(
    config: &crate::config::ES5InstanceConfig,
    voice_states: &[Option<u8>; 16],
    last_trigger: Option<std::time::Instant>,
) -> Vec<String> {
    let mut lines = Vec::new();

    // MIDI channel string
    let midi_ch_str = if config.midi_channel_filter() == 255 {
        "omni".to_string()
    } else {
        format!("{}", config.midi_channel_filter() + 1)
    };

    // Title: m<midi>:a<audio>-<audio+1>
    lines.push(format!("  m{}:a{}-{}", midi_ch_str, config.audioch, config.audioch + 1));
    lines.push(String::new());

    // Each output: index, note, gate state
    for (i, output) in config.outputs.iter().enumerate() {
        let recently_triggered = last_trigger
            .map(|t| t.elapsed().as_millis() < 80)
            .unwrap_or(false);
        let active = voice_states[i].is_some() || (recently_triggered && voice_states[i].is_some());
        let indicator = if active { "+++" } else { "---" };
        lines.push(format!("  {}: {:>3} {}", i + 1, output.note, indicator));
    }

    // Pad remaining param lines to match other instruments
    while lines.len() < 8 {
        lines.push(String::new());
    }

    // Type label
    lines.push(String::new());
    lines.push("  es5".to_string());

    while lines.len() < 16 {
        lines.push(String::new());
    }

    pad_lines(&mut lines);
    lines
}

/// Build lines for a sampler instance
fn build_sampler_lines(
    config: &crate::config::SamplerInstanceConfig,
    voice_states: &[Option<u8>; 16],
    last_trigger: Option<std::time::Instant>,
    is_selected: bool,
    selected_sampler_param: SamplerParameter,
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

    // Parameters (label width aligns to longest, "Release:" = 8)
    add_sampler_param_line(&mut lines, is_selected, selected_sampler_param, SamplerParameter::Gain, "Gain", format!("{:.1}dB", config.gain), 8);
    add_sampler_param_line(&mut lines, is_selected, selected_sampler_param, SamplerParameter::Pitch, "Pitch", format!("{:+}", config.pitch), 8);
    add_sampler_param_line(&mut lines, is_selected, selected_sampler_param, SamplerParameter::Start, "Start", format!("{:.2}", config.start), 8);
    add_sampler_param_line(&mut lines, is_selected, selected_sampler_param, SamplerParameter::Attack, "Attack", format!("{:.3}s", config.attack), 8);
    add_sampler_param_line(&mut lines, is_selected, selected_sampler_param, SamplerParameter::Release, "Release", format!("{:.3}s", config.release), 8);

    lines.push(String::new()); // Blank line
    lines.push(String::new()); // Blank line

    // Voice / trigger indicator (line 10, matching other instruments)
    let recently_triggered = last_trigger
        .map(|t| t.elapsed().as_millis() < 80)
        .unwrap_or(false);
    let active_voices = voice_states.iter().filter(|v| v.is_some()).count();
    let indicator = if active_voices > 0 || recently_triggered { "+++" } else { "---" };
    if config.voices > 1 {
        lines.push(format!("  {} {}/{}", indicator, active_voices, config.voices));
    } else {
        lines.push(format!("  {}", indicator));
    }

    lines.push(String::new()); // Blank line

    // Sample name (no prefix): a single line showing what fits, truncated with
    // a trailing "..." so long names can't widen the instance.
    const NAME_WIDTH: usize = 16; // characters shown for the name / mapping
    let stem = std::path::Path::new(&config.file)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(config.file.as_str());
    lines.push(format!("  {}", truncate_ellipsis(stem, NAME_WIDTH)));

    // Note mapping: root with its MIDI number (matching the drums display),
    // plus the range if melodic.
    let root_num = config.parse_root().unwrap_or(0);
    let mapping = match &config.range {
        Some(range) if range.len() == 2 => {
            format!("{} ({}) {}-{}", config.root, root_num, range[0], range[1])
        }
        _ => format!("{} ({})", config.root, root_num),
    };
    lines.push(format!("  {}", truncate_ellipsis(&mapping, NAME_WIDTH)));

    // Pad to match other instruments (16 lines total)
    while lines.len() < 16 {
        lines.push(String::new());
    }

    pad_lines(&mut lines);

    // Fixed panel width so long sample names don't widen the instance.
    const FIXED_WIDTH: usize = 2 + NAME_WIDTH;
    for line in lines.iter_mut() {
        if line.len() < FIXED_WIDTH {
            line.push_str(&" ".repeat(FIXED_WIDTH - line.len()));
        }
    }

    lines
}

/// Helper to add a sampler parameter line with cursor indicator
fn add_sampler_param_line(
    lines: &mut Vec<String>,
    is_selected: bool,
    selected: SamplerParameter,
    param: SamplerParameter,
    name: &str,
    value: String,
    width: usize,
) {
    let cursor = if is_selected && selected == param {
        ">"
    } else {
        " "
    };
    let label = format!("{}:", name);
    lines.push(format!("{} {:<width$} {}", cursor, label, value, width = width));
}

/// Truncate `s` to at most `width` characters, appending "..." when it is cut.
fn truncate_ellipsis(s: &str, width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= width {
        return s.to_string();
    }
    let keep = width.saturating_sub(3);
    let mut out: Vec<char> = chars[..keep].to_vec();
    out.extend(['.', '.', '.']);
    out.iter().collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sampler_config(file: &str, range: Option<Vec<String>>) -> crate::config::SamplerInstanceConfig {
        crate::config::SamplerInstanceConfig {
            midich: crate::config::MidiChannelSpec::Channel(1),
            audioch: 5,
            file: file.to_string(),
            root: "c2".to_string(),
            range,
            voices: 1,
            gain: 0.0,
            pitch: 0,
            start: 0.0,
            attack: 0.0,
            release: 0.05,
        }
    }

    #[test]
    fn test_sampler_lines_fixed_width_and_no_prefix() {
        let config = sampler_config("samples/short.wav", None);
        let lines = build_sampler_lines(&config, &[None; 16], None, false, SamplerParameter::Gain);

        // Every line shares the same fixed width.
        assert!(lines.iter().all(|l| l.len() == 18), "all lines must be fixed width 18");
        // The old "smp:" prefix is gone.
        assert!(lines.iter().all(|l| !l.contains("smp:")));
        // Short name on its single line, mapping (with root MIDI number) below.
        assert_eq!(lines[11].trim(), "short");
        assert_eq!(lines[12].trim(), "c2 (36)");
    }

    #[test]
    fn test_sampler_lines_long_name_truncated() {
        // 40-char stem exceeds the single 16-char line.
        let long = "a".repeat(40);
        let config = sampler_config(&format!("samples/{}.wav", long), None);
        let lines = build_sampler_lines(&config, &[None; 16], None, false, SamplerParameter::Gain);

        let name = lines[11].trim();
        assert_eq!(name.chars().count(), 16); // 13 name chars + "..."
        assert!(name.ends_with("..."));
    }

    #[test]
    fn test_sampler_lines_range_mapping() {
        let config = sampler_config(
            "samples/piano.wav",
            Some(vec!["c2".to_string(), "c5".to_string()]),
        );
        let lines = build_sampler_lines(&config, &[None; 16], None, false, SamplerParameter::Gain);
        assert_eq!(lines[12].trim(), "c2 (36) c2-c5");
    }

    #[test]
    fn test_combine_columns_rectangular_no_detached_divider() {
        // Three columns of equal height but differing widths, with dividers
        // before the 2nd and 3rd (mimicking poly | drums | sampler). The last
        // row is blank in every column so it should be trimmed away.
        let blocks = vec![
            vec!["aa".into(), "aa".into(), "  ".into()],
            vec!["bbb".into(), "   ".into(), "   ".into()],
            vec!["c".into(), "c".into(), " ".into()],
        ];
        let divider_before = vec![false, true, true];

        let lines = combine_columns(&blocks, &divider_before, 20);

        // Trailing all-blank/divider row trimmed (3 rows -> 2).
        assert_eq!(lines.len(), 2);
        // Rectangular: every row is the same width.
        let w = lines[0].len();
        assert!(lines.iter().all(|l| l.len() == w), "rows must be equal width");
        // No row starts with a detached leading divider.
        assert!(lines.iter().all(|l| !l.starts_with("  :")), "no detached divider");
    }

    #[test]
    fn test_combine_columns_trims_multi_divider_blank_rows() {
        // A trailing row carrying only spacing and *two* dividers must trim.
        let blocks = vec![
            vec!["x".into(), " ".into()],
            vec!["y".into(), " ".into()],
            vec!["z".into(), " ".into()],
        ];
        let divider_before = vec![false, true, true];

        let lines = combine_columns(&blocks, &divider_before, 20);
        assert_eq!(lines.len(), 1, "row of only spaces and ':' dividers must be trimmed");
    }

    fn synth_config(midich: u8, audioch: usize) -> crate::config::SynthInstanceConfig {
        crate::config::SynthInstanceConfig {
            name: "x".to_string(),
            midich: crate::config::MidiChannelSpec::Channel(midich),
            audioch,
            attack: 0.01,
            decay: 0.1,
            sustain: 0.5,
            release: 0.1,
            wave: crate::config::WaveformSpec::Sine,
        }
    }

    #[test]
    fn test_screen_lines_synths_then_samplers_aligned() {
        use crate::instruments::poly16::SynthParameters;
        use crate::instruments::sampler::SamplerParameters;
        use std::sync::Arc;

        // Two synths (shorter panels) to the left of two samplers - the layout
        // that exposed detached dividers before synth panels were height-padded.
        let app = App::new_multi_instance(
            vec![Arc::new(SynthParameters::default()), Arc::new(SynthParameters::default())],
            vec![synth_config(2, 2), synth_config(3, 3)],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![],
            vec![Arc::new(SamplerParameters::new()), Arc::new(SamplerParameters::new())],
            vec![
                sampler_config("glitch/20260614-122419-dense.wav", None),
                sampler_config(
                    &format!("glitch/{}.wav", "a".repeat(40)),
                    Some(vec!["c2".to_string(), "c5".to_string()]),
                ),
            ],
        );

        let lines = build_screen_lines(&app, 20);

        // Rectangular block: every row is the same width.
        let w = lines[0].len();
        assert!(lines.iter().all(|l| l.len() == w), "rows must be equal width");
        // No row begins with a detached divider.
        assert!(lines.iter().all(|l| !l.starts_with("  :")), "no detached leading divider");
        // Trailing divider-only rows trimmed; the last row carries real content.
        let last = lines.last().unwrap();
        assert!(last.chars().any(|c| c != ' ' && c != ':'), "last row should carry content");
    }
}
