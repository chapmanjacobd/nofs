//! rmdir command - Remove empty directories

use crate::error::Result;
use crate::pool::Pool;
use std::path::Path;

/// Execute the rmdir command
///
/// # Errors
///
/// Returns an error if the directory cannot be removed.
pub fn execute(pool: &Pool, path: &str, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Find all branches containing this directory
    let branches = pool.find_all_branches(pool_path);

    if branches.is_empty() {
        eprintln!("nofs: cannot remove '{path}': No such file or directory");
        std::process::exit(1);
    }

    for branch in branches {
        let full_path = branch.path.join(pool_path);
        if !full_path.is_dir() {
            eprintln!("nofs: cannot remove '{}': Not a directory", full_path.display());
            std::process::exit(1);
        }

        // Check if directory is empty
        let is_empty = std::fs::read_dir(&full_path)?.next().is_none();
        if !is_empty {
            eprintln!("nofs: cannot remove '{}': Directory not empty", full_path.display());
            std::process::exit(1);
        }

        if verbose {
            eprintln!("removing directory: {}", full_path.display());
        }
        std::fs::remove_dir(&full_path)?;
    }

    Ok(())
}
