//! mkdir command - Create directories

use crate::error::Result;
use crate::pool::Pool;
use std::path::Path;

/// Execute the mkdir command
///
/// # Errors
///
/// Returns an error if the directory cannot be created.
pub fn execute(pool: &Pool, path: &str, parents: bool, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Get the best branch for creating this path
    let parent = pool_path.parent().unwrap_or(Path::new(""));
    let branch = pool.select_create_branch(parent)?;

    // Create the full path on the selected branch
    let full_path = branch.path.join(pool_path);

    if parents {
        if verbose {
            eprintln!("creating directory (with parents): {}", full_path.display());
        }
        std::fs::create_dir_all(&full_path)?;
    } else {
        if verbose {
            eprintln!("creating directory: {}", full_path.display());
        }
        std::fs::create_dir(&full_path)?;
    }

    Ok(())
}
