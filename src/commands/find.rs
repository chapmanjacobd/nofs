//! find command - Find files matching patterns

use std::path::Path;
use std::io::{self, Write};
use walkdir::WalkDir;
use crate::pool::Pool;
use crate::error::Result;

pub fn execute(
    pool: &Pool,
    path: &str,
    name_pattern: Option<&str>,
    type_filter: Option<&str>,
    maxdepth: Option<usize>,
) -> Result<()> {
    let pool_path = Path::new(path);
    
    // Find all branches with this path
    let branches = pool.find_all_branches(pool_path);
    
    if branches.is_empty() {
        eprintln!("nofs: cannot access '{}': No such file or directory", path);
        return Ok(());
    }

    let stdout = io::stdout();
    let mut handle = stdout.lock();
    let mut seen_paths = std::collections::HashSet::new();

    for branch in &branches {
        let branch_path = branch.path.join(pool_path);
        
        let mut walker = WalkDir::new(&branch_path)
            .follow_links(true);
        
        if let Some(depth) = maxdepth {
            walker = walker.max_depth(depth);
        }

        for entry in walker.into_iter().flatten() {
            let entry_path = entry.path();
            
            // Get path relative to branch
            let relative = entry_path.strip_prefix(&branch_path)
                .unwrap_or(Path::new(""));
            
            // Get path relative to pool mount point
            let pool_relative = if let Some(mp) = &pool.mountpoint {
                mp.join(relative)
            } else {
                relative.to_path_buf()
            };

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
                let metadata = match entry.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
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
            writeln!(handle, "{}", pool_relative.display()).ok();
        }
    }

    Ok(())
}
