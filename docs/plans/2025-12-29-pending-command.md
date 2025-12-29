# Pending Command Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `audiobookctl pending` command for bulk viewing and applying pending edits.

**Architecture:** New top-level command with subcommands (list/show/apply/clear) following the existing `backups` command pattern. Add `list_all()` method to `PendingEditsCache` to enumerate pending edits.

**Tech Stack:** Rust, clap (subcommands), chrono (date formatting)

---

### Task 1: Add `list_all()` to PendingEditsCache

**Files:**
- Modify: `src/safety/pending.rs:133-150`

**Step 1: Write the `list_all` method**

Add after the `clear_all` method in `PendingEditsCache`:

```rust
/// List all pending edits
pub fn list_all(&self) -> Result<Vec<PendingEdit>> {
    let mut edits = Vec::new();

    if !self.cache_dir.exists() {
        return Ok(edits);
    }

    for entry in fs::read_dir(&self.cache_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "toml") {
            // Read the file to get the original path from header
            let content = fs::read_to_string(&path)?;

            let mut original_path = PathBuf::new();
            let mut created_at = chrono::Utc::now();
            let mut toml_start = 0;

            for (i, line) in content.lines().enumerate() {
                if line.starts_with("# Pending edit for: ") {
                    let path_str = line.trim_start_matches("# Pending edit for: ");
                    original_path = PathBuf::from(path_str);
                } else if line.starts_with("# Created: ") {
                    let ts_str = line.trim_start_matches("# Created: ");
                    if let Ok(ts) = chrono::DateTime::parse_from_rfc3339(ts_str) {
                        created_at = ts.with_timezone(&chrono::Utc);
                    }
                } else if !line.starts_with('#') && !line.is_empty() {
                    toml_start = content.lines().take(i).map(|l| l.len() + 1).sum();
                    break;
                }
            }

            if !original_path.as_os_str().is_empty() {
                let toml_content = content[toml_start..].to_string();
                edits.push(PendingEdit {
                    original_path,
                    toml_content,
                    created_at,
                });
            }
        }
    }

    // Sort by created_at (oldest first)
    edits.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    Ok(edits)
}
```

**Step 2: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 3: Commit**

```bash
git add src/safety/pending.rs
git commit -m "feat(pending): add list_all method to PendingEditsCache"
```

---

### Task 2: Add CLI definitions for pending command

**Files:**
- Modify: `src/cli.rs`

**Step 1: Add PendingAction enum after BackupsAction**

Add after line 188 (after `BackupsAction` enum):

```rust
#[derive(Subcommand)]
pub enum PendingAction {
    /// List all pending edits
    List {
        /// Show diff preview for each pending edit
        #[arg(long)]
        diff: bool,
    },
    /// Show diff for a specific pending edit
    Show {
        /// Path to the m4b file
        file: PathBuf,
    },
    /// Apply pending edits
    Apply {
        /// Path to specific m4b file (applies all if not specified)
        file: Option<PathBuf>,

        /// Skip confirmation prompt
        #[arg(long)]
        yes: bool,

        /// Skip creating backup files
        #[arg(long = "no-backup-i-void-my-warranty")]
        no_backup: bool,
    },
    /// Clear pending edits
    Clear {
        /// Path to specific m4b file (clears all if not specified)
        file: Option<PathBuf>,
    },
}
```

**Step 2: Add Pending variant to Commands enum**

Add after `Backups` variant (around line 163):

```rust
    /// Manage pending edits
    Pending {
        #[command(subcommand)]
        action: PendingAction,
    },
```

**Step 3: Remove `--clear` from Edit command**

Remove these lines from the `Edit` variant (lines 54-57):

```rust
        /// Clear pending edit(s)
        #[arg(long)]
        clear: bool,
```

**Step 4: Run `cargo check`**

Run: `cargo check`
Expected: Errors about missing `clear` in main.rs and edit.rs (expected, will fix next)

**Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat(cli): add Pending command and PendingAction enum"
```

---

### Task 3: Create pending command module

**Files:**
- Create: `src/commands/pending.rs`

**Step 1: Create the pending.rs file**

```rust
//! Pending command - manage pending edits

use crate::editor::{compute_changes, format_diff, toml_to_metadata};
use crate::metadata::read_metadata;
use crate::safety::{create_backup, PendingEditsCache};
use crate::metadata::write_metadata;
use anyhow::{bail, Context, Result};
use std::io::{self, Write};
use std::path::Path;

