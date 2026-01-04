use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::format::FormatTemplate;
use super::scanner::ScannedFile;
use crate::hash::get_hash;

/// A planned auxiliary file operation
#[derive(Debug, Clone)]
pub struct AuxiliaryOperation {
    pub source: PathBuf,
    pub dest: PathBuf,
}

/// A planned file operation (copy or move)
#[derive(Debug, Clone)]
pub struct PlannedOperation {
    pub source: PathBuf,
    pub dest: PathBuf,
    /// Auxiliary files to copy/move with this m4b
    pub auxiliary: Vec<AuxiliaryOperation>,
}

/// A file that already exists at destination with matching content
#[derive(Debug, Clone)]
#[allow(dead_code)] // Will be used by organize command display in upcoming tasks
pub struct AlreadyPresent {
    pub source: PathBuf,
    pub dest: PathBuf,
    pub hash: String,
}

/// A file that couldn't be organized due to missing metadata
#[derive(Debug, Clone)]
pub struct UncategorizedFile {
    pub source: PathBuf,
    pub missing_fields: Vec<String>,
}

/// A destination conflict (multiple sources mapping to same dest)
#[derive(Debug, Clone)]
pub struct Conflict {
    pub dest: PathBuf,
    pub sources: Vec<PathBuf>,
    /// True if the conflict is with an existing file on disk
    pub exists_on_disk: bool,
}

/// Result of planning an organize operation
#[derive(Debug)]
pub struct OrganizePlan {
    /// Operations to perform
    pub operations: Vec<PlannedOperation>,
    /// Files already present at destination (hash match)
    #[allow(dead_code)] // Will be used by organize command display in upcoming tasks
    pub already_present: Vec<AlreadyPresent>,
    /// Files that couldn't be organized (missing metadata)
    pub uncategorized: Vec<UncategorizedFile>,
    /// Detected conflicts
    pub conflicts: Vec<Conflict>,
}

/// Progress event during plan building
#[derive(Debug, Clone)]
pub struct PlanProgress<'a> {
    /// Current file being processed (1-indexed)
    pub current: usize,
    /// Total files to process
    pub total: usize,
    /// The file path being hashed
    pub path: &'a Path,
    /// Whether this is the source or destination file
    pub is_source: bool,
}

impl OrganizePlan {
    /// Build a plan for organizing files
    #[allow(dead_code)]
    pub fn build(files: &[ScannedFile], template: &FormatTemplate, dest_dir: &Path) -> Self {
        Self::build_with_progress(files, template, dest_dir, |_| {})
    }

    /// Build a plan for organizing files with progress callback
    pub fn build_with_progress<F>(
        files: &[ScannedFile],
        template: &FormatTemplate,
        dest_dir: &Path,
        mut on_progress: F,
    ) -> Self
    where
        F: FnMut(PlanProgress),
    {
        let mut operations = Vec::new();
        let mut uncategorized = Vec::new();
        let mut dest_to_sources: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        for file in files {
            match template.generate_path(&file.metadata, &file.filename) {
                Ok(relative_path) => {
                    let dest = dest_dir.join(&relative_path);
                    let dest_parent = dest.parent().unwrap_or(dest_dir);

                    // Build auxiliary operations preserving relative structure
                    let auxiliary: Vec<AuxiliaryOperation> = file
                        .auxiliary_files
                        .iter()
                        .map(|aux| AuxiliaryOperation {
                            source: aux.path.clone(),
                            dest: dest_parent.join(&aux.relative_path),
                        })
                        .collect();

                    operations.push(PlannedOperation {
                        source: file.path.clone(),
                        dest: dest.clone(),
                        auxiliary,
                    });
                    dest_to_sources
                        .entry(dest)
                        .or_default()
                        .push(file.path.clone());
                }
                Err(missing) => {
                    uncategorized.push(UncategorizedFile {
                        source: file.path.clone(),
                        missing_fields: missing,
                    });
                }
            }
        }

        // Detect conflicts and already-present files
        let mut conflicts = Vec::new();
        let mut already_present = Vec::new();
        let mut ops_to_remove = Vec::new();

        // Count how many files need hash comparison (dest exists, single source)
        let need_comparison: Vec<_> = dest_to_sources
            .iter()
            .filter(|(dest, sources)| sources.len() == 1 && dest.exists())
            .collect();
        let total_comparisons = need_comparison.len();
        let mut current_comparison = 0;

        for (dest, sources) in &dest_to_sources {
            let exists_on_disk = dest.exists();

            if sources.len() > 1 {
                // Multiple sources mapping to same dest - always a conflict
                conflicts.push(Conflict {
                    dest: dest.clone(),
                    sources: sources.clone(),
                    exists_on_disk,
                });
            } else if exists_on_disk {
                // Single source but dest exists - check hash
                current_comparison += 1;
                let source = &sources[0];
                on_progress(PlanProgress {
                    current: current_comparison,
                    total: total_comparisons,
                    path: source,
                    is_source: true,
                });
                // Use cached hash if available, write cache if computed
                let src_hash_result = get_hash(source, true);
                on_progress(PlanProgress {
                    current: current_comparison,
                    total: total_comparisons,
                    path: dest,
                    is_source: false,
                });
                // Use cached hash if available, write cache if computed
                let dest_hash_result = get_hash(dest, true);

                match (src_hash_result, dest_hash_result) {
                    (Ok(src_hash), Ok(dest_hash)) if src_hash == dest_hash => {
                        // Same content - mark as already present
                        already_present.push(AlreadyPresent {
                            source: source.clone(),
                            dest: dest.clone(),
                            hash: src_hash,
                        });
                        ops_to_remove.push(source.clone());
                    }
                    (Ok(_), Ok(_)) => {
                        // Different content - conflict
                        conflicts.push(Conflict {
                            dest: dest.clone(),
                            sources: sources.clone(),
                            exists_on_disk: true,
                        });
                    }
                    (Err(_), _) | (_, Err(_)) => {
                        // Hash error - treat as conflict to be safe
                        conflicts.push(Conflict {
                            dest: dest.clone(),
                            sources: sources.clone(),
                            exists_on_disk: true,
                        });
                    }
                }
            }
        }

        // Remove already-present files from operations
        operations.retain(|op| !ops_to_remove.contains(&op.source));

        // Sort for consistent output
        operations.sort_by(|a, b| a.source.cmp(&b.source));
        already_present.sort_by(|a, b| a.source.cmp(&b.source));
        uncategorized.sort_by(|a, b| a.source.cmp(&b.source));
        conflicts.sort_by(|a, b| a.dest.cmp(&b.dest));

        Self {
            operations,
            already_present,
            uncategorized,
            conflicts,
        }
    }
}

