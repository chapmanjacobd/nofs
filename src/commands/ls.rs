//! ls command - List directory contents

use crate::branch::Branch;
use crate::conflict::{detect_conflicts, FileConflict};
use crate::error::{NofsError, Result};
use crate::pool::Pool;
use std::fs;
use std::io::{self, Write};
use std::os::linux::fs::MetadataExt;
use std::path::Path;

/// Execute the ls command
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
#[allow(clippy::fn_params_excessive_bools)]
pub fn execute(
    pool: &Pool,
    path: &str,
    long: bool,
    all: bool,
    verbose: bool,
    conflicts: bool,
    hash: bool,
) -> Result<()> {
    let pool_path = Path::new(path);

    // Find all branches with this path
    let branches = pool.find_all_branches(pool_path);

    if branches.is_empty() {
        return Err(NofsError::Command(format!(
            "cannot access '{path}': No such file or directory"
        )));
    }

    if verbose {
        let stderr = io::stderr();
        let mut h = stderr.lock();
        writeln!(h, "found in:")?;
        for branch in &branches {
            writeln!(h, "  {}", branch.path.join(pool_path).display())?;
        }
    }

    // Detect conflicts if requested
    let conflict_list = if conflicts {
        detect_conflicts(&branches, pool_path, hash)?
    } else {
        Vec::new()
    };

    // Report conflicts
    if conflicts && !conflict_list.is_empty() {
        report_conflicts(&conflict_list, verbose)?;
    }

    // Collect all entries from all branches
    let mut entries: Vec<(std::path::PathBuf, String)> =
        collect_directory_entries(&branches, pool_path, all, verbose);

    // Sort entries by name
    entries.sort_by(|a, b| a.1.cmp(&b.1));

    // Remove duplicates (same filename from multiple branches)
    let mut seen = std::collections::HashSet::new();
    let unique_entries: Vec<_> = entries
        .into_iter()
        .filter(|(_, name)| seen.insert(name.clone()))
        .collect();

    // Build a set of conflicting file names for quick lookup
    let conflict_names: std::collections::HashSet<&str> =
        conflict_list.iter().map(|c| c.name.as_str()).collect();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for (entry_path, file_name) in unique_entries {
        let is_conflict = conflict_names.contains(file_name.as_str());

        if long {
            // Long format: show details
            if let Ok(metadata) = fs::metadata(&entry_path) {
                let file_type = if metadata.is_dir() {
                    "d"
                } else if metadata.is_symlink() {
                    "l"
                } else {
                    "-"
                };

                let permissions = format_permissions(metadata.st_mode());
                let size = metadata.len();

                let conflict_marker = if is_conflict { " !" } else { "" };
                writeln!(
                    handle,
                    "{} {} {:>8} {}{}",
                    file_type,
                    permissions,
                    human_size(size),
                    conflict_marker,
                    file_name
                )?;
            }
        }
        // Short format: just the name
        // Add trailing slash for directories
        else if entry_path.is_dir() {
            writeln!(handle, "{file_name}/")?;
        } else {
            let conflict_marker = if is_conflict { " !" } else { "" };
            writeln!(handle, "{file_name}{conflict_marker}")?;
        }
    }

    Ok(())
}

/// Report conflicts to stderr
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
fn report_conflicts(conflicts: &[FileConflict], verbose: bool) -> Result<()> {
    let stderr = io::stderr();
    let mut h = stderr.lock();

    writeln!(
        h,
        "conflicts detected: {} file(s) differ across branches",
        conflicts.len()
    )?;

    if verbose {
        for conflict in conflicts {
            writeln!(h, "  {}:", conflict.name)?;
            for branch in &conflict.branches {
                writeln!(h, "    {} ({} bytes)", branch.path, branch.size)?;
            }
        }
    }

    Ok(())
}

/// Collect directory entries from all branches
fn collect_directory_entries(
    branches: &[&Branch],
    pool_path: &Path,
    all: bool,
    verbose: bool,
) -> Vec<(std::path::PathBuf, String)> {
    let mut entries: Vec<(std::path::PathBuf, String)> = Vec::new();

    for branch in branches {
        let branch_path = branch.path.join(pool_path);

        match fs::read_dir(&branch_path) {
            Ok(read_dir) => {
                for entry_result in read_dir {
                    match entry_result {
                        Ok(entry) => {
                            let file_name = entry.file_name();
                            let file_name_str = file_name.to_string_lossy().to_string();

                            // Skip hidden files unless --all
                            if !all && file_name_str.starts_with('.') {
                                continue;
                            }

                            entries.push((entry.path(), file_name_str));
                        }
                        Err(e) if verbose => {
                            eprintln!(
                                "nofs: warning: failed to read entry in '{}': {}",
                                branch_path.display(),
                                e
                            );
                        }
                        Err(_) => {}
                    }
                }
            }
            Err(e) if verbose => {
                eprintln!(
                    "nofs: warning: cannot read directory '{}': {}",
                    branch_path.display(),
                    e
                );
            }
            Err(_) => {}
        }
    }

    entries
}

fn format_permissions(mode: u32) -> String {
    let mut result = String::with_capacity(9);

    // Owner
    result.push(if mode & 0o400 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o200 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o100 != 0 { 'x' } else { '-' });

    // Group
    result.push(if mode & 0o040 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o020 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o010 != 0 { 'x' } else { '-' });

    // Other
    result.push(if mode & 0o004 != 0 { 'r' } else { '-' });
    result.push(if mode & 0o002 != 0 { 'w' } else { '-' });
    result.push(if mode & 0o001 != 0 { 'x' } else { '-' });

    result
}

fn human_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    #[allow(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        clippy::float_arithmetic
    )]
    if size >= TB {
        format!("{:.1}T", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{size}B")
    }
}
