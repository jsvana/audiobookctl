# Optional Placeholders Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `{field?}` syntax for optional placeholders that collapse (skip directory level) when missing.

**Architecture:** Extend the `Segment::Placeholder` enum with an `optional: bool` field. Update parsing to detect `?` suffix. Update `generate_path()` to skip optional placeholders when missing instead of returning an error.

**Tech Stack:** Rust, existing `FormatTemplate` in `src/organize/format.rs`

**Issue:** audiobookctl-tk4

---

## Task 1: Add Optional Field to Placeholder Segment

**Files:**
- Modify: `src/organize/format.rs:29-36`
- Test: `src/organize/format.rs` (existing tests should still pass)

**Step 1: Add optional field to Segment enum**

In `src/organize/format.rs`, update the `Segment` enum:

```rust
#[derive(Debug, Clone)]
enum Segment {
    Literal(String),
    Placeholder {
        name: String,
        padding: Option<usize>,
        optional: bool,
    },
}
```

**Step 2: Update existing Placeholder construction to set optional: false**

In `FormatTemplate::parse()` around line 88, update:

```rust
segments.push(Segment::Placeholder { name, padding, optional: false });
```

**Step 3: Run tests to verify nothing broke**

Run: `cargo test`
Expected: All 51 tests pass

**Step 4: Commit**

```bash
git add src/organize/format.rs
git commit -m "refactor: add optional field to Placeholder segment"
```

---

## Task 2: Parse Optional Syntax

**Files:**
- Modify: `src/organize/format.rs:69-76` (parsing logic)
- Test: `src/organize/format.rs` (add new test)

**Step 1: Write the failing test**

Add to the `#[cfg(test)]` module in `src/organize/format.rs`:

```rust
#[test]
fn test_parse_optional_placeholder() {
    let template = FormatTemplate::parse("{author}/{series?}/{title}/{filename}").unwrap();
    let metadata = AudiobookMetadata {
        title: Some("Book".to_string()),
        author: Some("Author".to_string()),
        series: None,
        ..Default::default()
    };
    // For now just verify it parses - we'll test collapsing in next task
    assert!(template.generate_path(&metadata, "book.m4b").is_err());
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_parse_optional_placeholder`
Expected: FAIL - `?` is included in placeholder name and fails validation

**Step 3: Update parsing to detect and strip `?` suffix**

In `FormatTemplate::parse()`, update the padding/name parsing section (around line 69-76):

```rust
// Parse optional padding (e.g., "series_position:02") and optional marker (?)
let (name, padding, optional) = {
    let mut work = placeholder.clone();

    // Check for optional marker at end
    let optional = work.ends_with('?');
    if optional {
        work.pop();
    }

    // Check for padding
    if let Some(colon_pos) = work.find(':') {
        let name = work[..colon_pos].to_string();
        let pad_str = &work[colon_pos + 1..];
        let padding = pad_str.parse::<usize>().ok();
        (name, padding, optional)
    } else {
        (work, None, optional)
    }
};
```

Then update line 88 to use the new `optional` variable:

```rust
segments.push(Segment::Placeholder { name, padding, optional });
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_parse_optional_placeholder`
Expected: PASS (test expects error for now since collapsing not implemented)

**Step 5: Commit**

```bash
git add src/organize/format.rs
git commit -m "feat: parse optional placeholder syntax {field?}"
```

---

## Task 3: Implement Path Collapsing for Optional Placeholders

**Files:**
- Modify: `src/organize/format.rs:131-154` (generate_path logic)
- Test: `src/organize/format.rs` (update test)

**Step 1: Update test to expect collapsing**

Update `test_parse_optional_placeholder` to expect success with collapsed path:

```rust
#[test]
fn test_optional_placeholder_collapses() {
    let template = FormatTemplate::parse("{author}/{series?}/{title}/{filename}").unwrap();
    let metadata = AudiobookMetadata {
        title: Some("Book".to_string()),
        author: Some("Author".to_string()),
        series: None,
        ..Default::default()
    };
    let path = template.generate_path(&metadata, "book.m4b").unwrap();
    assert_eq!(path, PathBuf::from("Author/Book/book.m4b"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_optional_placeholder_collapses`
Expected: FAIL - returns Err with missing "series"

**Step 3: Update generate_path to handle optional placeholders**

In `generate_path()`, update the `Segment::Placeholder` match arm (around line 131-154):

```rust
Segment::Placeholder { name, padding, optional } => {
    let value = self.get_field_value(metadata, name, original_filename);
    match value {
        Some(v) => {
            let formatted = if let Some(pad) = padding {
                format!("{:0>width$}", v, width = *pad)
            } else {
                v
            };
            // Sanitize for filesystem
            let sanitized = sanitize_path_component(&formatted);
            current_part.push_str(&sanitized);
        }
        None if *optional => {
            // Optional placeholder missing - mark current part as empty
            // so it gets filtered out
            // Don't add to missing list
        }
        None => {
            if name != "filename" {
                missing.push(name.clone());
            }
            // Use placeholder text for now (will fail later if missing)
            current_part.push_str(&format!("{{{}}}", name));
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_optional_placeholder_collapses`
Expected: PASS

