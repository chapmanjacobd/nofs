//! du command - Disk usage analysis with recursive directory size calculation

use crate::error::Result;
use crate::pool::Pool;
use serde::Serialize;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Output structure for JSON format
#[derive(Debug, Serialize)]
struct DuEntry {
    /// File or directory path
    path: String,
    /// Size in bytes
    size: u64,
    /// Human-readable size (only in JSON output with -H flag)
    #[serde(skip_serializing_if = "Option::is_none")]
    size_human: Option<String>,
}

/// Data collected for a branch
struct DuBranchData {
    /// Total size of all files in the directory
    total_size: u64,
    /// Sizes of subdirectories
    subdirs: BTreeMap<PathBuf, u64>,
}

/// Execute the du command
///
/// # Errors
///
/// Returns an error if there is an IO error during output or path traversal.
#[allow(clippy::too_many_lines, clippy::fn_params_excessive_bools)]
pub fn execute(
    pool: &Pool,
    pool_path: &str,
    human: bool,
    max_depth: Option<usize>,
    all: bool,
    json: bool,
) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Normalize the path: strip leading `/` to make it relative to share root
    let normalized_path = normalize_pool_path(pool_path);
    let pool_path_obj = Path::new(&normalized_path);

    // Resolve the path across all branches
    let resolved_paths = pool.resolve_path(pool_path_obj);

    if resolved_paths.is_empty() {
        return Err(crate::error::NofsError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("Path not found in any branch: {pool_path}"),
        )));
    }

    // Collect disk usage from all branches
    let mut branch_usage: BTreeMap<PathBuf, DuBranchData> = BTreeMap::new();

    for branch_path in &resolved_paths {
        // Verify this path belongs to a branch
        if !pool
            .branches
            .iter()
            .any(|b| branch_path.starts_with(&b.path))
        {
            continue;
        }

        let data = calculate_directory_usage(branch_path, max_depth, all);

        branch_usage.insert(branch_path.clone(), data);
    }

    if json {
        let mut entries: Vec<DuEntry> = Vec::new();
        for (path, data) in &branch_usage {
            let size_human = human.then(|| crate::utils::format_size(data.total_size));
            entries.push(DuEntry {
                path: path.display().to_string(),
                size: data.total_size,
                size_human,
            });

            // Add subdirectories if showing all
            if all {
                for (subpath, size) in &data.subdirs {
                    let subdir_size_human = human.then(|| crate::utils::format_size(*size));
                    entries.push(DuEntry {
                        path: subpath.display().to_string(),
                        size: *size,
                        size_human: subdir_size_human,
                    });
                }
            }
        }
        println!("{}", serde_json::to_string_pretty(&entries)?);
    } else {
        // Human-readable output format (similar to du command)
        for (path, data) in &branch_usage {
            let size_str = if human {
                crate::utils::format_size(data.total_size)
            } else {
                data.total_size.to_string()
            };
            writeln!(handle, "{:<12} {}", size_str, path.display())?;

            // Show subdirectories if requested
            if all {
                let mut sorted_subdirs: Vec<_> = data.subdirs.iter().collect();
                sorted_subdirs.sort_by(|a, b| a.0.cmp(b.0));
                for (subpath, size) in sorted_subdirs {
                    let subdir_size_str = if human {
                        crate::utils::format_size(*size)
                    } else {
                        size.to_string()
                    };
                    writeln!(handle, "{:<12} {}", subdir_size_str, subpath.display())?;
                }
            }
        }
    }

    Ok(())
}

/// Calculate directory usage recursively
fn calculate_directory_usage(path: &Path, max_depth: Option<usize>, all: bool) -> DuBranchData {
    let mut total_size = 0u64;
    let mut subdirs: BTreeMap<PathBuf, u64> = BTreeMap::new();

    let base_depth = path.components().count();

    for entry in walkdir::WalkDir::new(path)
        .max_depth(max_depth.unwrap_or(usize::MAX))
        .into_iter()
        .filter_map(std::result::Result::ok)
    {
        let Ok(metadata) = entry.metadata() else {
            continue; // Skip files we can't read metadata for
        };

        if metadata.is_file() {
            total_size = total_size.saturating_add(metadata.len());
        }

        // Track directory sizes if showing all
        if all && metadata.is_dir() {
            let entry_depth = entry_path_components_count(&entry);
            // Only include subdirectories, not the root
            if entry_depth > base_depth {
                subdirs.insert(entry.path().to_path_buf(), 0);
            }
        }
    }

    // Calculate per-directory sizes by aggregating file sizes
    if all {
        for entry in walkdir::WalkDir::new(path)
            .max_depth(max_depth.unwrap_or(usize::MAX))
            .into_iter()
            .filter_map(std::result::Result::ok)
        {
            let Ok(metadata) = entry.metadata() else {
                continue;
            };

            if metadata.is_file() {
                let file_size = metadata.len();
                // Collect matching directory paths first to avoid borrow conflict
                let matching_dirs: Vec<_> = subdirs
                    .keys()
                    .filter(|dir_path| entry.path().starts_with(dir_path))
                    .cloned()
                    .collect();
                // Add this file's size to all matching parent directories
                for dir_path in matching_dirs {
                    if let Some(size) = subdirs.get_mut(&dir_path) {
                        *size = size.saturating_add(file_size);
                    }
                }
            }
        }
    }

    DuBranchData {
        total_size,
        subdirs,
    }
}

/// Get the number of components in an entry's path
///
/// This is a helper to avoid borrowing issues with `entry.path().components().count()`
fn entry_path_components_count(entry: &walkdir::DirEntry) -> usize {
    entry.path().components().count()
}

/// Normalize a pool path by stripping leading `/` to make it relative to share root
///
/// In the nofs context, paths like `/`, `/dir`, `/dir/subdir` should be treated
/// as relative to the share root, not as absolute filesystem paths.
fn normalize_pool_path(pool_path: &str) -> String {
    // Strip leading `/` characters to make the path relative
    pool_path.trim_start_matches('/').to_string()
}
