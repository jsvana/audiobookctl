# Batch Lookup & Display Improvements Design

## Overview

Two related improvements to the lookup functionality:
1. New `lookup-all` command for batch metadata lookup across directories
2. Deduplicated source display with early-exit when no changes

## Batch Lookup Command

### Command Interface

```
audiobookctl lookup-all <directory>
```

**Flags:**
- `--auto-accept` - Auto-apply changes when all sources agree (skip editor)
- `--no-dry-run` - Actually write changes (same as existing `lookup`)
- `--yes` - Skip confirmation prompts
- `--no-backup-i-void-my-warranty` - Skip backups

### Queue Mode Flow (Default)

1. Recursively scan directory for `.m4b` files (reuse existing scanner)
2. Query APIs for each file (with progress indicator)
3. Filter to only files with differences
4. Print summary: `Found 12 files with available updates (47 already up to date)`
5. Process each file interactively, showing `[3/12] Processing book.m4b...`
6. For skipped files, print: `book.m4b: metadata matches [audible, openlibrary] - skipping`

### Auto-Accept Mode (`--auto-accept`)

- If all sources agree on a value that differs from file → auto-apply
- If sources conflict → still open editor for that file
- Print what was auto-applied: `book.m4b: auto-applied title, narrator from [audible, openlibrary]`

### Backup Storage Limits

**Configuration** (in `~/.config/audiobookctl/config.toml`):
```toml
[backups]
max_storage_bytes = 2147483648  # 2GB default
```

**Enforcement flow:**
1. Calculate current backup usage (sum of `*.m4b.bak` files)
2. Before processing queue, check: `current_backups + sum(queued_file_sizes) > max_storage`
3. If over limit, truncate queue and warn:
   ```
   Found 12 files with updates, but backup limit (2GB) allows only 5.
   Current backup usage: 1.2GB
   Run `audiobookctl backups clean` or increase limit in config.
   Processing first 5 files...
   ```

### Backup Management Subcommand

```
audiobookctl backups list   # Show all .bak files and total size
audiobookctl backups clean  # Interactive cleanup (or --all to remove all)
```

## Deduplicated Source Display

### Agreed Values

Collapse sources into single comment:
```toml
title = "The Martian"  # [file, audible, openlibrary]
narrator = "R.C. Bray"  # [audible, audnexus]
```

### Conflicting Values

Group sources by value:
```toml
# year: Sources disagree - pick one:
year = "2014"  # [file, audible]
# year = "2011"  # [openlibrary, audnexus]
```

### Early Exit

When all lookup results match existing file metadata:
```
book.m4b: metadata matches [audible, openlibrary] - skipping
```

Applies to both single-file `lookup` and batch `lookup-all`.

## File Structure

### New Files

- `src/commands/lookup_all.rs` - Batch command orchestration
- `src/commands/backups.rs` - Backup management subcommand
- `src/config.rs` - Configuration loading

### Modified Files

- `src/commands/lookup.rs` - Extract shared logic, add early-exit check
- `src/lookup/merge.rs` - Change `FieldValue::Conflicting` to group by value
- `src/cli.rs` - Add `LookupAll` and `Backups` commands

### Shared Logic

```
lookup.rs::run()           lookup_all.rs::run()
         \                    /
          \                  /
           v                v
      lookup.rs (shared pub fns)
        - query_and_merge(file) -> MergedResult
        - check_no_changes(merged) -> bool
        - process_single_file(file, merged, flags)
```

## Configuration

Location: `~/.config/audiobookctl/config.toml`

```toml
[backups]
max_storage_bytes = 2147483648  # 2GB default
```

Falls back to defaults if file doesn't exist.
