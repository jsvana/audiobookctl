# db export Command Design

## Overview

Add a `db export` command that exports the audiobook database as CSV to stdout.

## Command Interface

```bash
# Basic usage - core metadata
audiobookctl db export

# Full export with file info
audiobookctl db export --full
```

## Output Columns

**Core columns (default):**
- title, author, narrator, series, series_position, year, genre, publisher

**Full columns (--full flag):**
- All core columns plus: description, duration_seconds, chapter_count, isbn, asin, file_path, file_size, sha256, indexed_at

## Behavior

- Auto-detects `.audiobookctl.db` by walking up from current directory
- Outputs CSV to stdout (users redirect with `> file.csv` if needed)
- Exports entire database (no filtering)
- Errors if no database found

## Implementation

**Files to modify:**
1. `src/commands/db_export.rs` (new) - Command implementation
2. `src/cli.rs` - Add `DbExport` variant to `Commands` enum
3. `src/main.rs` - Add dispatch for the new command
4. `src/commands/mod.rs` - Export the new module

**Dependencies:**
- Add `csv` crate for CSV output

**Error handling:**
- No database found: error with hint to run `index` first
- Empty database: output header row only
