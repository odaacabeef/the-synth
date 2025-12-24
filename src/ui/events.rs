use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use super::app::App;

/// Handle keyboard events and update app state
pub fn handle_events(app: &mut App) -> anyhow::Result<()> {
    // Poll for events with timeout
    if event::poll(Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            handle_key_event(app, key);
        }
    }
    Ok(())
}

/// Process individual key press
fn handle_key_event(app: &mut App, key: KeyEvent) {
    // Check for Ctrl+C
    if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
        app.quit();
        return;
    }

    match key.code {
        // Quit
        KeyCode::Char('q') => {
            app.quit();
        }

        // Toggle help
        KeyCode::Char('?') => {
            app.toggle_help();
        }

        // Navigate parameters (vim-style: h=left, l=right)
        KeyCode::Char('l') | KeyCode::Right => {
            app.next_parameter();
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.prev_parameter();
        }

        // Adjust values (vim-style: k=up, j=down)
        KeyCode::Char('k') | KeyCode::Up => {
            app.increase_value();
        }
        KeyCode::Char('j') | KeyCode::Down => {
            app.decrease_value();
        }

        // Quick waveform selection
        KeyCode::Char('1') => {
            app.waveform = crate::types::waveform::Waveform::Sine;
            app.parameters.waveform.store(0, std::sync::atomic::Ordering::Relaxed);
        }
        KeyCode::Char('2') => {
            app.waveform = crate::types::waveform::Waveform::Triangle;
            app.parameters.waveform.store(1, std::sync::atomic::Ordering::Relaxed);
        }
        KeyCode::Char('3') => {
            app.waveform = crate::types::waveform::Waveform::Sawtooth;
            app.parameters.waveform.store(2, std::sync::atomic::Ordering::Relaxed);
        }
        KeyCode::Char('4') => {
            app.waveform = crate::types::waveform::Waveform::Square;
            app.parameters.waveform.store(3, std::sync::atomic::Ordering::Relaxed);
        }

        _ => {}
    }
}
