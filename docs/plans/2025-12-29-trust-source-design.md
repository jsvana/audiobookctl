# Design: `--trust-source` Flag

## Overview

Add a `--trust-source <SOURCE>` flag to `lookup` and `lookup-all` commands that auto-accepts values from a designated source, skipping conflict resolution.

**Valid sources:** `audible`, `audnexus`, `openlibrary`

**Example usage:**
```bash
# Single file - trust Audible, apply immediately
audiobookctl lookup book.m4b --trust-source audible --no-dry-run

# Batch - trust Audible for all files
audiobookctl lookup-all ./library --trust-source audible --no-dry-run
```

## Conflict Resolution Behavior

When `--trust-source audible` is specified:

| Scenario | Behavior |
|----------|----------|
| File and Audible agree | Use agreed value (as normal) |
| File and Audible conflict | **Use Audible's value** |
| Only file has value | Keep file's value |
| Only Audible has value | Use Audible's value |
| Audible returns no results | **Skip file, log warning** |
| Audible API unavailable | **Skip file, log error** |

Other sources (Open Library, Audnexus) are still queried but their values are ignored when they conflict with the trusted source.

**Log output for skipped files:**
```
[3/10] Skipping "book.m4b": trusted source 'audible' returned no results
```

## Implementation Approach

**Files to modify:**

1. **`src/commands/lookup.rs`** - Add `--trust-source` arg to `LookupArgs`, pass to merge logic

2. **`src/commands/lookup_all.rs`** - Add `--trust-source` arg to `LookupAllArgs`, same handling

3. **`src/lookup/merge.rs`** - Core change: new function `merge_with_trusted_source()` that:
   - Takes the trusted source name
   - Returns `MergedResult` with all conflicts auto-resolved to trusted source
   - Returns `None` if trusted source had no results (signals skip)

4. **`src/lookup/api.rs`** - No changes needed (sources already identified by name in `LookupResult.source`)

**New validation:** Reject invalid source names at CLI parse time with clap's `value_parser`.

## Testing Strategy

**Unit tests in `src/lookup/merge.rs`:**
- `test_trusted_source_wins_conflict` - Trusted source value chosen over file
- `test_trusted_source_no_results_returns_none` - Skip signal when no data
- `test_trusted_source_preserves_file_only_values` - File values kept when trusted source lacks them
- `test_invalid_trusted_source_rejected` - Bad source names fail early

**Integration tests:**
- End-to-end with mock API responses (or real calls if acceptable)
- Verify skip logging output format
- Verify `--trust-source` + `--no-dry-run` applies changes

**Edge cases to cover:**
- Trusted source returns partial data (some fields only)
- Multiple files in batch, some with results, some without
- Trusted source is `audnexus` but no ASIN available (will return no results)
