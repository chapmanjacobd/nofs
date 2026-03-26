//! cat command - Read file content from first found branch

use std::path::Path;
use std::io::{self, Read, Write};
use std::fs::File;
use crate::pool::Pool;
use crate::error::Result;

pub fn execute(pool: &Pool, path: &str) -> Result<()> {
    let pool_path = Path::new(path);
    
    // Find first branch containing the file
    if let Some(full_path) = pool.resolve_path_first(pool_path) {
        let mut file = File::open(&full_path)?;
        
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        handle.write_all(&buffer)?;
    } else {
        eprintln!("nofs: cannot open '{}' for reading: No such file", path);
        std::process::exit(1);
    }

    Ok(())
}
