use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::planner::PlannedOperation;

/// A tree node for displaying the directory structure
#[derive(Debug, Default)]
struct TreeNode {
    children: BTreeMap<String, TreeNode>,
    is_file: bool,
}

impl TreeNode {
    fn insert(&mut self, components: &[&str]) {
        if components.is_empty() {
            return;
        }

        let name = components[0];
        let remaining = &components[1..];

        let child = self.children.entry(name.to_string()).or_default();

        if remaining.is_empty() {
            child.is_file = true;
        } else {
            child.insert(remaining);
        }
    }
}

/// Render a tree view of planned operations
pub fn render_tree(operations: &[PlannedOperation], dest_dir: &Path) -> String {
    let mut root = TreeNode::default();

    // Build tree from operations
    for op in operations {
        // Get path relative to dest_dir
        let relative = op.dest.strip_prefix(dest_dir).unwrap_or(&op.dest);

        let components: Vec<&str> = relative
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();

        root.insert(&components);
    }

    // Render tree
    let mut output = String::new();
    output.push_str(&format!("{}/\n", dest_dir.display()));
    render_node(&root, &mut output, "");

    output
}

fn render_node(node: &TreeNode, output: &mut String, prefix: &str) {
    let count = node.children.len();

    for (i, (name, child)) in node.children.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let child_prefix = if is_last { "    " } else { "│   " };

        if child.is_file {
            output.push_str(&format!("{}{}{}\n", prefix, connector, name));
        } else {
            output.push_str(&format!("{}{}{}/\n", prefix, connector, name));
        }

        render_node(child, output, &format!("{}{}", prefix, child_prefix));
    }
}

/// Render a list view of planned operations (source → dest pairs)
pub fn render_list(operations: &[PlannedOperation]) -> String {
    let mut output = String::new();

    for op in operations {
        output.push_str(&format!(
            "{} → {}\n",
            op.source.display(),
            op.dest.display()
        ));
    }

    output
}

/// Render uncategorized files with their reasons
pub fn render_uncategorized(files: &[(PathBuf, Vec<String>)]) -> String {
    let mut output = String::new();
    output.push_str("__uncategorized__/\n");

    let count = files.len();
    for (i, (path, _)) in files.iter().enumerate() {
        let is_last = i == count - 1;
        let connector = if is_last { "└── " } else { "├── " };

        let filename = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.display().to_string());

        output.push_str(&format!("{}{}\n", connector, filename));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_tree() {
        let operations = vec![
            PlannedOperation {
                source: PathBuf::from("/source/book1.m4b"),
                dest: PathBuf::from("/dest/Author A/Title 1/book1.m4b"),
            },
            PlannedOperation {
                source: PathBuf::from("/source/book2.m4b"),
                dest: PathBuf::from("/dest/Author A/Title 2/book2.m4b"),
            },
            PlannedOperation {
                source: PathBuf::from("/source/book3.m4b"),
                dest: PathBuf::from("/dest/Author B/Title 3/book3.m4b"),
            },
        ];

        let tree = render_tree(&operations, Path::new("/dest"));

        assert!(tree.contains("Author A/"));
        assert!(tree.contains("Author B/"));
        assert!(tree.contains("Title 1/"));
        assert!(tree.contains("book1.m4b"));
    }

    #[test]
    fn test_render_list() {
        let operations = vec![PlannedOperation {
            source: PathBuf::from("/source/book.m4b"),
            dest: PathBuf::from("/dest/Author/Title/book.m4b"),
        }];

        let list = render_list(&operations);
        assert!(list.contains("/source/book.m4b → /dest/Author/Title/book.m4b"));
    }
}
