//! tree command - Visual directory tree view
//!
//! This command displays a tree-like view of directory structure across all branches.

use crate::cache::OperationCache;
use crate::error::{NofsError, Result};
use crate::pool::Pool;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Output from the `tree` command
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct TreeOutput {
    pub path: String,
    pub root: TreeNode,
}

/// A node in the tree
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct TreeNode {
    pub name: String,
    #[serde(skip_serializing_if = "is_false")]
    pub is_file: bool,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<TreeNode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub branches: Vec<String>,
}

/// Check if a boolean is false (for serde `skip_serializing_if`)
// Note: serde requires this signature for skip_serializing_if
#[expect(clippy::trivially_copy_pass_by_ref)]
const fn is_false(b: &bool) -> bool {
    !*b
}

/// Configuration for tree command output
#[non_exhaustive]
#[derive(Clone, Copy)]
pub struct TreeOptions {
    /// Show all branches for each file
    pub all_branches: bool,
    /// Maximum depth to display
    pub max_depth: Option<usize>,
    /// Show directories only
    pub directories_only: bool,
    /// Show files only
    pub files_only: bool,
    /// Show human-readable file sizes
    pub human_size: bool,
    /// Enable verbose output
    pub verbose: bool,
    /// Output in JSON format
    pub json: bool,
}

