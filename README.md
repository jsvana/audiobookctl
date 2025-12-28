# audiobookctl

A command-line tool for reading, editing, and organizing m4b audiobook metadata.

## Features

- **Show metadata** - Display audiobook metadata in pretty, JSON, or single-field format
- **Edit metadata** - Edit tags in your `$EDITOR` with TOML format and side-by-side diff preview
- **Safety-first** - Dry-run by default, backups before modifications, pending edits cache

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
```

## Usage

### Show metadata

```bash
# Pretty print all metadata
audiobookctl show book.m4b

# Output as JSON
audiobookctl show book.m4b --json

# Get a specific field
audiobookctl show book.m4b --field title
```

### Edit metadata

```bash
# Open in $EDITOR, preview changes (dry-run)
audiobookctl edit book.m4b

# Apply pending changes
audiobookctl edit book.m4b --no-dry-run

# Skip confirmation prompt
audiobookctl edit book.m4b --no-dry-run --yes

# Clear pending edits
audiobookctl edit --clear
audiobookctl edit book.m4b --clear

# Delete backup after verifying changes
audiobookctl edit book.m4b --commit

# Delete all backups in current directory
audiobookctl edit --commit-all
```

## Safety Model

**Data safety is paramount.** Audiobook files are irreplaceable user data.

- All modifying operations are **dry-run by default**
- Use `--no-dry-run` to actually apply changes
- Backups (`.bak` files) are created before any modification
- Pending edits are saved to `~/.cache/audiobookctl/pending/` so you can review before applying
- Use `--no-backup-i-void-my-warranty` to skip backups (not recommended)

## Supported Metadata Fields

| Field | Editable | Description |
|-------|----------|-------------|
| title | Yes | Book title |
| author | Yes | Author name |
| narrator | Yes | Narrator name |
| series | Yes | Series name |
| series_position | Yes | Position in series |
| year | Yes | Publication year |
| description | Yes | Book description |
| publisher | Yes | Publisher name |
| genre | Yes | Genre |
| isbn | Yes | ISBN |
| asin | Yes | Amazon ASIN |
| duration | No | Total duration (read-only) |
| chapters | No | Chapter count (read-only) |
| cover | No | Cover image info (read-only) |

## TOML Edit Format

When editing, metadata is presented as TOML:

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
# publisher = ""
genre = "Science Fiction"
# isbn = ""
# asin = ""

# Read-only (cannot be edited)
# duration = "16:10:35"
# chapters = 32
# cover = "embedded (1400x1400 JPEG)"
```

## License

MIT License - see [LICENSE](LICENSE) for details.
