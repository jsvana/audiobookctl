# TUI Browse Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a terminal UI for browsing and filtering audiobooks from the SQLite database.

**Architecture:** Ratatui-based TUI with two-pane layout (list + details). Loads all audiobooks on startup, filters in-memory for responsive real-time search. State machine pattern for input modes.

**Tech Stack:** ratatui, crossterm, existing LibraryDb

---

## Task 1: Add Dependencies

**Files:**
- Modify: `Cargo.toml`

**Step 1: Add ratatui and crossterm**

Add to `[dependencies]` section:

```toml
ratatui = "0.29"
crossterm = "0.28"
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add ratatui and crossterm for TUI"
```

---

## Task 2: Expose list_all in Database

**Files:**
- Modify: `src/database/mod.rs`

**Step 1: Make list_all public**

Change line 327 from:
```rust
    fn list_all(&self) -> Result<Vec<AudiobookRecord>> {
```

To:
```rust
    pub fn list_all(&self) -> Result<Vec<AudiobookRecord>> {
```

**Step 2: Verify it compiles**

Run: `cargo check`
Expected: Compiles with no errors

**Step 3: Commit**

```bash
git add src/database/mod.rs
git commit -m "feat(database): expose list_all method for TUI"
```

---

## Task 3: Create TUI State Module

**Files:**
- Create: `src/tui/mod.rs`
- Create: `src/tui/state.rs`

**Step 1: Create tui module root**

Create `src/tui/mod.rs`:

```rust
//! Terminal UI for browsing audiobooks

mod state;

pub use state::{App, FilterField, Focus, Mode};
```

**Step 2: Create state module**

Create `src/tui/state.rs`:

```rust
//! Application state for the TUI

use crate::database::AudiobookRecord;

/// Which field to filter by
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FilterField {
    #[default]
    All,
    Author,
    Series,
    Genre,
    Narrator,
}

impl FilterField {
    pub fn next(self) -> Self {
        match self {
            Self::All => Self::Author,
            Self::Author => Self::Series,
            Self::Series => Self::Genre,
            Self::Genre => Self::Narrator,
            Self::Narrator => Self::All,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Author => "Author",
            Self::Series => "Series",
            Self::Genre => "Genre",
            Self::Narrator => "Narrator",
        }
    }
}

/// Which pane has focus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    List,
    Details,
}

/// Input mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    #[default]
    Normal,
    Searching,
    Help,
}

/// Main application state
pub struct App {
    /// All audiobooks from database
    pub audiobooks: Vec<AudiobookRecord>,
    /// Indices into audiobooks matching current filter
    pub filtered: Vec<usize>,
    /// Current selection index (into filtered)
    pub selected: usize,
    /// Current search query
    pub search_query: String,
    /// Which field to filter by
    pub filter_field: FilterField,
    /// Which pane has focus
    pub focus: Focus,
    /// Current input mode
    pub mode: Mode,
    /// Whether app should exit
    pub should_quit: bool,
    /// Scroll offset for details panel
    pub details_scroll: u16,
}

impl App {
    pub fn new(audiobooks: Vec<AudiobookRecord>) -> Self {
        let filtered: Vec<usize> = (0..audiobooks.len()).collect();
        Self {
            audiobooks,
            filtered,
            selected: 0,
            search_query: String::new(),
            filter_field: FilterField::default(),
            focus: Focus::default(),
            mode: Mode::default(),
            should_quit: false,
            details_scroll: 0,
        }
    }

    /// Get the currently selected audiobook, if any
    pub fn selected_audiobook(&self) -> Option<&AudiobookRecord> {
        self.filtered
            .get(self.selected)
            .and_then(|&idx| self.audiobooks.get(idx))
    }

    /// Apply current search query and filter field
    pub fn apply_filter(&mut self) {
        let query = self.search_query.to_lowercase();

        self.filtered = self
            .audiobooks
            .iter()
            .enumerate()
            .filter(|(_, book)| {
                if query.is_empty() {
                    return true;
                }

                match self.filter_field {
                    FilterField::All => {
                        book.title.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false)
                            || book.author.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false)
                            || book.series.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false)
                            || book.narrator.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false)
                            || book.description.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false)
                    }
                    FilterField::Author => {
                        book.author.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false)
                    }
                    FilterField::Series => {
                        book.series.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false)
                    }
                    FilterField::Genre => {
                        book.genre.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false)
                    }
                    FilterField::Narrator => {
                        book.narrator.as_ref().map(|s| s.to_lowercase().contains(&query)).unwrap_or(false)
                    }
                }
            })
            .map(|(idx, _)| idx)
            .collect();

        // Reset selection if out of bounds
        if self.selected >= self.filtered.len() {
            self.selected = self.filtered.len().saturating_sub(1);
        }
        // Reset details scroll
        self.details_scroll = 0;
    }

    /// Move selection up
    pub fn select_previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.details_scroll = 0;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
            self.details_scroll = 0;
        }
    }

    /// Jump up by n items
    pub fn select_previous_page(&mut self, n: usize) {
        self.selected = self.selected.saturating_sub(n);
        self.details_scroll = 0;
    }

    /// Jump down by n items
    pub fn select_next_page(&mut self, n: usize) {
        self.selected = (self.selected + n).min(self.filtered.len().saturating_sub(1));
        self.details_scroll = 0;
    }

    /// Jump to first item
    pub fn select_first(&mut self) {
        self.selected = 0;
        self.details_scroll = 0;
    }

    /// Jump to last item
    pub fn select_last(&mut self) {
        self.selected = self.filtered.len().saturating_sub(1);
        self.details_scroll = 0;
    }

    /// Cycle to next filter field
    pub fn cycle_filter(&mut self) {
        self.filter_field = self.filter_field.next();
        self.apply_filter();
    }

    /// Add character to search query
    pub fn search_push(&mut self, c: char) {
        self.search_query.push(c);
        self.apply_filter();
    }

    /// Remove last character from search query
    pub fn search_pop(&mut self) {
        self.search_query.pop();
        self.apply_filter();
    }

    /// Clear search query
    pub fn search_clear(&mut self) {
        self.search_query.clear();
        self.apply_filter();
    }

    /// Scroll details panel up
    pub fn scroll_details_up(&mut self) {
        self.details_scroll = self.details_scroll.saturating_sub(1);
    }

    /// Scroll details panel down
    pub fn scroll_details_down(&mut self) {
        self.details_scroll = self.details_scroll.saturating_add(1);
    }
}
```

