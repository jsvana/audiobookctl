//! Browse command - interactive TUI for audiobook library

use anyhow::{Context, Result};
use crossterm::{
    event::Event,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::{io::stdout, path::Path, time::Duration};

use crate::config::Config;
use crate::database::LibraryDb;
use crate::tui::{handle_key, poll_event, render, App};

/// Run the browse TUI
pub fn run(db_path: Option<&Path>) -> Result<()> {
    // Open database
    let db = if let Some(path) = db_path {
        LibraryDb::open(path)?
    } else {
        // Try config destination first, then fall back to cwd
        let config = Config::load().context("Failed to load config")?;
        if let Some(dest) = config.dest(None) {
            LibraryDb::find_from(&dest)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "No database found in configured destination '{}'. Run 'audiobookctl index <dir>' first, or specify --db",
                    dest.display()
                )
            })?
        } else {
            let cwd = std::env::current_dir()?;
            LibraryDb::find_from(&cwd)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "No database found and no destination configured. Set [organize] dest in config, run 'audiobookctl index <dir>', or specify --db"
                )
            })?
        }
    };

    // Load all audiobooks
    let audiobooks = db.list_all()?;

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    // Set up panic hook to restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut app = App::new(audiobooks);

    // Main loop
    loop {
        // Render
        terminal.draw(|frame| render(frame, &app))?;

        // Handle events
        if let Some(Event::Key(key)) = poll_event(Duration::from_millis(100))? {
            handle_key(&mut app, key);
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

    Ok(())
}
