use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::format::FormatTemplate;
use super::scanner::ScannedFile;

/// A planned file operation (copy or move)
#[derive(Debug, Clone)]
pub struct PlannedOperation {
    pub source: PathBuf,
    pub dest: PathBuf,
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
    /// Files that couldn't be organized (missing metadata)
    pub uncategorized: Vec<UncategorizedFile>,
    /// Detected conflicts
    pub conflicts: Vec<Conflict>,
}

impl OrganizePlan {
    /// Build a plan for organizing files
    pub fn build(files: &[ScannedFile], template: &FormatTemplate, dest_dir: &Path) -> Self {
        let mut operations = Vec::new();
        let mut uncategorized = Vec::new();
        let mut dest_to_sources: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        for file in files {
            match template.generate_path(&file.metadata, &file.filename) {
                Ok(relative_path) => {
                    let dest = dest_dir.join(relative_path);
                    operations.push(PlannedOperation {
                        source: file.path.clone(),
                        dest: dest.clone(),
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

        // Detect conflicts
        let mut conflicts = Vec::new();

        for (dest, sources) in dest_to_sources {
            let exists_on_disk = dest.exists();

            if sources.len() > 1 || exists_on_disk {
                conflicts.push(Conflict {
                    dest,
                    sources,
                    exists_on_disk,
                });
            }
        }

        // Sort for consistent output
        operations.sort_by(|a, b| a.source.cmp(&b.source));
        uncategorized.sort_by(|a, b| a.source.cmp(&b.source));
        conflicts.sort_by(|a, b| a.dest.cmp(&b.dest));

        Self {
            operations,
            uncategorized,
            conflicts,
        }
    }

    /// Check if the plan has any issues that would prevent execution
    pub fn has_issues(&self, allow_uncategorized: bool) -> bool {
        !self.conflicts.is_empty() || (!allow_uncategorized && !self.uncategorized.is_empty())
    }

    /// Get count of files that will be processed
    pub fn operation_count(&self) -> usize {
        self.operations.len()
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
                        needs_fix.push(PlannedOperation {
                            source: file.path.clone(),
                            dest: expected_dest.clone(),
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

    /// Check if the plan has any issues that would prevent execution
    pub fn has_issues(&self) -> bool {
        !self.conflicts.is_empty()
    }

    /// Get count of files that need fixing
    pub fn fix_count(&self) -> usize {
        self.needs_fix.len()
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
        }];

        let template = FormatTemplate::parse("{author}/{title}/{filename}").unwrap();
        let plan = OrganizePlan::build(&files, &template, Path::new("/dest"));

        assert!(plan.operations.is_empty());
        assert_eq!(plan.uncategorized.len(), 1);
        assert_eq!(plan.uncategorized[0].missing_fields, vec!["author"]);
    }
}