/// Result of planning a fix operation (for already-organized files)
#[derive(Debug)]
pub struct FixPlan {
    /// Files that need to be moved
    pub needs_fix: Vec<PlannedOperation>,
    /// Files that are already compliant
    pub compliant: Vec<PathBuf>,
    /// Files that couldn't be checked (missing metadata)
    pub uncategorized: Vec<UncategorizedFile>,
    /// Detected conflicts
    pub conflicts: Vec<Conflict>,
}

impl FixPlan {
    /// Build a plan for fixing non-compliant files in an organized library
    pub fn build(files: &[ScannedFile], template: &FormatTemplate, dest_dir: &Path) -> Self {
        let mut needs_fix = Vec::new();
        let mut compliant = Vec::new();
        let mut uncategorized = Vec::new();
        let mut dest_to_sources: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        for file in files {
            match template.generate_path(&file.metadata, &file.filename) {
                Ok(relative_path) => {
                    let expected_dest = dest_dir.join(relative_path);

                    // Check if file is already at the correct location
                    if file.path == expected_dest {
                        compliant.push(file.path.clone());
                    } else {
                        // Build auxiliary operations preserving relative structure
                        let dest_parent = expected_dest.parent().unwrap_or(dest_dir);
                        let auxiliary: Vec<AuxiliaryOperation> = file
                            .auxiliary_files
                            .iter()
                            .map(|aux| AuxiliaryOperation {
                                source: aux.path.clone(),
                                dest: dest_parent.join(&aux.relative_path),
                            })
                            .collect();

                        needs_fix.push(PlannedOperation {
                            source: file.path.clone(),
                            dest: expected_dest.clone(),
                            auxiliary,
                        });
                        dest_to_sources
                            .entry(expected_dest)
                            .or_default()
                            .push(file.path.clone());
                    }
                }
                Err(missing) => {
                    uncategorized.push(UncategorizedFile {
                        source: file.path.clone(),
                        missing_fields: missing,
                    });
                }
            }
        }

        // Detect conflicts (only for files that need fixing)
        let mut conflicts = Vec::new();

        for (dest, sources) in dest_to_sources {
            // For fix, also check if dest already exists (and isn't one of the sources)
            let exists_on_disk = dest.exists() && !sources.contains(&dest);

            if sources.len() > 1 || exists_on_disk {
                conflicts.push(Conflict {
                    dest,
                    sources,
                    exists_on_disk,
                });
            }
        }

        // Sort for consistent output
        needs_fix.sort_by(|a, b| a.source.cmp(&b.source));
        compliant.sort();
        uncategorized.sort_by(|a, b| a.source.cmp(&b.source));
        conflicts.sort_by(|a, b| a.dest.cmp(&b.dest));

        Self {
            needs_fix,
            compliant,
            uncategorized,
            conflicts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::AudiobookMetadata;

    fn make_scanned_file(path: &str, author: &str, title: &str) -> ScannedFile {
        ScannedFile {
            path: PathBuf::from(path),
            filename: PathBuf::from(path)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            metadata: AudiobookMetadata {
                author: Some(author.to_string()),
                title: Some(title.to_string()),
                ..Default::default()
            },
            auxiliary_files: Vec::new(),
        }
    }

    #[test]
    fn test_build_basic_plan() {
        let files = vec![
            make_scanned_file("/source/book1.m4b", "Author A", "Title 1"),
            make_scanned_file("/source/book2.m4b", "Author B", "Title 2"),
        ];

        let template = FormatTemplate::parse("{author}/{title}/{filename}").unwrap();
        let plan = OrganizePlan::build(&files, &template, Path::new("/dest"));

        assert_eq!(plan.operations.len(), 2);
        assert!(plan.uncategorized.is_empty());
        assert!(plan.conflicts.is_empty());

        assert_eq!(
            plan.operations[0].dest,
            PathBuf::from("/dest/Author A/Title 1/book1.m4b")
        );
    }

    #[test]
    fn test_detect_missing_metadata() {
        let files = vec![ScannedFile {
            path: PathBuf::from("/source/book.m4b"),
            filename: "book.m4b".to_string(),
            metadata: AudiobookMetadata {
                author: None,
                title: Some("Title".to_string()),
                ..Default::default()
            },
            auxiliary_files: Vec::new(),
        }];

        let template = FormatTemplate::parse("{author}/{title}/{filename}").unwrap();
        let plan = OrganizePlan::build(&files, &template, Path::new("/dest"));

        assert!(plan.operations.is_empty());
        assert_eq!(plan.uncategorized.len(), 1);
        assert_eq!(plan.uncategorized[0].missing_fields, vec!["author"]);
    }
}