/// List all pending edits
pub fn list(show_diff: bool) -> Result<()> {
    let cache = PendingEditsCache::new()?;
    let edits = cache.list_all()?;

    if edits.is_empty() {
        println!("No pending edits.");
        return Ok(());
    }

    println!("Pending edits:");

    for edit in &edits {
        let timestamp = edit.created_at.format("%Y-%m-%d %H:%M");
        println!(
            "  {} (edit saved {})",
            edit.original_path.display(),
            timestamp
        );

        if show_diff {
            // Show diff if file still exists
            if edit.original_path.exists() {
                match show_diff_for_edit(&edit.original_path, &edit.toml_content) {
                    Ok(diff) => {
                        for line in diff.lines() {
                            println!("    {}", line);
                        }
                    }
                    Err(e) => {
                        println!("    (error reading file: {})", e);
                    }
                }
            } else {
                println!("    (file no longer exists)");
            }
            println!();
        }
    }

    println!();
    println!("{} pending edit(s)", edits.len());

    Ok(())
}

/// Show diff for a specific pending edit
pub fn show(file: &Path) -> Result<()> {
    let cache = PendingEditsCache::new()?;

    let pending = cache.load(file)?;
    match pending {
        Some(edit) => {
            let diff = show_diff_for_edit(&edit.original_path, &edit.toml_content)?;
            println!("{}", diff);
        }
        None => {
            bail!("No pending edit found for: {}", file.display());
        }
    }

    Ok(())
}

/// Apply pending edits
pub fn apply(file: Option<&Path>, yes: bool, no_backup: bool) -> Result<()> {
    let cache = PendingEditsCache::new()?;

    if let Some(file) = file {
        // Apply single file
        apply_single(&cache, file, yes, no_backup)?;
    } else {
        // Apply all
        apply_all(&cache, yes, no_backup)?;
    }

    Ok(())
}

/// Clear pending edits
pub fn clear(file: Option<&Path>) -> Result<()> {
    let cache = PendingEditsCache::new()?;

    if let Some(file) = file {
        if cache.clear(file)? {
            println!("Cleared pending edit for: {}", file.display());
        } else {
            println!("No pending edit found for: {}", file.display());
        }
    } else {
        let count = cache.clear_all()?;
        println!("Cleared {} pending edit(s).", count);
    }

    Ok(())
}

fn show_diff_for_edit(file: &Path, toml_content: &str) -> Result<String> {
    let original_metadata = read_metadata(file)?;
    let new_metadata = toml_to_metadata(toml_content)?;
    let changes = compute_changes(&original_metadata, &new_metadata);
    Ok(format_diff(&file.display().to_string(), &changes))
}

fn apply_single(cache: &PendingEditsCache, file: &Path, yes: bool, no_backup: bool) -> Result<()> {
    let pending = cache.load(file)?;
    let edit = match pending {
        Some(e) => e,
        None => bail!("No pending edit found for: {}", file.display()),
    };

    // Show diff
    let diff = show_diff_for_edit(file, &edit.toml_content)?;
    println!("{}", diff);

    // Confirm
    if !yes {
        print!("Apply changes to {}? [y/N] ", file.display());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") && !input.trim().eq_ignore_ascii_case("yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    // Apply
    let new_metadata = toml_to_metadata(&edit.toml_content)?;

    if !no_backup {
        let backup_path = create_backup(file)?;
        println!("Created backup: {}", backup_path.display());
    }

    write_metadata(file, &new_metadata)?;
    cache.clear(file)?;
    println!("Applied changes to: {}", file.display());

    Ok(())
}

fn apply_all(cache: &PendingEditsCache, yes: bool, no_backup: bool) -> Result<()> {
    let edits = cache.list_all()?;

    if edits.is_empty() {
        println!("No pending edits to apply.");
        return Ok(());
    }

    // Show summary
    println!("Pending edits to apply:");
    for edit in &edits {
        let status = if edit.original_path.exists() {
            ""
        } else {
            " (file missing)"
        };
        println!("  {}{}", edit.original_path.display(), status);
    }
    println!();

    // Confirm
    if !yes {
        print!("Apply {} pending edit(s)? [y/N] ", edits.len());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") && !input.trim().eq_ignore_ascii_case("yes") {
            println!("Aborted.");
            return Ok(());
        }
    }

    println!();
    println!("Applying {} pending edit(s)...", edits.len());

    let mut applied = 0;
    let mut failed = 0;

    for edit in &edits {
        let result = apply_edit(cache, &edit.original_path, &edit.toml_content, no_backup);
        match result {
            Ok(()) => {
                println!("  \u{2713} {}", edit.original_path.display());
                applied += 1;
            }
            Err(e) => {
                println!("  \u{2717} {} ({})", edit.original_path.display(), e);
                failed += 1;
            }
        }
    }

    println!();
    println!("Applied: {}, Failed: {}", applied, failed);

    Ok(())
}