/// Execute the tree command
///
/// # Errors
///
/// Returns an error if there is an IO error during traversal or output.
pub fn execute(pool: &Pool, path: &str, options: TreeOptions) -> Result<()> {
    let pool_path = Path::new(path);

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    // Find all branches with this path (cached)
    let branches = pool.find_all_branches_cached(pool_path, &cache);

    if branches.is_empty() {
        return Err(NofsError::Command(format!(
            "cannot access '{path}': No such file or directory"
        )));
    }

    // Build tree structure
    let mut tree: BTreeMap<String, TreeEntry> = BTreeMap::new();

    for branch in &branches {
        let branch_path = branch.path.join(pool_path);
        let branch_name = branch.path.to_string_lossy().to_string();

        if !branch_path.exists() {
            continue;
        }

        build_tree_from_dir(&branch_path, &branch_name, &mut tree, options, 0)?;
    }

    // Convert to TreeNode structure
    let root = build_tree_node(".", &tree, options.all_branches);

    if options.json {
        let output = TreeOutput {
            path: path.to_string(),
            root,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let output_options = TreeOutputOptions {
            all_branches: options.all_branches,
            human_size: options.human_size,
        };
        output_tree(&root, "", true, &output_options)?;
    }

    Ok(())
}

/// Tree entry for building the structure
#[derive(Debug)]
struct TreeEntry {
    /// True if this entry represents a file
    is_file: bool,
    /// List of branch paths containing this entry
    branches: Vec<String>,
    /// Size of the file (None for directories)
    size: Option<u64>,
    /// Child entries (for directories)
    children: BTreeMap<String, TreeEntry>,
}

impl TreeEntry {
    /// Create a new `TreeEntry`
    fn new(is_file: bool, branch: String, size: Option<u64>) -> Self {
        Self {
            is_file,
            branches: vec![branch],
            size,
            children: BTreeMap::new(),
        }
    }
}

/// Build tree structure from a directory
fn build_tree_from_dir(
    dir_path: &Path,
    branch_name: &str,
    tree: &mut BTreeMap<String, TreeEntry>,
    options: TreeOptions,
    current_depth: usize,
) -> Result<()> {
    // Check max depth
    if let Some(max) = options.max_depth {
        if current_depth > max {
            return Ok(());
        }
    }

    for entry_result in fs::read_dir(dir_path).map_err(|e| {
        if options.verbose {
            eprintln!("nofs: warning: cannot read '{}': {}", dir_path.display(), e);
        }
        NofsError::Command(format!("cannot read '{}': {}", dir_path.display(), e))
    })? {
        let Ok(entry) = entry_result else {
            continue;
        };

        let entry_path = entry.path();
        let name = entry_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Skip hidden files unless verbose
        if name.starts_with('.') && !options.verbose {
            continue;
        }

        let Ok(metadata) = entry_path.metadata() else {
            continue;
        };

        let is_file = metadata.is_file();

        // Filter by type
        if options.directories_only && is_file {
            continue;
        }
        if options.files_only && !is_file {
            continue;
        }

        let size = is_file.then_some(metadata.len());

        if is_file {
            // Add file entry
            if let Some(existing) = tree.get_mut(&name) {
                if options.all_branches {
                    let branch_str = branch_name.to_string();
                    if !existing.branches.contains(&branch_str) {
                        existing.branches.push(branch_str);
                    }
                }
                // Keep first found size
            } else {
                tree.insert(name.clone(), TreeEntry::new(true, branch_name.to_string(), size));
            }
        } else {
            // Directory entry
            if let Some(existing) = tree.get_mut(&name) {
                if options.all_branches {
                    let branch_str = branch_name.to_string();
                    if !existing.branches.contains(&branch_str) {
                        existing.branches.push(branch_str);
                    }
                }
                // Recurse into subdirectory
                build_tree_from_dir(
                    &entry_path,
                    branch_name,
                    &mut existing.children,
                    options,
                    current_depth.saturating_add(1),
                )?;
            } else {
                let mut new_entry = TreeEntry::new(false, branch_name.to_string(), None);
                build_tree_from_dir(
                    &entry_path,
                    branch_name,
                    &mut new_entry.children,
                    options,
                    current_depth.saturating_add(1),
                )?;
                tree.insert(name.clone(), new_entry);
            }
        }
    }

    Ok(())
}

/// Build `TreeNode` from `TreeEntry` map
fn build_tree_node(name: &str, entries: &BTreeMap<String, TreeEntry>, all_branches: bool) -> TreeNode {
    let mut children = Vec::new();

    for (child_name, entry) in entries {
        let child_node = TreeNode {
            name: child_name.clone(),
            is_file: entry.is_file,
            children: build_tree_node_map(&entry.children, all_branches),
            size: entry.size,
            branches: if all_branches { entry.branches.clone() } else { vec![] },
        };
        children.push(child_node);
    }

    // For root, we need to handle it specially
    if name.is_empty() {
        TreeNode {
            name: ".".to_string(),
            is_file: false,
            children,
            size: None,
            branches: if all_branches {
                entries.values().flat_map(|e| e.branches.iter().cloned()).collect()
            } else {
                vec![]
            },
        }
    } else {
        TreeNode {
            name: name.to_string(),
            is_file: false,
            children,
            size: None,
            branches: if all_branches {
                entries.values().flat_map(|e| e.branches.iter().cloned()).collect()
            } else {
                vec![]
            },
        }
    }
}

/// Build tree nodes from a map
fn build_tree_node_map(entries: &BTreeMap<String, TreeEntry>, all_branches: bool) -> Vec<TreeNode> {
    let mut children = Vec::new();

    for (child_name, entry) in entries {
        let child_node = TreeNode {
            name: child_name.clone(),
            is_file: entry.is_file,
            children: build_tree_node_map(&entry.children, all_branches),
            size: entry.size,
            branches: if all_branches { entry.branches.clone() } else { vec![] },
        };
        children.push(child_node);
    }

    children
}

/// Output options for tree display
struct TreeOutputOptions {
    /// Show all branches for each file
    all_branches: bool,
    /// Show human-readable file sizes
    human_size: bool,
}

/// Output tree in text format
fn output_tree(node: &TreeNode, prefix: &str, is_last: bool, options: &TreeOutputOptions) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Print current node
    let connector = if is_last { "└── " } else { "├── " };
    let mut line = format!("{prefix}{connector}{}", node.name);

    if node.is_file {
        if let Some(size) = node.size {
            let size_str = if options.human_size {
                format!(" ({})", format_size(size))
            } else {
                format!(" ({size} bytes)")
            };
            line.push_str(&size_str);
        }
    } else {
        line.push('/');
    }

    if options.all_branches && !node.branches.is_empty() {
        let branch_str = format!(" [{} branch(es)]", node.branches.len());
        line.push_str(&branch_str);
    }

    writeln!(handle, "{line}")?;

    // Print children
    let child_prefix = format!("{prefix}{}", if is_last { "    " } else { "│   " });
    let child_count = node.children.len();
    for (i, child) in node.children.iter().enumerate() {
        output_tree(child, &child_prefix, i == child_count.saturating_sub(1), options)?;
    }

    Ok(())
}

/// Format size in human-readable format
#[allow(clippy::as_conversions, clippy::cast_precision_loss, clippy::float_arithmetic)]
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{size}B")
    }
}
