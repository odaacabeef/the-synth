use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use super::app::{App, MultiInstance};

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

        // Navigate parameters (vim-style: j=down, k=up)
        KeyCode::Char('j') | KeyCode::Down => {
            // Check if current instance is synth or drum
            if let Some(instance) = app.multi_instances.get(app.current_instance) {
                match instance {
                    MultiInstance::Synth { .. } => {
                        app.next_parameter();
                    }
                    MultiInstance::Drum { config, .. } => {
                        app.next_drum_parameter(config.drum_type);
                    }
                }
            }
        }
        KeyCode::Char('k') | KeyCode::Up => {
            // Check if current instance is synth or drum
            if let Some(instance) = app.multi_instances.get(app.current_instance) {
                match instance {
                    MultiInstance::Synth { .. } => {
                        app.prev_parameter();
                    }
                    MultiInstance::Drum { config, .. } => {
                        app.prev_drum_parameter(config.drum_type);
                    }
                }
            }
        }

        // Adjust values (vim-style: l=right, h=left)
        KeyCode::Char('l') | KeyCode::Right => {
            app.increase_value();
        }
        KeyCode::Char('h') | KeyCode::Left => {
            app.decrease_value();
        }

        // Switch instances in multi mode (Tab/Shift+Tab or H/L)
        KeyCode::Tab | KeyCode::Char('L') => {
            app.next_instance();
        }
        KeyCode::BackTab | KeyCode::Char('H') => {
            app.prev_instance();
        }

        // Jump to first/last instance (vim-style: 0/$)
        KeyCode::Char('0') => {
            app.jump_to_first();
        }
        KeyCode::Char('$') => {
            app.jump_to_last();
        }

        _ => {}
    }
}
