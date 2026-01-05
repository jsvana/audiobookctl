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
                        book.title
                            .as_ref()
                            .map(|s| s.to_lowercase().contains(&query))
                            .unwrap_or(false)
                            || book
                                .author
                                .as_ref()
                                .map(|s| s.to_lowercase().contains(&query))
                                .unwrap_or(false)
                            || book
                                .series
                                .as_ref()
                                .map(|s| s.to_lowercase().contains(&query))
                                .unwrap_or(false)
                            || book
                                .narrator
                                .as_ref()
                                .map(|s| s.to_lowercase().contains(&query))
                                .unwrap_or(false)
                            || book
                                .description
                                .as_ref()
                                .map(|s| s.to_lowercase().contains(&query))
                                .unwrap_or(false)
                    }
                    FilterField::Author => book
                        .author
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&query))
                        .unwrap_or(false),
                    FilterField::Series => book
                        .series
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&query))
                        .unwrap_or(false),
                    FilterField::Genre => book
                        .genre
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&query))
                        .unwrap_or(false),
                    FilterField::Narrator => book
                        .narrator
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&query))
                        .unwrap_or(false),
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
