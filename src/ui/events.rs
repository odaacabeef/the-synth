use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
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
        // Check for Ctrl+C
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            app.quit();
            return;
        }

        match key.code {
            KeyCode::Char('q') => {
                app.quit();
            }
            KeyCode::Up | KeyCode::Char('k') => {
                app.prev_device();
            }
            KeyCode::Down | KeyCode::Char('j') => {
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
