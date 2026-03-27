//! touch command - Create or update files

use crate::error::Result;
use crate::pool::Pool;
use std::path::Path;

/// Execute the touch command
///
/// # Errors
///
/// Returns an error if the file cannot be created or updated.
pub fn execute(pool: &Pool, path: &str, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Check if file exists in any branch
    let branches = pool.find_all_branches(pool_path);

    if !branches.is_empty() {
        // File exists - update timestamps on all copies
        for branch in branches {
            let full_path = branch.path.join(pool_path);
            if verbose {
                eprintln!("updating timestamp: {}", full_path.display());
            }
            let now = filetime::FileTime::now();
            filetime::set_file_times(&full_path, now, now)?;
        }
    } else {
        // File doesn't exist - create on best branch
        let parent = pool_path.parent().unwrap_or(Path::new(""));
        let branch = pool.select_create_branch(parent)?;

        // Create the full path on the selected branch
        let full_path = branch.path.join(pool_path);

        // Ensure parent directory exists
        if let Some(parent_dir) = full_path.parent() {
            std::fs::create_dir_all(parent_dir)?;
        }

        if verbose {
            eprintln!("creating file: {}", full_path.display());
        }
        std::fs::File::create(&full_path)?;
    }

    Ok(())
}
