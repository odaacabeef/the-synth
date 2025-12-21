use crossterm::event::{self, Event, KeyCode, KeyEvent};
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
    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => {
            app.quit();
        }

        // Navigate parameters
        KeyCode::Tab | KeyCode::Right => {
            app.next_parameter();
        }
        KeyCode::Left => {
            app.prev_parameter();
        }

        // Adjust values
        KeyCode::Up | KeyCode::Char('+') | KeyCode::Char('=') => {
            app.increase_value();
        }
        KeyCode::Down | KeyCode::Char('-') | KeyCode::Char('_') => {
            app.decrease_value();
        }

        // Quick waveform selection
        KeyCode::Char('1') => {
            app.waveform = crate::types::waveform::Waveform::Sine;
        }
        KeyCode::Char('2') => {
            app.waveform = crate::types::waveform::Waveform::Triangle;
        }
        KeyCode::Char('3') => {
            app.waveform = crate::types::waveform::Waveform::Sawtooth;
        }
        KeyCode::Char('4') => {
            app.waveform = crate::types::waveform::Waveform::Square;
        }

        _ => {}
    }
}
