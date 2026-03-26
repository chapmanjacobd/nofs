//! where command - Find which branch contains a file

use std::path::Path;
use std::io::{self, Write};
use crate::pool::Pool;
use crate::error::Result;

pub fn execute(pool: &Pool, path: &str, all: bool) -> Result<()> {
    let pool_path = Path::new(path);
    
    if all {
        // Show all branches containing the file
        let branches = pool.find_all_branches(pool_path);
        
        if branches.is_empty() {
            eprintln!("nofs: '{}' not found in pool", path);
            return Ok(());
        }

        let stdout = io::stdout();
        let mut handle = stdout.lock();

        for branch in branches {
            let full_path = branch.path.join(pool_path);
            writeln!(handle, "{}", full_path.display()).ok();
        }
    } else {
        // Show first branch containing the file
        if let Some(full_path) = pool.resolve_path_first(pool_path) {
            println!("{}", full_path.display());
        } else {
            eprintln!("nofs: '{}' not found in pool", path);
        }
    }

    Ok(())
}
