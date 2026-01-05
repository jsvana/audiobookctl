//! Event handling for the TUI

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};

use super::state::{App, Focus, Mode};

/// Handle a key event, returning true if the app should continue
pub fn handle_key(app: &mut App, key: KeyEvent) -> bool {
    // Handle Ctrl+C globally
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return true;
    }

    match app.mode {
        Mode::Normal => handle_normal_mode(app, key),
        Mode::Searching => handle_search_mode(app, key),
        Mode::Help => handle_help_mode(app, key),
    }

    true
}

fn handle_normal_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('?') => app.mode = Mode::Help,
        KeyCode::Char('/') => app.mode = Mode::Searching,
        KeyCode::Char('f') => app.cycle_filter(),
        KeyCode::Tab => {
            app.focus = match app.focus {
                Focus::List => Focus::Details,
                Focus::Details => Focus::List,
            };
        }
        KeyCode::Esc => {
            if !app.search_query.is_empty() {
                app.search_clear();
            }
        }
        // Navigation
        KeyCode::Up | KeyCode::Char('k') => {
            if app.focus == Focus::List {
                app.select_previous();
            } else {
                app.scroll_details_up();
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.focus == Focus::List {
                app.select_next();
            } else {
                app.scroll_details_down();
            }
        }
        KeyCode::PageUp => {
            if app.focus == Focus::List {
                app.select_previous_page(10);
            } else {
                for _ in 0..10 {
                    app.scroll_details_up();
                }
            }
        }
        KeyCode::PageDown => {
            if app.focus == Focus::List {
                app.select_next_page(10);
            } else {
                for _ in 0..10 {
                    app.scroll_details_down();
                }
            }
        }
        KeyCode::Home => {
            if app.focus == Focus::List {
                app.select_first();
            } else {
                app.details_scroll = 0;
            }
        }
        KeyCode::End => {
            if app.focus == Focus::List {
                app.select_last();
            }
        }
        _ => {}
    }
}

fn handle_search_mode(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
        }
        KeyCode::Backspace => {
            app.search_pop();
        }
        KeyCode::Char(c) => {
            app.search_push(c);
        }
        // Allow navigation while searching
        KeyCode::Up => app.select_previous(),
        KeyCode::Down => app.select_next(),
        _ => {}
    }
}

fn handle_help_mode(app: &mut App, _key: KeyEvent) {
    // Any key closes help
    app.mode = Mode::Normal;
}

/// Poll for events with a timeout
pub fn poll_event(timeout: std::time::Duration) -> std::io::Result<Option<Event>> {
    if event::poll(timeout)? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
