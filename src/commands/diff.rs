//! diff command - Show differences between branches
//!
//! This command detects and reports files that differ across branches.

use crate::branch::Branch;
use crate::cache::OperationCache;
use crate::conflict::{detect_conflicts, detect_single_file_conflict};
use crate::error::{NofsError, Result};
use crate::output::{ConflictBranch, ConflictEntry};
use crate::pool::Pool;
use serde::Serialize;
use std::io::{self, Write};
use std::path::Path;

/// Output from the `diff` command for directory comparison
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct DiffOutput {
    pub path: String,
    pub conflict_count: usize,
    pub conflicts: Vec<ConflictEntry>,
}

/// Output from the `diff` command for single file comparison
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct DiffFileOutput {
    pub path: String,
    pub has_conflict: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<ConflictEntry>,
}

/// Execute the diff command
///
/// # Errors
///
/// Returns an error if there is an IO error during output or file access.
pub fn execute(pool: &Pool, path: &str, verbose: bool, hash: bool, json: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    // Find all branches with this path (cached)
    let branches = pool.find_all_branches_cached(pool_path, &cache);

    if branches.is_empty() {
        return Err(NofsError::Command(format!(
            "cannot access '{path}': No such file or directory"
        )));
    }

    // Check if this is a file or directory by checking the first branch
    let is_file = branches
        .iter()
        .find_map(|b| {
            let full_path = b.path.join(pool_path);
            full_path.exists().then(|| full_path.is_file())
        })
        .unwrap_or(false);

    if is_file {
        // Single file diff
        diff_single_file(pool, &branches, pool_path, path, verbose, hash, json)
    } else {
        // Directory diff
        diff_directory(pool, &branches, pool_path, path, verbose, hash, json)
    }
}

/// Diff a single file across branches
fn diff_single_file(
    _pool: &Pool,
    branches: &[&Branch],
    pool_path: &Path,
    path: &str,
    verbose: bool,
    hash: bool,
    json: bool,
) -> Result<()> {
    let conflict_opt = detect_single_file_conflict(branches, pool_path, hash)?;

    if json {
        let output = DiffFileOutput {
            path: path.to_string(),
            has_conflict: conflict_opt.is_some(),
            conflict: conflict_opt.map(|c| ConflictEntry {
                name: c.name,
                branches: c
                    .branches
                    .into_iter()
                    .map(|b| ConflictBranch {
                        path: b.path,
                        size: b.size,
                        mtime: b.mtime,
                        ctime: b.ctime,
                    })
                    .collect(),
            }),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Text output
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        if let Some(conflict) = conflict_opt {
            writeln!(handle, "conflict: {}", conflict.name)?;
            writeln!(handle)?;
            writeln!(handle, "branches:")?;
            for branch in &conflict.branches {
                writeln!(handle, "  {}", branch.path)?;
                writeln!(handle, "    size: {} bytes", branch.size)?;
                if let Some(mtime) = branch.mtime {
                    writeln!(handle, "    mtime: {}", format_timestamp(mtime))?;
                }
                if let Some(file_hash) = &branch.hash {
                    writeln!(handle, "    hash: {file_hash}")?;
                }
                if verbose {
                    if let Some(ctime) = branch.ctime {
                        writeln!(handle, "    ctime: {}", format_timestamp(ctime))?;
                    }
                }
            }
        } else {
            writeln!(handle, "no conflicts: {path} exists identically across branches")?;
        }
    }

    Ok(())
}

/// Diff a directory across branches
fn diff_directory(
    _pool: &Pool,
    branches: &[&Branch],
    pool_path: &Path,
    path: &str,
    verbose: bool,
    hash: bool,
    json: bool,
) -> Result<()> {
    let conflicts = detect_conflicts(branches, pool_path, hash)?;

    if json {
        let output = DiffOutput {
            path: path.to_string(),
            conflict_count: conflicts.len(),
            conflicts: conflicts
                .into_iter()
                .map(|c| ConflictEntry {
                    name: c.name,
                    branches: c
                        .branches
                        .into_iter()
                        .map(|b| ConflictBranch {
                            path: b.path,
                            size: b.size,
                            mtime: b.mtime,
                            ctime: b.ctime,
                        })
                        .collect(),
                })
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        // Text output
        let stdout = io::stdout();
        let mut handle = stdout.lock();

        if conflicts.is_empty() {
            writeln!(handle, "no conflicts found in {path}")?;
        } else {
            writeln!(handle, "found {} conflicting file(s) in {path}:\n", conflicts.len())?;

            for conflict in &conflicts {
                writeln!(handle, "{}:", conflict.name)?;
                for branch in &conflict.branches {
                    writeln!(handle, "  {} ({} bytes)", branch.path, branch.size)?;
                    if verbose {
                        if let Some(mtime) = branch.mtime {
                            writeln!(handle, "    mtime: {}", format_timestamp(mtime))?;
                        }
                        if let Some(file_hash) = &branch.hash {
                            writeln!(handle, "    hash: {file_hash}")?;
                        }
                    }
                }
                writeln!(handle)?;
            }
        }
    }

    Ok(())
}

/// Format a Unix timestamp as a human-readable date
#[allow(
    clippy::integer_division,
    clippy::arithmetic_side_effects,
    clippy::as_conversions,
    clippy::unnecessary_cast
)]
fn format_timestamp(secs: u64) -> String {
    // Calculate approximate date (this is a rough calculation)
    let secs_since_epoch = secs;
    let days = secs_since_epoch / 86400;
    let remaining_secs = secs_since_epoch % 86400;
    let hours = remaining_secs / 3600;
    let minutes = (remaining_secs % 3600) / 60;
    let seconds = remaining_secs % 60;

    // Calculate approximate year
    let year = 1970 + (days / 365) as u64;
    let day_of_year = days % 365;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        year, 1, day_of_year as u32, hours, minutes, seconds
    )
}
