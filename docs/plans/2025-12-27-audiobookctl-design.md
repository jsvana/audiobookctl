# audiobookctl Design Document

A CLI tool for reading, editing, and organizing m4b audiobook metadata.

## CLI Structure

```
audiobookctl <command> [options] <file(s)>

Commands:
  show     Display metadata for an m4b file
  edit     Edit metadata in $EDITOR with diff preview
  organize Organize files based on metadata (phase 3)

Global flags:
  --no-dry-run                    Actually modify files (default: dry-run)
  --no-backup-i-void-my-warranty  Skip creating .bak files when modifying
  -v, --verbose                   Increase output verbosity
  -q, --quiet                     Suppress non-essential output
```

### show command

```bash
audiobookctl show book.m4b           # Pretty-print all metadata
audiobookctl show book.m4b --json    # JSON output for scripting
audiobookctl show book.m4b --field author  # Single field
```

### edit command

```bash
audiobookctl edit book.m4b
# Opens $EDITOR, shows diff, writes changes to pending cache
# Prints: "To apply: audiobookctl edit book.m4b --no-dry-run"

audiobookctl edit book.m4b --no-dry-run
# Detects pending edit, shows diff again, applies after confirmation
# Clears pending file on success
```

## Metadata Fields

M4b files use MP4/iTunes atom tags:

| Field | M4B Atom | Notes |
|-------|----------|-------|
| title | `©nam` | Book title |
| author | `©ART` | Writer |
| narrator | `©nrt` | Reader/performer |
| series | `tvsh` | Series name |
| series_position | `tves` | Book number in series |
| year | `©day` | Publication year |
| description | `desc` | Long description/synopsis |
| publisher | `©pub` | Publishing house |
| genre | `©gen` | Category |
| cover | `covr` | Embedded artwork (PNG/JPEG) |
| duration | (calculated) | From audio stream, read-only |
| chapters | `chpl` | Chapter markers, read-only in phase 1-2 |
| isbn | `----:com.apple.iTunes:ISBN` | Custom atom |
| asin | `----:com.apple.iTunes:ASIN` | Custom atom |

### TOML representation

```toml
title = "Project Hail Mary"
author = "Andy Weir"
narrator = "Ray Porter"
series = ""
series_position = 0
year = 2021
description = "Ryland Grace is the sole survivor..."
publisher = "Audible Studios"
genre = "Science Fiction"
isbn = "978-0593135204"
asin = "B08G9PRS1K"

# Read-only (shown but not editable)
# duration = "16:10:35"
# chapters = 32
# cover = "embedded (1400x1400 JPEG)"
```

## Safety Model

### Dry-run by default

All modifying operations show what *would* happen without `--no-dry-run`.

### Backup on modify

```
book.m4b → book.m4b.bak  (created before any modification)
```

Skip with `--no-backup-i-void-my-warranty`

### Pending edits

Location: `~/.cache/audiobookctl/pending/<hash-of-filepath>.toml`

- Keyed by full path hash to avoid collisions
- Contains the edited TOML
- Cleared after successful apply or explicit `audiobookctl edit --clear`

### Confirmation

```
Apply these changes to book.m4b? [y/N]
```

Add `--yes` flag to skip prompt for scripting.

## Project Structure

```
audiobookctl/
├── src/
│   ├── main.rs           # CLI entry, clap setup
│   ├── lib.rs            # Public API for library use
│   ├── commands/
│   │   ├── mod.rs
│   │   ├── show.rs       # show command
│   │   └── edit.rs       # edit command
│   ├── metadata/
│   │   ├── mod.rs
│   │   ├── fields.rs     # Metadata struct, field definitions
│   │   ├── reader.rs     # Read from m4b
│   │   └── writer.rs     # Write to m4b
│   ├── editor/
│   │   ├── mod.rs
│   │   ├── toml.rs       # TOML serialization
│   │   └── diff.rs       # Side-by-side diff display
│   └── safety/
│       ├── mod.rs
│       ├── backup.rs     # .bak file creation
│       └── pending.rs    # Pending edits cache
├── tests/
│   └── integration/      # End-to-end tests with sample m4b
├── .github/workflows/
│   ├── ci.yml            # lint, clippy, test
│   └── release.yml       # Binary releases on tag
├── CLAUDE.md
└── Cargo.toml
```

## Dependencies

- `clap` - CLI parsing with derive
- `mp4ameta` - M4B/M4A tag reading/writing (pure Rust)
- `toml` - TOML serialization
- `serde` - Serialization framework
- `dirs` - XDG paths for cache directory
- `colored` - Terminal output formatting

## Implementation Phases

### Phase 1 - Show command

- Parse m4b metadata with mp4ameta
- Pretty-print all fields with formatting
- `--json` output for scripting
- `--field <name>` for single field

### Phase 2 - Edit command

- Serialize metadata to TOML temp file
- Open in `$EDITOR`
- Parse edited TOML, compute diff
- Display side-by-side diff
- Save to pending cache (dry-run) or apply with backup (--no-dry-run)

### Phase 3 - Organize

Details TBD - will gather requirements when beginning this phase.

## CI/CD

### ci.yml

- Triggers: push to main, all PRs
- Matrix: stable + beta Rust
- Steps: `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`

### release.yml

- Triggers: tags matching `v*`
- Builds: Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64)
- Creates GitHub Release with binaries
- Uses `cross` for cross-compilation
