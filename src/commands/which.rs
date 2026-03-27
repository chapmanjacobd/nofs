//! which command - Find which branch contains a file

use crate::cache::OperationCache;
use crate::conflict::detect_single_file_conflict;
use crate::error::Result;
use crate::output::{ConflictBranch, ConflictEntry, WhichOutput};
use crate::pool::Pool;
use serde_json;
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
    json: bool,
) -> Result<()> {
    let pool_path = Path::new(path);

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    if all {
        // Show all branches containing the file (cached)
        let branches = pool.find_all_branches_cached(pool_path, &cache);

        if branches.is_empty() {
            if json {
                let output = WhichOutput {
                    path: path.to_string(),
                    locations: vec![],
                    conflict: None,
                };
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                eprintln!("nofs: '{path}' not found in share");
            }
            return Ok(());
        }

        // Detect conflicts if requested
        let conflict = if conflicts {
            detect_single_file_conflict(&branches, pool_path, hash)?
        } else {
            None
        };

        // Report conflict to stderr
        if let Some(ref c) = conflict {
            report_conflict(c, verbose)?;
        } else if conflicts && verbose {
            eprintln!("no conflict: file content is identical across branches");
        } else {
            // No conflict or not reporting conflict status
        }

        if verbose {
            let stderr = io::stderr();
            let mut h = stderr.lock();
            writeln!(h, "found in:")?;
            for branch in &branches {
                writeln!(h, "  {}", branch.path.join(pool_path).display())?;
            }
        }

        let locations: Vec<String> = branches
            .iter()
            .map(|branch| branch.path.join(pool_path).display().to_string())
            .collect();

        let json_conflict = conflict.as_ref().map(|c| ConflictEntry {
            name: c.name.clone(),
            branches: c
                .branches
                .iter()
                .map(|b| ConflictBranch {
                    path: b.path.clone(),
                    size: b.size,
                })
                .collect(),
        });

        if json {
            let output = WhichOutput {
                path: path.to_string(),
                locations,
                conflict: json_conflict,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            for loc in &locations {
                writeln!(handle, "{loc}")?;
            }
        }
    }
    // Show first branch containing the file (cached)
    else if let Some(full_path) = pool.resolve_path_first_cached(pool_path, &cache) {
        if verbose {
            eprintln!("selected:");
            eprintln!("  {} (first-found policy)", full_path.display());
        }

        if json {
            let output = WhichOutput {
                path: path.to_string(),
                locations: vec![full_path.display().to_string()],
                conflict: None,
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("{}", full_path.display());
        }
    } else {
        if !json {
            eprintln!("nofs: '{path}' not found in share");
        }
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
