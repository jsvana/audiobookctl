# Phase 2: Edit Command Design

## Overview

The `edit` command allows users to modify m4b audiobook metadata using their preferred text editor with a TOML format, side-by-side diff preview, and safety-first defaults.

## Command Flow

### Dry-run mode (default)

```bash
audiobookctl edit book.m4b
```

1. Read metadata from m4b file
2. Serialize to TOML (empty fields commented out)
3. Write to temp file, open in `$EDITOR`
4. Wait for editor to close
5. Parse edited TOML
6. Show side-by-side diff (old | new)
7. Save to `~/.cache/audiobookctl/pending/<hash>.toml`
8. Print: `To apply: audiobookctl edit book.m4b --no-dry-run`

### Apply mode

```bash
audiobookctl edit book.m4b --no-dry-run
```

1. Check for pending edit at `~/.cache/audiobookctl/pending/<hash>.toml`
2. If found: load it (skip editor)
3. If not found: run full edit flow above
4. Show side-by-side diff
5. Prompt: `Apply these changes to book.m4b? [y/N]` (skip with `--yes`)
6. Create backup: `book.m4b.bak`
   - Print: `Created backup: /path/to/book.m4b.bak`
7. Write changes to m4b
8. Clear pending file

## TOML Format

```toml
# Audiobook Metadata - Edit and save to apply changes
# Commented fields are empty - uncomment and fill to add values

title = "Project Hail Mary"
author = "Andy Weir"
# narrator = ""
series = "Standalone"
# series_position = 0
year = 2021
description = "Ryland Grace is the sole survivor..."
publisher = "Audible Studios"
genre = "Science Fiction"
isbn = "978-0593135204"
asin = "B08G9PRS1K"

# Read-only (cannot be edited)
# duration = "16:10:35"
# chapters = 32
# cover = "embedded (1400x1400 JPEG)"
```

**Rules:**
- Fields with values: shown normally
- Empty editable fields: commented with empty value placeholder
- Read-only fields: commented with actual value and note
- Header comment explains the format

## Side-by-Side Diff Display

```
Changes to book.m4b:

  Field          | Current                | New
 ----------------+------------------------+------------------------
  narrator       | (empty)                | Ray Porter
  series         | Standalone             | (empty)
  genre          | Science Fiction        | Sci-Fi
```

**Rules:**
- Only show changed fields (not unchanged)
- "(empty)" for None/missing values
- Truncate long values with "..." if needed
- Use box-drawing characters for clean table layout
- No changes: print "No changes detected." and exit

## Pending Edits Cache

**Location:** `~/.cache/audiobookctl/pending/<hash>.toml`

**Hash calculation:**
- SHA256 of absolute file path
- Truncated to 16 hex chars for filename
- Example: `/home/user/books/book.m4b` → `a3f2dd91bc4e7810.toml`

**Cache file format:**
```toml
# Pending edit for: /home/user/books/book.m4b
# Created: 2025-12-27T14:30:00Z
# Run: audiobookctl edit "/home/user/books/book.m4b" --no-dry-run

title = "Project Hail Mary"
author = "Andy Weir"
narrator = "Ray Porter"
# ... rest of metadata
```

**Commands:**
- `audiobookctl edit --clear` - clear all pending edits
- `audiobookctl edit book.m4b --clear` - clear pending edit for specific file

**Behavior:**
- Pending edit never expires (user must explicitly apply or clear)
- If file modified since pending edit: warn and ask to re-edit or apply anyway

## Backup & Safety

**Backup creation:**
```
book.m4b → book.m4b.bak
```

- Created immediately before writing changes
- If `.bak` already exists: overwrite it (only keep most recent backup)
- Print: `Created backup: /path/to/book.m4b.bak`

**Skip backup flag:**
```
--no-backup-i-void-my-warranty
```

- Deliberately long and scary
- Skips backup creation entirely
- Prints warning: `Warning: No backup created. Changes cannot be undone.`

**Confirmation prompt:**
```
Apply these changes to book.m4b? [y/N]
```

- Default is No (must type `y` or `yes`)
- `--yes` flag skips prompt (for scripting)

**Commit (delete backup after verification):**
```bash
audiobookctl edit book.m4b --commit
# Delete backup for specific file after verifying change is okay

audiobookctl edit --commit-all
# Find and delete all .bak files in current directory (recursive)
# Shows list and total size, prompts for confirmation
```

**`--commit-all` output:**
```
Found 3 backup files (1.7 GB total):
  /home/user/books/book1.m4b.bak (523 MB)
  /home/user/books/book2.m4b.bak (612 MB)
  /home/user/books/series/book3.m4b.bak (589 MB)

Delete all backups? [y/N]
```

**Error handling:**
- If backup fails (permissions, disk full): abort with error, don't modify original
- If write fails: original file untouched (backup was already made)
- If m4b is malformed: error at read stage, nothing modified

## CLI Arguments

```rust
#[derive(Subcommand)]
pub enum Commands {
    /// Edit metadata in $EDITOR with diff preview
    Edit {
        /// Path to the m4b file (optional for --clear/--commit-all)
        file: Option<PathBuf>,

        /// Actually apply changes (default: dry-run)
        #[arg(long)]
        no_dry_run: bool,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,

        /// Skip creating backup file
        #[arg(long = "no-backup-i-void-my-warranty")]
        no_backup: bool,

        /// Clear pending edit(s)
        #[arg(long)]
        clear: bool,

        /// Delete backup after verifying changes
        #[arg(long)]
        commit: bool,

        /// Delete all backup files recursively
        #[arg(long)]
        commit_all: bool,
    },
}
```

## New Dependencies

- `toml` - TOML parsing/serialization
- `dirs` - XDG cache directory (`~/.cache`)
- `sha2` - SHA256 for path hashing
- `walkdir` - For `--commit-all` recursive search
- `chrono` - Timestamps for pending edit files

## New Files

```
src/
├── commands/
│   └── edit.rs           # Edit command
├── metadata/
│   └── writer.rs         # Write metadata to m4b
├── editor/
│   ├── mod.rs
│   ├── toml.rs           # TOML serialization with comments
│   └── diff.rs           # Side-by-side diff display
└── safety/
    ├── mod.rs
    ├── backup.rs         # .bak file handling
    └── pending.rs        # Pending edits cache
```