**Step 3: Register module in main.rs**

Add after line 9 in `src/main.rs`:
```rust
mod tui;
```

**Step 4: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Commit**

```bash
git add src/tui/mod.rs src/tui/state.rs src/main.rs
git commit -m "feat(tui): add application state module"
```

---

## Task 4: Create UI Rendering Module

**Files:**
- Create: `src/tui/ui.rs`
- Modify: `src/tui/mod.rs`

**Step 1: Create UI module**

Create `src/tui/ui.rs`:

```rust
//! UI rendering for the TUI

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::state::{App, Focus, Mode};

/// Format duration in seconds to "Xh Ym" format
fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    if hours > 0 {
        format!("{}h {}m", hours, minutes)
    } else {
        format!("{}m", minutes)
    }
}

/// Render the entire UI
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Check minimum size
    if area.width < 80 || area.height < 24 {
        let msg = Paragraph::new("Terminal too small. Minimum: 80x24")
            .style(Style::default().fg(Color::Red));
        frame.render_widget(msg, area);
        return;
    }

    // Main layout: header, body, footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with search
            Constraint::Min(10),   // Body
            Constraint::Length(1), // Footer
        ])
        .split(area);

    render_header(frame, app, chunks[0]);
    render_body(frame, app, chunks[1]);
    render_footer(frame, app, chunks[2]);

    // Render help overlay if in help mode
    if app.mode == Mode::Help {
        render_help_overlay(frame, area);
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(20),    // Title + search
            Constraint::Length(20), // Filter dropdown
        ])
        .split(area);

    // Title and search
    let search_style = if app.mode == Mode::Searching {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let cursor = if app.mode == Mode::Searching { "_" } else { "" };
    let search_text = format!("Search: {}{}", app.search_query, cursor);

    let title_block = Block::default()
        .title(" audiobookctl ")
        .borders(Borders::ALL);

    let search = Paragraph::new(search_text)
        .style(search_style)
        .block(title_block);

    frame.render_widget(search, chunks[0]);

    // Filter dropdown
    let filter_text = format!("[Filter: {}]", app.filter_field.label());
    let filter = Paragraph::new(filter_text)
        .style(Style::default().fg(Color::Cyan))
        .block(Block::default().borders(Borders::ALL));

    frame.render_widget(filter, chunks[1]);
}

fn render_body(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_list(frame, app, chunks[0]);
    render_details(frame, app, chunks[1]);
}

fn render_list(frame: &mut Frame, app: &App, area: Rect) {
    let title = format!(
        " AUDIOBOOKS ({} total, {} shown) ",
        app.audiobooks.len(),
        app.filtered.len()
    );

    let border_style = if app.focus == Focus::List {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title(title)
        .borders(Borders::ALL)
        .border_style(border_style);

    if app.filtered.is_empty() {
        let msg = if app.search_query.is_empty() {
            "No audiobooks indexed"
        } else {
            "No matches"
        };
        let empty = Paragraph::new(msg)
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(empty, area);
        return;
    }

    // Calculate visible items based on area height
    let inner_height = area.height.saturating_sub(2) as usize; // -2 for borders
    let items_per_page = inner_height / 2; // 2 lines per item

    // Calculate scroll offset to keep selected item visible
    let scroll_offset = if items_per_page > 0 {
        (app.selected / items_per_page) * items_per_page
    } else {
        0
    };

    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .skip(scroll_offset)
        .take(items_per_page)
        .enumerate()
        .filter_map(|(display_idx, &book_idx)| {
            let book = app.audiobooks.get(book_idx)?;
            let actual_idx = scroll_offset + display_idx;
            let is_selected = actual_idx == app.selected;

            let title = book.title.as_deref().unwrap_or("Unknown Title");
            let author = book.author.as_deref().unwrap_or("Unknown");

            let series_info = match (&book.series, book.series_position) {
                (Some(s), Some(pos)) => format!("{} #{}", s, pos),
                (Some(s), None) => s.clone(),
                _ => "Standalone".to_string(),
            };

            let duration = book
                .duration_seconds
                .map(format_duration)
                .unwrap_or_default();

            let line1 = if is_selected {
                Line::from(vec![
                    Span::raw("> "),
                    Span::styled(title, Style::default().add_modifier(Modifier::BOLD)),
                ])
            } else {
                Line::from(vec![
                    Span::raw("  "),
                    Span::raw(title),
                ])
            };

            let line2_style = if is_selected {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let line2 = Line::from(vec![
                Span::raw("  "),
                Span::styled(
                    format!("{} · {} · {}", author, series_info, duration),
                    line2_style,
                ),
            ]);

            Some(ListItem::new(vec![line1, line2]))
        })
        .collect();

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
}

fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    let border_style = if app.focus == Focus::Details {
        Style::default().fg(Color::Yellow)
    } else {
        Style::default()
    };

    let block = Block::default()
        .title(" DETAILS ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let Some(book) = app.selected_audiobook() else {
        let empty = Paragraph::new("No audiobook selected")
            .style(Style::default().fg(Color::DarkGray))
            .block(block);
        frame.render_widget(empty, area);
        return;
    };

    let mut lines: Vec<Line> = Vec::new();

    // Title
    if let Some(ref title) = book.title {
        lines.push(Line::from(vec![
            Span::styled("Title: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(title),
        ]));
    }

    // Author
    if let Some(ref author) = book.author {
        lines.push(Line::from(vec![
            Span::styled("Author: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(author),
        ]));
    }

    // Narrator
    if let Some(ref narrator) = book.narrator {
        lines.push(Line::from(vec![
            Span::styled("Narrator: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(narrator),
        ]));
    }

    // Series
    if let Some(ref series) = book.series {
        let series_text = match book.series_position {
            Some(pos) => format!("{} #{}", series, pos),
            None => series.clone(),
        };
        lines.push(Line::from(vec![
            Span::styled("Series: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(series_text),
        ]));
    }

    // Year
    if let Some(year) = book.year {
        lines.push(Line::from(vec![
            Span::styled("Year: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(year.to_string()),
        ]));
    }

    // Duration
    if let Some(secs) = book.duration_seconds {
        lines.push(Line::from(vec![
            Span::styled("Duration: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format_duration(secs)),
        ]));
    }

    // Genre
    if let Some(ref genre) = book.genre {
        lines.push(Line::from(vec![
            Span::styled("Genre: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(genre),
        ]));
    }

    // Publisher
    if let Some(ref publisher) = book.publisher {
        lines.push(Line::from(vec![
            Span::styled("Publisher: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(publisher),
        ]));
    }

    // ASIN/ISBN
    if let Some(ref asin) = book.asin {
        lines.push(Line::from(vec![
            Span::styled("ASIN: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(asin),
        ]));
    }
    if let Some(ref isbn) = book.isbn {
        lines.push(Line::from(vec![
            Span::styled("ISBN: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(isbn),
        ]));
    }

    // Blank line before description
    if book.description.is_some() {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Description:",
            Style::default().add_modifier(Modifier::BOLD),
        )));
    }

    // Description
    if let Some(ref desc) = book.description {
        lines.push(Line::from(desc.as_str()));
    }

    let paragraph = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: true })
        .scroll((app.details_scroll, 0));

    frame.render_widget(paragraph, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.mode {
        Mode::Searching => "Type to search · Esc clear · Enter confirm",
        Mode::Help => "Press any key to close",
        Mode::Normal => "↑↓/jk Navigate · / Search · f Filter · Tab Switch · ? Help · q Quit",
    };

    let footer = Paragraph::new(help_text)
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(footer, area);
}

fn render_help_overlay(frame: &mut Frame, area: Rect) {
    // Center the help dialog
    let width = 50.min(area.width.saturating_sub(4));
    let height = 16.min(area.height.saturating_sub(4));
    let x = (area.width - width) / 2;
    let y = (area.height - height) / 2;
    let help_area = Rect::new(x, y, width, height);

    // Clear the area behind
    frame.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("↑ / k        Move up"),
        Line::from("↓ / j        Move down"),
        Line::from("PgUp / PgDn  Jump 10 items"),
        Line::from("Home / End   Jump to first/last"),
        Line::from("/            Start search"),
        Line::from("f            Cycle filter field"),
        Line::from("Tab          Switch pane focus"),
        Line::from("Esc          Clear search / exit mode"),
        Line::from("q            Quit"),
        Line::from("?            Toggle this help"),
        Line::from(""),
        Line::from(Span::styled(
            "Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(help_text).block(
        Block::default()
            .title(" Help ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow)),
    );

    frame.render_widget(help, help_area);
}
```

