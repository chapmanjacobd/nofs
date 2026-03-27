//! create command - Get the best branch path for creating a new file

use crate::error::Result;
use crate::output::CreateOutput;
use crate::pool::Pool;
use serde_json;
use std::path::Path;

/// Execute the create command
///
/// # Errors
///
/// Returns an error if no suitable branch is found.
pub fn execute(pool: &Pool, path: &str, verbose: bool, json: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Get parent directory for path preservation policies
    let parent = pool_path.parent().unwrap_or_else(|| Path::new(""));

    // Select the best branch
    let branch = pool.select_create_branch(parent)?;

    // Return the full path on the selected branch
    let full_path = branch.path.join(pool_path);
    let full_path_str = full_path.display().to_string();

    if verbose {
        eprintln!("selected:");
        eprintln!("  {} ({} policy)", full_path.display(), pool.create_policy);
    }

    if json {
        let output = CreateOutput {
            path: path.to_string(),
            selected_branch: full_path_str,
            policy: pool.create_policy.to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{full_path_str}");
    }

    Ok(())
}