fn apply_edit(
    cache: &PendingEditsCache,
    file: &Path,
    toml_content: &str,
    no_backup: bool,
) -> Result<()> {
    if !file.exists() {
        bail!("file not found");
    }

    let new_metadata = toml_to_metadata(toml_content).context("invalid TOML")?;

    if !no_backup {
        create_backup(file)?;
    }

    write_metadata(file, &new_metadata)?;
    cache.clear(file)?;

    Ok(())
}
```

**Step 2: Run `cargo check`**

Run: `cargo check`
Expected: Errors about pending module not declared (will fix next)

**Step 3: Commit**

```bash
git add src/commands/pending.rs
git commit -m "feat(pending): implement pending command handlers"
```

---

### Task 4: Wire up pending command in mod.rs and main.rs

**Files:**
- Modify: `src/commands/mod.rs`
- Modify: `src/main.rs`

**Step 1: Add pending to commands/mod.rs**

Add `pub mod pending;` to `src/commands/mod.rs`:

```rust
pub mod backups;
pub mod edit;
pub mod fields;
pub mod fix;
pub mod init;
pub mod lookup;
pub mod lookup_all;
pub mod organize;
pub mod pending;
pub mod show;
```

**Step 2: Add Pending match arm to main.rs**

Add after the `Backups` match arm (around line 97):

```rust
        Commands::Pending { action } => {
            use cli::PendingAction;
            match action {
                PendingAction::List { diff } => {
                    commands::pending::list(diff)?;
                }
                PendingAction::Show { file } => {
                    commands::pending::show(&file)?;
                }
                PendingAction::Apply { file, yes, no_backup } => {
                    commands::pending::apply(file.as_deref(), yes, no_backup)?;
                }
                PendingAction::Clear { file } => {
                    commands::pending::clear(file.as_deref())?;
                }
            }
        }
```

**Step 3: Run `cargo check`**

Run: `cargo check`
Expected: Errors about `clear` parameter in Edit (will fix next)

**Step 4: Commit**

```bash
git add src/commands/mod.rs src/main.rs
git commit -m "feat(pending): wire up pending command in main"
```

---

### Task 5: Remove --clear from edit command

**Files:**
- Modify: `src/main.rs`
- Modify: `src/commands/edit.rs`

**Step 1: Remove `clear` from Edit match in main.rs**

Update the Edit match arm to remove `clear`:

```rust
        Commands::Edit {
            file,
            no_dry_run,
            yes,
            no_backup,
            commit,
            commit_all,
        } => {
            commands::edit::run(
                file.as_deref(),
                no_dry_run,
                yes,
                no_backup,
                commit,
                commit_all,
            )?;
        }
```

**Step 2: Update edit::run signature**

Remove `clear` parameter from `run()` function in `src/commands/edit.rs`:

```rust
pub fn run(
    file: Option<&Path>,
    no_dry_run: bool,
    yes: bool,
    no_backup: bool,
    commit: bool,
    commit_all: bool,
) -> Result<()> {
```

**Step 3: Remove handle_clear call and function**

Remove lines 21-24 (the `if clear` block):

```rust
    // Handle --clear (no file needed for --clear without file)
    if clear {
        return handle_clear(file);
    }
```

Remove the `handle_clear` function entirely (lines 160-175).

**Step 4: Run `cargo check`**

Run: `cargo check`
Expected: Compiles successfully

**Step 5: Run `cargo test`**

Run: `cargo test`
Expected: All tests pass

**Step 6: Commit**

```bash
git add src/main.rs src/commands/edit.rs
git commit -m "refactor(edit): remove --clear flag, now in pending command"
```

---

### Task 6: Manual testing

**Step 1: Build release**

Run: `cargo build --release`

**Step 2: Test `pending list` with no pending edits**

Run: `./target/release/audiobookctl pending list`
Expected: "No pending edits."

**Step 3: Create a pending edit (if you have a test .m4b file)**

Run: `./target/release/audiobookctl edit <file.m4b>` (make a change and save)
Expected: "Changes saved to pending cache."

**Step 4: Test `pending list`**

Run: `./target/release/audiobookctl pending list`
Expected: Shows the pending edit with timestamp

**Step 5: Test `pending list --diff`**

Run: `./target/release/audiobookctl pending list --diff`
Expected: Shows pending edit with diff preview

**Step 6: Test `pending show <file>`**

Run: `./target/release/audiobookctl pending show <file.m4b>`
Expected: Shows diff for that file

**Step 7: Test `pending clear`**

Run: `./target/release/audiobookctl pending clear`
Expected: Clears all pending edits

---

### Task 7: Final commit and cleanup

**Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass

**Step 2: Run clippy**

Run: `cargo clippy`
Expected: No warnings

**Step 3: Run format check**

Run: `cargo fmt --check`
Expected: No formatting issues (run `cargo fmt` if needed)
