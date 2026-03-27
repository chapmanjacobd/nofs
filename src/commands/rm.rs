//! rm command - Remove files and directories

use crate::error::Result;
use crate::pool::Pool;
use std::path::Path;

/// Execute the rm command
///
/// # Errors
///
/// Returns an error if the file cannot be removed.
pub fn execute(pool: &Pool, path: &str, recursive: bool, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Find all branches containing this file
    let branches = pool.find_all_branches(pool_path);

    if branches.is_empty() {
        eprintln!("nofs: cannot remove '{path}': No such file or directory");
        std::process::exit(1);
    }

    for branch in branches {
        let full_path = branch.path.join(pool_path);
        if recursive {
            if verbose {
                eprintln!("removing: {}", full_path.display());
            }
            std::fs::remove_dir_all(&full_path)?;
        } else {
            if full_path.is_dir() {
                if verbose {
                    eprintln!("removing directory: {}", full_path.display());
                }
                std::fs::remove_dir(&full_path)?;
            } else {
                if verbose {
                    eprintln!("removing: {}", full_path.display());
                }
                std::fs::remove_file(&full_path)?;
            }
        }
    }

    Ok(())
}
