# Idempotent Organize & Local Search Database

**Date**: 2026-01-02
**Status**: Approved

## Overview

Two related features:
1. Make `organize` idempotent using SHA256 hash comparison
2. Build a SQLite database during organization for local `search` queries

## Feature 1: Idempotent Organize with SHA256

### Current Behavior

When destination path exists OR multiple sources map to same destination, create a `Conflict` and refuse to proceed.

### New Behavior

1. When a destination file already exists, compute SHA256 of both source and destination
2. If hashes match: Create `AlreadyPresent` operation - no conflict, skip copy
3. If hashes differ: Still a conflict (different file, same name)
4. Multiple sources mapping to same destination: Still a conflict (can't auto-resolve)

### Output

```
Organize Plan:
  ✓ Copy: book1.m4b → Author/Title/book1.m4b
  ≡ Already present: book2.m4b → Author/Title2/book2.m4b (hash match)
  ✗ Conflict: book3.m4b → Author/Title3/book3.m4b (different content)
```

### Implementation Notes

- Add `sha2` dependency for SHA256
- Compute hash lazily only when destination exists
- Hash computation streams file to avoid loading entire file into memory

## Feature 2: SQLite Database

### Storage Location

Per-destination database: `<destination>/.audiobookctl.db`

Benefits:
- Portable - database travels with library
- Self-contained - no external state
- Multiple libraries supported naturally

### Schema

```sql
CREATE TABLE audiobooks (
    id INTEGER PRIMARY KEY,
    -- File info
    file_path TEXT NOT NULL UNIQUE,  -- Relative to database location
    file_size INTEGER NOT NULL,
    sha256 TEXT NOT NULL,
    indexed_at TEXT NOT NULL,        -- ISO8601 timestamp

    -- Core metadata
    title TEXT,
    author TEXT,
    narrator TEXT,
    series TEXT,
    series_position REAL,            -- Supports "2.5" style positions
    year INTEGER,

    -- Extended metadata
    description TEXT,
    publisher TEXT,
    genre TEXT,
    asin TEXT,
    isbn TEXT,
    duration_seconds INTEGER,
    chapter_count INTEGER
);

CREATE INDEX idx_author ON audiobooks(author);
CREATE INDEX idx_title ON audiobooks(title);
CREATE INDEX idx_series ON audiobooks(series);
CREATE INDEX idx_sha256 ON audiobooks(sha256);
```

### Indexing

**During organize** (automatic):
- After successfully copying a file, insert/update record in database
- For "already present" files (hash match), update `indexed_at` timestamp
- Database created automatically if it doesn't exist

**New `index` command** (manual):
```bash
audiobookctl index <directory>        # Index all m4b files
audiobookctl index <directory> --full # Re-index everything
audiobookctl index <directory> --prune # Remove entries for missing files
```

Behavior:
- Walks directory finding all `.m4b` files
- Skips files already in DB with matching path + size + mtime (fast check)
- `--full` forces SHA256 recomputation and metadata re-read
- `--prune` removes DB entries where file no longer exists

## Feature 3: Search Command (Repurposed)

### Changes

- **Remove**: All API query functionality (Audible, Audnexus, OpenLibrary)
- **Add**: Local SQLite database queries

Note: The `lookup` command retains API functionality for fetching metadata for specific files.

### Interface

```bash
# Free-text search (searches title, author, narrator, series, description)
audiobookctl search "Sanderson"

# Field-specific filters (combinable)
audiobookctl search --author "Sanderson" --series "Stormlight"
audiobookctl search --year 2020
audiobookctl search --asin B08...

# Combined
audiobookctl search "Mistborn" --author "Sanderson"

# Options
audiobookctl search "query" --db <path>    # Specify database location
audiobookctl search "query" --json         # JSON output
audiobookctl search "query" --limit 20     # Limit results (default: 50)
```

### Database Discovery

1. If `--db` provided, use that
2. Look for `.audiobookctl.db` in current directory
3. Walk up parent directories looking for `.audiobookctl.db`
4. Error if no database found

### Output

Table format showing title, author, series, file path. `--json` for full details.

## Implementation Plan

### Files to Modify

- `src/organize/planner.rs` - Add hash comparison, `AlreadyPresent` variant
- `src/commands/organize.rs` - Display "already present", integrate DB writes
- `src/commands/search.rs` - Complete rewrite for local DB queries
- `src/cli.rs` - Add index subcommand, update search args

### Files to Add

- `src/commands/index.rs` - New index command
- `src/database/mod.rs` - SQLite operations (create, insert, query, prune)
- `src/hash.rs` - SHA256 computation utility

### New Dependencies

- `rusqlite` - SQLite bindings (with bundled feature for portability)
- `sha2` - SHA256 hashing

### Implementation Order

1. Add SHA256 utility module
2. Implement idempotent organize (hash comparison in planner)
3. Add database module with schema and operations
4. Integrate DB writes into organize command
5. Implement index command
6. Rewrite search command for local queries
