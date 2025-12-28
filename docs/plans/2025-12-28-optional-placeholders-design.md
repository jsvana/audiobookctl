# Optional Placeholders for Format Templates

## Overview

Add support for optional placeholders in format strings that collapse (skip) directory levels when metadata is missing, rather than failing.

## Syntax

- `{field}` - Required placeholder, fails if missing
- `{field?}` - Optional placeholder, collapses if missing
- `{field?:02}` - Optional with padding

**Example:**
```
{author}/{series?}/{title}/{filename}
```

| Metadata | Result |
|----------|--------|
| author=Weir, series=Hail Mary, title=Book | `Weir/Hail Mary/Book/file.m4b` |
| author=Weir, series=None, title=Book | `Weir/Book/file.m4b` (collapsed) |

## Implementation

### Parsing Changes (`src/organize/format.rs`)

Add `optional` field to `Segment::Placeholder`:

```rust
enum Segment {
    Literal(String),
    Placeholder {
        name: String,
        padding: Option<usize>,
        optional: bool,  // true if {field?}
    },
}
```

Update `parse()` to detect `?` suffix after field name (and before/after padding specifier).

### Path Generation Changes

In `generate_path()`, when processing a placeholder:

- **Value present:** Add to path as normal
- **Value missing + optional:** Skip silently, do not add to missing list
- **Value missing + required:** Add to missing list (existing behavior)

Empty path components are filtered out, so skipping an optional placeholder naturally collapses the path.

### Files to Modify

1. `src/organize/format.rs` - Parsing and generation logic
2. No changes needed to `organize.rs`, `fix.rs`, or `config.rs`

### Tests to Add

- Optional placeholder present → normal path
- Optional placeholder missing → collapsed path
- Multiple optional placeholders, some missing
- Optional with padding: `{series_position?:02}`
- Required placeholder still fails when missing
- Backwards compatibility: existing formats work unchanged

## Backwards Compatibility

Existing format strings without `?` continue to work identically - all placeholders remain required by default.
