//! create command - Get the best branch path for creating a new file

use std::path::Path;
use crate::pool::Pool;
use crate::error::Result;

pub fn execute(pool: &Pool, path: &str, _policy_str: &str) -> Result<()> {
    let pool_path = Path::new(path);
    
    // Get parent directory for path preservation policies
    let parent = pool_path.parent().unwrap_or(Path::new(""));

    // Select the best branch
    let branch = pool.select_create_branch(parent)?;
    
    // Return the full path on the selected branch
    let full_path = branch.path.join(pool_path);
    println!("{}", full_path.display());

    Ok(())
}
