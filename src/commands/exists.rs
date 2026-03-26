//! exists command - Check if a file exists and return its location

use std::path::Path;
use crate::pool::Pool;
use crate::error::Result;

pub fn execute(pool: &Pool, path: &str) -> Result<()> {
    let pool_path = Path::new(path);
    
    if pool.exists(pool_path) {
        // File exists - print first location
        if let Some(full_path) = pool.resolve_path_first(pool_path) {
            println!("{}", full_path.display());
        }
        // Exit with success
        std::process::exit(0);
    } else {
        // File does not exist
        eprintln!("nofs: '{}' not found in pool", path);
        // Exit with failure
        std::process::exit(1);
    }
}
