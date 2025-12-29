# Design: `audiobookctl pending` Command

Bulk show and apply operations for pending edits.

## Command Structure

```
audiobookctl pending list          # Summary of all pending edits
audiobookctl pending list --diff   # Summary + diff preview for each
audiobookctl pending show <file>   # Show diff for specific file's pending edit
audiobookctl pending apply         # Apply all pending edits (with confirmation)
audiobookctl pending apply <file>  # Apply specific pending edit
audiobookctl pending clear         # Clear all pending edits
audiobookctl pending clear <file>  # Clear specific pending edit
```

## Output Formats

### `pending list`

```
Pending edits:
  /path/to/book1.m4b (edit saved 2025-12-28 14:30)
  /path/to/book2.m4b (edit saved 2025-12-29 09:15)

2 pending edit(s)
```

### `pending apply` (bulk)

```
Applying 2 pending edit(s)...
  ✓ /path/to/book1.m4b
  ✗ /path/to/book2.m4b (file not found)

Applied: 1, Failed: 1
```

## Behavior

- **apply**: One confirmation prompt for all files, then process sequentially
- **apply error handling**: Skip failed files (e.g., file moved/deleted), continue with remaining, show summary at end
- **apply flags**: `--yes` (skip confirmation), `--no-backup-i-void-my-warranty` (skip backups)

## Implementation

### CLI Changes (`src/cli.rs`)

Add `Pending` variant to `Commands` enum with `PendingAction` subcommand:

```rust
Pending {
    #[command(subcommand)]
    action: PendingAction,
}

enum PendingAction {
    List { diff: bool },
    Show { file: PathBuf },
    Apply { file: Option<PathBuf>, yes: bool, no_backup: bool },
    Clear { file: Option<PathBuf> },
}
```

Remove `--clear` flag from `Edit` command (replaced by `pending clear`).

### PendingEditsCache (`src/safety/pending.rs`)

Add method:

```rust
pub fn list_all(&self) -> Result<Vec<PendingEdit>>
```

Enumerates all `.toml` files in the cache directory and loads each.

### Files Changed

| File | Change |
|------|--------|
| `src/safety/pending.rs` | Add `list_all()` method |
| `src/cli.rs` | Add `Pending` command + `PendingAction` enum, remove `--clear` from `Edit` |
| `src/commands/pending.rs` | New file with `run()` and subcommand handlers |
| `src/commands/mod.rs` | Add `pub mod pending;` |
| `src/main.rs` | Add match arm for `Commands::Pending` |
| `src/commands/edit.rs` | Remove `handle_clear()` function and `clear` parameter |
