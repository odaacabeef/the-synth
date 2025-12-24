use ratatui::{
    text::Line,
    widgets::Paragraph,
    Frame,
};

use super::app::{App, Parameter};

/// Render the TUI
pub fn render(frame: &mut Frame, app: &App) {
    // Show help screen if toggled
    if app.show_help {
        render_synthesizer_help(frame);
    } else {
        render_synthesizer(frame, app);
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
    lines.push(Line::from(format!("{} Waveform: {:?}", cursor_waveform, app.waveform)));

    // Blank line
    lines.push(Line::from(""));

    // Voice states (first 8 voices)
    let mut voice_line1 = String::from("  ");
    for i in 0..8 {
        if i > 0 {
            voice_line1.push(' ');
        }
        match app.voice_states[i] {
            Some(note) => voice_line1.push_str(&midi_note_to_name(note)),
            None => voice_line1.push_str("--"),
        }
    }
    lines.push(Line::from(voice_line1));

    // Voice states (last 8 voices)
    let mut voice_line2 = String::from("  ");
    for i in 8..16 {
        if i > 8 {
            voice_line2.push(' ');
        }
        match app.voice_states[i] {
            Some(note) => voice_line2.push_str(&midi_note_to_name(note)),
            None => voice_line2.push_str("--"),
        }
    }
    lines.push(Line::from(voice_line2));

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

/// Render synthesizer help screen
fn render_synthesizer_help(frame: &mut Frame) {
    let help_text = vec![
        Line::from(""),
        Line::from("Controls"),
        Line::from(""),
        Line::from("  h, l, ←, →     Adjust the selected parameter value"),
        Line::from("  j, k, ↑, ↓     Move cursor between parameters"),
        Line::from("  1              Set waveform to Sine"),
        Line::from("  2              Set waveform to Triangle"),
        Line::from("  3              Set waveform to Sawtooth"),
        Line::from("  4              Set waveform to Square"),
        Line::from("  ?              Toggle this help screen"),
        Line::from("  q, Ctrl+C      Quit application"),
        Line::from(""),
        Line::from("Press ? to close this help screen"),
    ];

    let paragraph = Paragraph::new(help_text);
    frame.render_widget(paragraph, frame.size());
}
