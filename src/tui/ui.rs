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
                Line::from(vec![Span::raw("  "), Span::raw(title)])
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

    let footer = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));

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
