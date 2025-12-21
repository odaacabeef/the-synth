use crossterm::event::{self, Event, KeyCode, KeyEvent};
use std::time::Duration;

use super::app::{App, AppMode};

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
    // Handle device selection mode
    if app.mode == AppMode::DeviceSelection {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                app.quit();
            }
            KeyCode::Up => {
                app.prev_device();
            }
            KeyCode::Down => {
                app.next_device();
            }
            KeyCode::Enter => {
                if !app.midi_devices.is_empty() {
                    app.confirm_device();
                }
            }
            _ => {}
        }
        return;
    }

    // Handle synthesizer mode
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
