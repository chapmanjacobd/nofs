//! exists command - Check if a file exists and return its location

use crate::error::Result;
use crate::pool::Pool;
use std::path::Path;

/// Execute the exists command
///
/// # Errors
///
/// Returns an error if there is an IO error (exits with status code otherwise).
pub fn execute(pool: &Pool, path: &str, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);

    if pool.exists(pool_path) {
        // File exists - print first location
        if let Some(full_path) = pool.resolve_path_first(pool_path) {
            if verbose {
                eprintln!("selected:");
                eprintln!("  {} (first-found policy)", full_path.display());
            }
            println!("{}", full_path.display());
        }
        // Exit with success
        std::process::exit(0);
    } else {
        // File does not exist
        eprintln!("nofs: '{path}' not found in pool");
        // Exit with failure
        std::process::exit(1);
    }
}
