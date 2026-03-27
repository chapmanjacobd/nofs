//! cat command - Read file content from first found branch

use crate::error::{NofsError, Result};
use crate::pool::Pool;
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::Path;

/// Execute the cat command
///
/// # Errors
///
/// Returns an error if the file cannot be read or written.
pub fn execute(pool: &Pool, path: &str, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Find first branch containing the file
    if let Some(full_path) = pool.resolve_path_first(pool_path) {
        if verbose {
            eprintln!("selected:");
            eprintln!("  {} (first-found policy)", full_path.display());
        }

        let mut file = File::open(&full_path)?;

        let stdout = io::stdout();
        let mut handle = stdout.lock();

        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        handle.write_all(&buffer)?;
    } else {
        return Err(NofsError::Command(format!(
            "cannot open '{path}' for reading: No such file"
        )));
    }

    Ok(())
}
