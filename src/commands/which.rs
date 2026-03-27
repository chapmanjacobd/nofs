//! which command - Find which branch contains a file

use crate::cache::OperationCache;
use crate::conflict::detect_single_file_conflict;
use crate::error::Result;
use crate::pool::Pool;
use std::io::{self, Write};
use std::path::Path;

/// Execute the which command
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
#[allow(clippy::fn_params_excessive_bools)]
pub fn execute(
    pool: &Pool,
    path: &str,
    all: bool,
    verbose: bool,
    conflicts: bool,
    hash: bool,
) -> Result<()> {
    let pool_path = Path::new(path);

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    if all {
        // Show all branches containing the file (cached)
        let branches = pool.find_all_branches_cached(pool_path, &cache);

        if branches.is_empty() {
            eprintln!("nofs: '{path}' not found in share");
            return Ok(());
        }

        // Detect conflicts if requested
        if conflicts {
            if let Some(conflict) = detect_single_file_conflict(&branches, pool_path, hash)? {
                report_conflict(&conflict, verbose)?;
            } else if verbose {
                eprintln!("no conflict: file content is identical across branches");
            } else {
                // Silent when not verbose
            }
        }

        if verbose {
            let stderr = io::stderr();
            let mut h = stderr.lock();
            writeln!(h, "found in:")?;
            for branch in &branches {
                writeln!(h, "  {}", branch.path.join(pool_path).display())?;
            }
        }

        let stdout = io::stdout();
        let mut handle = stdout.lock();

        for branch in branches {
            let full_path = branch.path.join(pool_path);
            writeln!(handle, "{}", full_path.display())?;
        }
    }
    // Show first branch containing the file (cached)
    else if let Some(full_path) = pool.resolve_path_first_cached(pool_path, &cache) {
        if verbose {
            eprintln!("selected:");
            eprintln!("  {} (first-found policy)", full_path.display());
        }
        println!("{}", full_path.display());
    } else {
        eprintln!("nofs: '{path}' not found in share");
    }

    Ok(())
}

/// Report a conflict to stderr
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
fn report_conflict(conflict: &crate::conflict::FileConflict, verbose: bool) -> Result<()> {
    let stderr = io::stderr();
    let mut h = stderr.lock();

    writeln!(
        h,
        "conflict detected: file '{}' differs across branches",
        conflict.name
    )?;

    if verbose {
        for branch in &conflict.branches {
            writeln!(h, "  {} ({} bytes)", branch.path, branch.size)?;
        }
    } else {
        writeln!(h, "  {} versions found", conflict.branches.len())?;
    }

    Ok(())
}