**Step 2: Update mod.rs**

Add to `src/tui/mod.rs`:

```rust
//! Terminal UI for browsing audiobooks

mod state;
mod ui;

pub use state::{App, FilterField, Focus, Mode};
pub use ui::render;
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/tui/ui.rs src/tui/mod.rs
git commit -m "feat(tui): add UI rendering module"
```

---

## Task 5: Create Event Handling Module

**Files:**
- Create: `src/tui/events.rs`
- Modify: `src/tui/mod.rs`

**Step 1: Create events module**

Create `src/tui/events.rs`:

```rust
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
```

**Step 2: Update mod.rs**

Update `src/tui/mod.rs`:

```rust
//! Terminal UI for browsing audiobooks

mod events;
mod state;
mod ui;

pub use events::{handle_key, poll_event};
pub use state::{App, FilterField, Focus, Mode};
pub use ui::render;
```

**Step 3: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/tui/events.rs src/tui/mod.rs
git commit -m "feat(tui): add event handling module"
```

---

## Task 6: Create Browse Command

**Files:**
- Create: `src/commands/browse.rs`
- Modify: `src/commands/mod.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

**Step 1: Create browse command**

Create `src/commands/browse.rs`:

```rust
//! Browse command - interactive TUI for audiobook library

use anyhow::Result;
use crossterm::{
    event::Event,
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::{io::stdout, path::Path, time::Duration};

use crate::database::LibraryDb;
use crate::tui::{handle_key, poll_event, render, App};

/// Run the browse TUI
pub fn run(db_path: Option<&Path>) -> Result<()> {
    // Open database
    let db = if let Some(path) = db_path {
        LibraryDb::open(path)?
    } else {
        let cwd = std::env::current_dir()?;
        LibraryDb::find_from(&cwd)?.ok_or_else(|| {
            anyhow::anyhow!(
                "No database found. Run 'audiobookctl index <dir>' first, or specify --db"
            )
        })?
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
        if let Some(event) = poll_event(Duration::from_millis(100))? {
            if let Event::Key(key) = event {
                handle_key(&mut app, key);
            }
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
```

