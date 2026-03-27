//! exists command - Check if a file exists and return its location

use crate::cache::OperationCache;
use crate::error::{NofsError, Result};
use crate::pool::Pool;
use std::path::Path;

/// Execute the exists command
///
/// # Errors
///
/// Returns an error if the path does not exist or if there is an IO error.
pub fn execute(pool: &Pool, path: &str, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    if pool.exists_cached(pool_path, &cache) {
        // File exists - print first location (cached)
        if let Some(full_path) = pool.resolve_path_first_cached(pool_path, &cache) {
            if verbose {
                eprintln!("selected:");
                eprintln!("  {} (first-found policy)", full_path.display());
            }
            println!("{}", full_path.display());
        }
        Ok(())
    } else {
        // File does not exist
        Err(NofsError::Command(format!("'{path}' not found in share")))
    }
}
