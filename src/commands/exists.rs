//! exists command - Check if a file exists and return its location

use crate::cache::OperationCache;
use crate::error::{NofsError, Result};
use crate::output::ExistsOutput;
use crate::pool::Pool;
use serde_json;
use std::path::Path;

/// Execute the exists command
///
/// # Errors
///
/// Returns an error if the path does not exist or if there is an IO error.
pub fn execute(pool: &Pool, path: &str, verbose: bool, json: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    if pool.exists_cached(pool_path, &cache) {
        // File exists - print first location (cached)
        if let Some(full_path) = pool.resolve_path_first_cached(pool_path, &cache) {
            let full_path_str = full_path.display().to_string();

            if verbose {
                eprintln!("selected:");
                eprintln!("  {} (first-found policy)", full_path.display());
            }

            if json {
                let output = ExistsOutput {
                    exists: true,
                    path: Some(full_path_str),
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                println!("{full_path_str}");
            }
        } else if json {
            let output = ExistsOutput {
                exists: true,
                path: None,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            unreachable!("file exists but no path and not JSON output is impossible");
        }
        Ok(())
    } else {
        // File does not exist
        if json {
            let output = ExistsOutput {
                exists: false,
                path: None,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
            Ok(())
        } else {
            Err(NofsError::Command(format!("'{path}' not found in share")))
        }
    }
}
