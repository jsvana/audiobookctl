# Changelog

All notable changes to this project will be documented in this file.

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
