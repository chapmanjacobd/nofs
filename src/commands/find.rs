//! find command - Find files matching patterns

use crate::cache::OperationCache;
use crate::error::{NofsError, Result};
use crate::output::FindOutput;
use crate::pool::Pool;
use serde_json;
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
    json: bool,
) -> Result<()> {
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

    if verbose {
        let stderr = io::stderr();
        let mut h = stderr.lock();
        writeln!(h, "found in:")?;
        for branch in &branches {
            writeln!(h, "  {}", branch.path.join(pool_path).display())?;
        }
    }

    let found_paths_set: dashmap::DashSet<String> = dashmap::DashSet::new();

    let mut handles = Vec::new();
    for branch_ref in branches {
        let branch = (*branch_ref).clone();
        let p_path = pool_path.to_path_buf();
        let n_pattern = name_pattern.map(String::from);
        let t_filter = type_filter.map(String::from);
        let found_paths_set_clone = found_paths_set.clone();

        let handle = std::thread::spawn(move || {
            let branch_path = branch.path.join(&p_path);

            let mut walker = WalkDir::new(&branch_path).follow_links(true);

            if let Some(depth) = maxdepth {
                walker = walker.max_depth(depth);
            }

            for entry_result in walker {
                let Ok(entry) = entry_result else { continue };

                let entry_path = entry.path();

                // Apply name filter
                if let Some(ref pattern) = n_pattern {
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
                if let Some(ref type_) = t_filter {
                    let Ok(metadata) = entry.metadata() else {
                        continue;
                    };

                    let matches = match type_.as_str() {
                        "f" | "file" => metadata.is_file(),
                        "d" | "dir" => metadata.is_dir(),
                        "l" | "link" => metadata.is_symlink(),
                        _ => true,
                    };

                    if !matches {
                        continue;
                    }
                }

                // Get path relative to branch
                let relative = entry_path.strip_prefix(&branch_path).unwrap_or_else(|_| Path::new(""));

                // Get path relative to pool mount point
                let pool_relative = relative.to_path_buf();

                // Add to results (DashSet handles duplicates)
                found_paths_set_clone.insert(pool_relative.display().to_string());
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        let _ = handle.join();
    }

    let mut found_paths: Vec<String> = found_paths_set.into_iter().collect();
    found_paths.sort();

    if json {
        let output = FindOutput {
            path: path.to_string(),
            files: found_paths,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        for path_str in &found_paths {
            writeln!(handle, "{path_str}")?;
        }
    }

    Ok(())
}
