# Changelog

All notable changes to this project will be documented in this file.

## [0.8.0] - 2026-01-01

### Added
- New `search` command to query APIs without a file
  - `--title` - Search by title
  - `--author` - Search by author
  - `--asin` - Direct ASIN lookup
  - `--json` - Output as JSON
- ASIN extraction from filenames for more accurate lookups
  - Supports patterns: `B0xxx_name.m4b`, `[B0xxx] name.m4b`, `name-B0xxx.m4b`
- Auxiliary file support in organize/fix commands
  - Automatically discovers and copies .cue, .pdf, .jpg, .png files alongside audiobooks
- New `series_title` format placeholder (e.g., "01 - Book Name")

### Fixed
- Colons in titles now become " - " instead of "_" for better readability
  - "Book: Subtitle" → "Book - Subtitle" instead of "Book_ Subtitle"

## [0.7.0] - 2025-12-30

### Added
- `fix` command to scan organized libraries and fix non-compliant paths
- `--show-all` flag to display compliant files in fix output

## [0.6.0] - 2025-12-29

### Added
- New `pending` command to manage pending edits
  - `pending list` - List all pending edits
  - `pending show <file>` - Show pending edit for a file
  - `pending clear [file]` - Clear pending edits
  - `pending apply <file>` - Apply a pending edit
- `list_all` method to PendingEditsCache for retrieving all pending edits

### Changed
- Moved `--clear` flag functionality from `edit` command to `pending clear`

## [0.5.0] - Previous Release

Initial tracked release.
