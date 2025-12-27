# audiobookctl

CLI tool for reading, editing, and organizing m4b audiobook metadata.

## Project Principles

**Data safety is paramount.** Audiobook files are irreplaceable user data.

- All modifying operations are dry-run by default
- Use `--no-dry-run` to actually apply changes
- Backups (.bak files) are created before any modification
- Use `--no-backup-i-void-my-warranty` to skip backups (strongly discouraged)

## Build & Test

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run all tests
cargo clippy             # Lint check
cargo fmt --check        # Format check
```

## Project Structure

- `src/commands/` - CLI command implementations (show, edit, organize)
- `src/metadata/` - M4B metadata reading/writing
- `src/editor/` - TOML editing and diff display
- `src/safety/` - Backup and pending edit handling
- `tests/integration/` - End-to-end tests

## CLI Commands

```bash
audiobookctl show <file>     # Display metadata
audiobookctl edit <file>     # Edit in $EDITOR (dry-run)
audiobookctl organize <dir>  # Organize files (phase 3)
```

## Key Dependencies

- `mp4ameta` - M4B/M4A tag reading/writing
- `clap` - CLI argument parsing
- `serde` + `toml` - Metadata serialization

## Development Notes

- Format: m4b only (no mp3/flac support planned)
- Edit workflow saves pending changes to `~/.cache/audiobookctl/pending/`
- Re-run with `--no-dry-run` to apply pending edits
- Side-by-side diffs shown before applying changes

## Issue Tracking

This project uses beads for issue tracking. See `.beads/` directory.
