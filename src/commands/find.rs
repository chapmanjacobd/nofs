//! find command - Find files matching patterns

use crate::error::{NofsError, Result};
use crate::pool::Pool;
use std::io::{self, Write};
use std::path::Path;
use walkdir::WalkDir;

/// Execute the find command
///
/// # Errors
///
/// Returns an error if there is an IO error during output or if the path is not found.
#[allow(clippy::too_many_lines)]
pub fn execute(
    pool: &Pool,
    path: &str,
    name_pattern: Option<&str>,
    type_filter: Option<&str>,
    maxdepth: Option<usize>,
    verbose: bool,
) -> Result<()> {
    let pool_path = Path::new(path);

    // Find all branches with this path
    let branches = pool.find_all_branches(pool_path);

    if branches.is_empty() {
        return Err(NofsError::Command(format!(
            "cannot access '{path}': No such file or directory"
        )));
    }

    if verbose {
        let stderr = io::stderr();
        let mut h = stderr.lock();
        writeln!(h, "found in:")?;
        for branch in &branches {
            writeln!(h, "  {}", branch.path.join(pool_path).display())?;
        }
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let mut seen_paths = std::collections::HashSet::new();

    for branch in &branches {
        let branch_path = branch.path.join(pool_path);

        let mut walker = WalkDir::new(&branch_path).follow_links(true);

        if let Some(depth) = maxdepth {
            walker = walker.max_depth(depth);
        }

        for entry_result in walker {
            let entry = match entry_result {
                Ok(e) => e,
                Err(e) => {
                    if verbose {
                        eprintln!(
                            "nofs: warning: error traversing '{}': {}",
                            branch_path.display(),
                            e
                        );
                    }
                    continue;
                }
            };

            let entry_path = entry.path();

            // Get path relative to branch
            let relative = entry_path
                .strip_prefix(&branch_path)
                .unwrap_or_else(|_| Path::new(""));

            // Get path relative to pool mount point
            let pool_relative = relative.to_path_buf();

            // Skip if we've already output this path
            if !seen_paths.insert(pool_relative.clone()) {
                continue;
            }

            // Apply name filter
            if let Some(pattern) = name_pattern {
                if let Some(file_name) = entry_path.file_name() {
                    let file_name_str = file_name.to_string_lossy();
                    if !glob::Pattern::new(pattern)
                        .map(|p| p.matches(&file_name_str))
                        .unwrap_or(false)
                    {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Apply type filter
            if let Some(type_) = type_filter {
                let Ok(metadata) = entry.metadata() else {
                    continue;
                };

                let matches = match type_ {
                    "f" | "file" => metadata.is_file(),
                    "d" | "dir" => metadata.is_dir(),
                    "l" | "link" => metadata.is_symlink(),
                    _ => true,
                };

                if !matches {
                    continue;
                }
            }

            // Output the path
            writeln!(handle, "{}", pool_relative.display())?;
        }
    }

    Ok(())
}