**Step 5: Commit**

```bash
git add src/organize/format.rs
git commit -m "feat: collapse path when optional placeholder is missing"
```

---

## Task 4: Test Optional Placeholder Present

**Files:**
- Test: `src/organize/format.rs`

**Step 1: Write test for optional placeholder when present**

Add test:

```rust
#[test]
fn test_optional_placeholder_present() {
    let template = FormatTemplate::parse("{author}/{series?}/{title}/{filename}").unwrap();
    let metadata = AudiobookMetadata {
        title: Some("Book".to_string()),
        author: Some("Author".to_string()),
        series: Some("Series".to_string()),
        ..Default::default()
    };
    let path = template.generate_path(&metadata, "book.m4b").unwrap();
    assert_eq!(path, PathBuf::from("Author/Series/Book/book.m4b"));
}
```

**Step 2: Run test**

Run: `cargo test test_optional_placeholder_present`
Expected: PASS (should work with existing implementation)

**Step 3: Commit**

```bash
git add src/organize/format.rs
git commit -m "test: verify optional placeholder works when present"
```

---

## Task 5: Test Optional with Padding

**Files:**
- Test: `src/organize/format.rs`

**Step 1: Write test for optional with padding**

Add test:

```rust
#[test]
fn test_optional_placeholder_with_padding() {
    let template = FormatTemplate::parse("{author}/{series?}/{series_position?:02}/{title}/{filename}").unwrap();

    // With both present
    let metadata_full = AudiobookMetadata {
        title: Some("Book".to_string()),
        author: Some("Author".to_string()),
        series: Some("Series".to_string()),
        series_position: Some(3),
        ..Default::default()
    };
    let path = template.generate_path(&metadata_full, "book.m4b").unwrap();
    assert_eq!(path, PathBuf::from("Author/Series/03/Book/book.m4b"));

    // With both missing
    let metadata_none = AudiobookMetadata {
        title: Some("Book".to_string()),
        author: Some("Author".to_string()),
        series: None,
        series_position: None,
        ..Default::default()
    };
    let path = template.generate_path(&metadata_none, "book.m4b").unwrap();
    assert_eq!(path, PathBuf::from("Author/Book/book.m4b"));
}
```

**Step 2: Run test**

Run: `cargo test test_optional_placeholder_with_padding`
Expected: PASS

**Step 3: Commit**

```bash
git add src/organize/format.rs
git commit -m "test: verify optional placeholder with padding syntax"
```

---

## Task 6: Test Required Still Fails

**Files:**
- Test: `src/organize/format.rs`

**Step 1: Write test to verify required placeholders still fail**

Add test:

```rust
#[test]
fn test_required_placeholder_still_fails() {
    let template = FormatTemplate::parse("{author}/{series?}/{title}/{filename}").unwrap();
    let metadata = AudiobookMetadata {
        title: Some("Book".to_string()),
        author: None,  // Required field missing
        series: None,
        ..Default::default()
    };
    let result = template.generate_path(&metadata, "book.m4b");
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), vec!["author"]);
}
```

**Step 2: Run test**

Run: `cargo test test_required_placeholder_still_fails`
Expected: PASS

**Step 3: Commit**

```bash
git add src/organize/format.rs
git commit -m "test: verify required placeholders still fail when missing"
```

---

## Task 7: Update Documentation

**Files:**
- Modify: `src/organize/format.rs:5-21` (PLACEHOLDERS docs)

**Step 1: Update PLACEHOLDERS docstring**

Update the module documentation near the top:

```rust
/// Available format placeholders with descriptions.
///
/// Use `{field}` for required placeholders (path generation fails if missing).
/// Use `{field?}` for optional placeholders (path collapses if missing).
///
/// Example: `{author}/{series?}/{title}/{filename}`
/// - With series: `Author/Series/Title/file.m4b`
/// - Without series: `Author/Title/file.m4b`
pub const PLACEHOLDERS: &[(&str, &str)] = &[
```

**Step 2: Run all tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Commit**

```bash
git add src/organize/format.rs
git commit -m "docs: document optional placeholder syntax"
```

---

## Task 8: Run Full Test Suite and Clippy

**Files:** None (verification only)

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No new warnings

**Step 3: Run fmt check**

Run: `cargo fmt --check`
Expected: No formatting issues (or run `cargo fmt` to fix)

---

## Task 9: Final Commit and Update Issue

**Step 1: Close the beads issue**

Run: `bd close audiobookctl-tk4`

**Step 2: Report completion**

Feature complete and ready for merge.