**Step 2: Update commands/mod.rs**

Read the current `src/commands/mod.rs` and add:
```rust
pub mod browse;
```

**Step 3: Update cli.rs**

Add to the `Commands` enum after the existing commands:

```rust
    /// Browse audiobook library interactively
    Browse {
        /// Path to database directory (auto-detected if not specified)
        #[arg(long)]
        db: Option<PathBuf>,
    },
```

**Step 4: Update main.rs**

Add the match arm for Browse in the main function, after the other commands:

```rust
        Commands::Browse { db } => {
            commands::browse::run(db.as_deref())?;
        }
```

**Step 5: Verify it compiles**

Run: `cargo check`
Expected: Compiles successfully

**Step 6: Test manually**

Run: `cargo run -- browse --help`
Expected: Shows browse command help

**Step 7: Commit**

```bash
git add src/commands/browse.rs src/commands/mod.rs src/cli.rs src/main.rs
git commit -m "feat: add browse command for interactive TUI"
```

---

## Task 7: Final Testing and Cleanup

**Step 1: Run clippy**

Run: `cargo clippy`
Expected: No warnings (or only pre-existing ones)

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Test the TUI manually**

Run against a real database:
```bash
cargo run -- browse --db /path/to/your/audiobook/library
```

Test:
- Navigation (up/down/j/k)
- Search (/ to start, type, Esc to clear)
- Filter cycling (f)
- Pane switching (Tab)
- Help overlay (?)
- Quit (q)

**Step 4: Final commit (if any fixes needed)**

```bash
git add -A
git commit -m "fix: address clippy warnings and polish TUI"
```
