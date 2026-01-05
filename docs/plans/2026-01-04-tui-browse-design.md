# TUI Browse Feature Design

## Overview

A terminal user interface for browsing and discovering audiobooks in the collection. Reads from the SQLite database created by `audiobookctl index`.

**Primary use case:** Navigate the collection by browsing and filtering to find what to listen to next.

## Layout

```
┌─────────────────────────────────────────────────────────────────────────────┐
│ audiobookctl                                       [Filter: Author ▼] _____ │
│ Search: _____________________________________________________________       │
├─────────────────────────────────────────┬───────────────────────────────────┤
│ AUDIOBOOKS (247 items, 3 filtered)      │ DETAILS                           │
│─────────────────────────────────────────│───────────────────────────────────│
│ > Project Hail Mary                     │ Title: Project Hail Mary          │
│   Andy Weir · Standalone · 16h 10m      │ Author: Andy Weir                 │
│                                         │ Narrator: Ray Porter              │
│   The Martian                           │ Year: 2021                        │
│   Andy Weir · Standalone · 10h 53m      │ Duration: 16h 10m                 │
│                                         │ Genre: Science Fiction            │
│   Dune                                  │                                   │
│   Frank Herbert · Dune #1 · 21h 2m      │ Description:                      │
│                                         │ Ryland Grace is the sole survivor │
│   Children of Dune                      │ on a desperate, last-chance       │
│   Frank Herbert · Dune #3 · 16h 6m      │ mission...                        │
│                                         │                                   │
├─────────────────────────────────────────┴───────────────────────────────────┤
│ ↑↓ Navigate  Enter Details  / Search  f Filter  Tab Switch pane  ? Help    │
└─────────────────────────────────────────────────────────────────────────────┘
```

**Three main regions:**
- **Header** - App title, filter dropdown, search input
- **Body** - Split into list (~50%) and detail panel (~50%)
- **Footer** - Context-sensitive keyboard shortcuts

**List item format:**
- Line 1: Title
- Line 2: Author · Series info · Duration
- Series shows as "Series Name #N" or "Standalone" if no series

## Interaction

### Keyboard Controls

| Key | Action |
|-----|--------|
| `↑` / `k` | Move selection up |
| `↓` / `j` | Move selection down |
| `PgUp` / `PgDn` | Jump 10 items |
| `Home` / `End` | Jump to first/last |
| `/` | Focus search input |
| `f` | Cycle filter field (All → Author → Series → Genre → Narrator) |
| `Escape` | Clear search/filter, or exit filter mode |
| `Tab` | Switch focus between list and detail panel |
| `q` | Quit |
| `?` | Toggle help overlay |

### Search Behavior

- Typing in search filters the list in real-time
- Filter field dropdown affects which column is searched
- "All" searches across title, author, series, narrator, and description
- Search is case-insensitive
- Results update as you type (debounced ~100ms for performance)

### Detail Panel

- Shows full metadata for currently selected book
- Scrollable if description is long (when panel has focus)
- Updates instantly as you navigate the list

## Architecture

### New Files

```
src/
├── tui/
│   ├── mod.rs          # Module root, App struct, main loop
│   ├── ui.rs           # Layout and rendering logic
│   ├── state.rs        # Application state (selection, filters, mode)
│   ├── events.rs       # Keyboard event handling
│   └── widgets.rs      # Custom widgets (audiobook list item)
├── commands/
│   └── browse.rs       # New `browse` subcommand entry point
```

### Dependencies

- `ratatui` - TUI framework
- `crossterm` - Terminal backend (cross-platform)

### App State

```rust
struct App {
    audiobooks: Vec<Audiobook>,      // Full list from DB
    filtered: Vec<usize>,            // Indices into audiobooks matching filter
    selected: usize,                 // Index into filtered
    search_query: String,
    filter_field: FilterField,       // All, Author, Series, Genre, Narrator
    focus: Focus,                    // List or DetailPanel
    mode: Mode,                      // Normal, Searching, Help
}
```

### Data Flow

1. Load all audiobooks from SQLite on startup
2. Apply filters in-memory (fast for typical collection sizes)
3. Re-filter on each keystroke in search mode
4. Render based on current state

## CLI Integration

### Command

```bash
audiobookctl browse [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory containing .audiobookctl.db (default: current directory)

Options:
  -d, --database <FILE>  Path to database file directly
  -h, --help             Print help
```

### Startup Behavior

1. Look for `.audiobookctl.db` in specified path (or current directory)
2. If not found, show friendly error: "No database found. Run `audiobookctl index <dir>` first."
3. Load audiobooks, display TUI

## Edge Cases

- **Empty database** - Show "No audiobooks indexed" message in list area
- **No search results** - Show "No matches" with current filter info
- **Very long titles/descriptions** - Truncate with ellipsis in list, wrap in detail panel
- **Missing optional fields** - Omit from detail view (don't show "Narrator: Unknown")
- **Terminal too small** - Minimum 80x24, show warning if smaller

### Graceful Exit

- `q` or `Ctrl+C` exits cleanly
- Restore terminal state on panic (using crossterm's panic hook)
