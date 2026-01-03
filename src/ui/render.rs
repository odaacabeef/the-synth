use ratatui::{
    text::Line,
    widgets::Paragraph,
    Frame,
};

use super::app::{App, AppMode, Parameter};

/// Render the TUI
pub fn render(frame: &mut Frame, app: &App) {
    // Show help screen if toggled
    if app.show_help {
        render_synthesizer_help(frame, app.mode);
    } else {
        match app.mode {
            AppMode::Single => render_synthesizer(frame, app),
            AppMode::Multi => render_multi_instance(frame, app),
        }
    }
}

/// Render synthesizer screen
fn render_synthesizer(frame: &mut Frame, app: &App) {
    // Build the UI as simple text lines
    let mut lines = Vec::new();

    // ADSR parameters
    let cursor_attack = if app.selected_param == Parameter::Attack { ">" } else { " " };
    lines.push(Line::from(format!("{} Attack:  {:.3}s", cursor_attack, app.attack)));

    let cursor_decay = if app.selected_param == Parameter::Decay { ">" } else { " " };
    lines.push(Line::from(format!("{} Decay:   {:.3}s", cursor_decay, app.decay)));

    let cursor_sustain = if app.selected_param == Parameter::Sustain { ">" } else { " " };
    lines.push(Line::from(format!("{} Sustain: {:.2}", cursor_sustain, app.sustain)));

    let cursor_release = if app.selected_param == Parameter::Release { ">" } else { " " };
    lines.push(Line::from(format!("{} Release: {:.3}s", cursor_release, app.release)));

    // Blank line
    lines.push(Line::from(""));

    // Waveform
    let cursor_waveform = if app.selected_param == Parameter::Waveform { ">" } else { " " };
    lines.push(Line::from(format!("{} Wave: {:?}", cursor_waveform, app.waveform)));

    // Blank line
    lines.push(Line::from(""));

    // Voice states (4 rows of 4 voices each)
    for row in 0..4 {
        let mut voice_line = String::from("  ");
        for col in 0..4 {
            if col > 0 {
                voice_line.push(' ');
            }
            let voice_idx = row * 4 + col;
            match app.voice_states[voice_idx] {
                Some(note) => {
                    let note_name = midi_note_to_name(note);
                    // Pad to 3 characters
                    voice_line.push_str(&format!("{:3}", note_name));
                }
                None => voice_line.push_str("---"),
            }
        }
        lines.push(Line::from(voice_line));
    }

    // Render as a simple paragraph
    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, frame.size());
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

    for (idx, instance) in app.multi_instances.iter().enumerate() {
        let is_selected = idx == app.current_instance;
        let instance_lines = build_instance_lines(instance, is_selected, app.selected_param);

        // Add spacing between instances (1 space)
        let spacing = if idx > 0 { " " } else { "" };

        // Merge instance lines horizontally
        for (line_idx, line) in instance_lines.iter().enumerate() {
            if line_idx < max_lines {
                combined_lines[line_idx].push_str(spacing);
                combined_lines[line_idx].push_str(line);
            }
        }
    }

    // Convert to ratatui Lines and render
    let lines: Vec<Line> = combined_lines
        .into_iter()
        .map(|s| Line::from(s))
        .collect();

    let paragraph = Paragraph::new(lines);
    frame.render_widget(paragraph, frame.size());
}

/// Build lines for a single synth instance
fn build_instance_lines(
    instance: &super::app::MultiInstance,
    is_selected: bool,
    selected_param: Parameter,
) -> Vec<String> {
    let mut lines = Vec::new();

    // MIDI channel string (1-indexed for display)
    let midi_ch_str = if instance.config.midi_channel_filter() == 255 {
        "omni".to_string()
    } else {
        format!("{}", instance.config.midi_channel_filter() + 1)
    };

    // Audio channel (already 1-indexed in config)
    let audio_ch = instance.config.audioch;

    // Title line: m<midi>:a<audio>
    lines.push(format!("  m{}:a{}", midi_ch_str, audio_ch));

    lines.push(String::new()); // Blank line

    // ADSR parameters (only show cursor if this instance is selected)
    let cursor_attack = if is_selected && selected_param == Parameter::Attack { ">" } else { " " };
    lines.push(format!("{} Attack:  {:.3}s", cursor_attack, instance.config.attack));

    let cursor_decay = if is_selected && selected_param == Parameter::Decay { ">" } else { " " };
    lines.push(format!("{} Decay:   {:.3}s", cursor_decay, instance.config.decay));

    let cursor_sustain = if is_selected && selected_param == Parameter::Sustain { ">" } else { " " };
    lines.push(format!("{} Sustain: {:.2}", cursor_sustain, instance.config.sustain));

    let cursor_release = if is_selected && selected_param == Parameter::Release { ">" } else { " " };
    lines.push(format!("{} Release: {:.3}s", cursor_release, instance.config.release));

    lines.push(String::new()); // Blank line

    // Waveform
    let cursor_waveform = if is_selected && selected_param == Parameter::Waveform { ">" } else { " " };
    let waveform = instance.config.waveform();
    let waveform_str = match waveform {
        crate::types::waveform::Waveform::Sine => "Sine",
        crate::types::waveform::Waveform::Triangle => "Triangle",
        crate::types::waveform::Waveform::Sawtooth => "Sawtooth",
        crate::types::waveform::Waveform::Square => "Square",
    };
    lines.push(format!("{} Wave: {}", cursor_waveform, waveform_str));

    lines.push(String::new()); // Blank line

    // Voice states (4 rows of 4 voices each) - add leading spaces for alignment
    for row in 0..4 {
        let mut voice_line = String::from("  "); // Leading spaces for alignment
        for col in 0..4 {
            if col > 0 {
                voice_line.push(' ');
            }
            let voice_idx = row * 4 + col;
            match instance.voice_states[voice_idx] {
                Some(note) => {
                    let note_name = midi_note_to_name(note);
                    voice_line.push_str(&format!("{:3}", note_name));
                }
                None => voice_line.push_str("---"),
            }
        }
        lines.push(voice_line);
    }

    // Pad all lines to same width for alignment
    let max_width = lines.iter().map(|s| s.len()).max().unwrap_or(0);
    lines.iter_mut().for_each(|line| {
        while line.len() < max_width {
            line.push(' ');
        }
    });

    lines
}

/// Render synthesizer help screen
fn render_synthesizer_help(frame: &mut Frame, mode: AppMode) {
    let mut help_text = vec![
        Line::from(""),
        Line::from("Controls"),
        Line::from(""),
        Line::from("  h, l, ←, →     Adjust the selected parameter value"),
        Line::from("  j, k, ↑, ↓     Move cursor between parameters"),
    ];

    if mode == AppMode::Multi {
        help_text.push(Line::from("  Tab/Shift+Tab  Switch between synth instances"));
    }

    help_text.extend(vec![
        Line::from("  ?              Toggle this help screen"),
        Line::from("  q, Ctrl+C      Quit application"),
        Line::from(""),
        Line::from("Press ? to close this help screen"),
    ]);

    let paragraph = Paragraph::new(help_text);
    frame.render_widget(paragraph, frame.size());
}
