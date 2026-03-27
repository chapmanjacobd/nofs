//! ls command - List directory contents

use crate::branch::Branch;
use crate::cache::OperationCache;
use crate::conflict::{detect_conflicts, FileConflict};
use crate::error::{NofsError, Result};
use crate::output::{ConflictBranch, ConflictEntry, LsEntry, LsOutput};
use crate::pool::Pool;
use serde_json;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

/// Execute the ls command
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
#[allow(clippy::too_many_arguments, clippy::fn_params_excessive_bools)]
pub fn execute(
    pool: &Pool,
    path: &str,
    long: bool,
    all: bool,
    verbose: bool,
    conflicts: bool,
    hash: bool,
    json: bool,
) -> Result<()> {
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
    let mut entries: Vec<(std::path::PathBuf, String)> = collect_directory_entries(&branches, pool_path, all, verbose);

    // Sort entry names alphabetically
    entries.sort_by(|a, b| a.1.cmp(&b.1));

    // Remove duplicates (same filename from multiple branches)
    let mut seen = std::collections::HashSet::new();
    let unique_entries: Vec<_> = entries
        .into_iter()
        .filter(|(_, name)| seen.insert(name.clone()))
        .collect();

    // Build a set of conflicting file names for quick lookup
    let conflict_names: std::collections::HashSet<&str> = conflict_list.iter().map(|c| c.name.as_str()).collect();

    if json {
        output_json(path, &unique_entries, &conflict_list, long, &conflict_names)?;
    } else {
        output_text(&unique_entries, long, &conflict_names)?;
    }

    Ok(())
}

/// Output JSON format
///
/// # Errors
///
/// Returns an error if there is a serialization or IO error.
fn output_json(
    path: &str,
    entries: &[(std::path::PathBuf, String)],
    conflict_list: &[FileConflict],
    long: bool,
    conflict_names: &std::collections::HashSet<&str>,
) -> Result<()> {
    let json_entries: Vec<LsEntry> = entries
        .iter()
        .map(|(entry_path, file_name)| {
            let entry_type = if entry_path.is_dir() {
                "directory"
            } else if entry_path.is_symlink() {
                "symlink"
            } else {
                "file"
            }
            .to_string();

            let (size, permissions) = if long {
                fs::metadata(entry_path)
                    .map(|metadata| (Some(metadata.len()), Some(format_permissions(get_mode(&metadata)))))
                    .unwrap_or((None, None))
            } else {
                (None, None)
            };

            let is_conflict = conflict_names.contains(file_name.as_str());
            let name = if is_conflict {
                format!("{file_name} !")
            } else {
                file_name.clone()
            };

            LsEntry {
                name,
                entry_type,
                size,
                permissions,
            }
        })
        .collect();

    let json_conflicts: Vec<ConflictEntry> = conflict_list
        .iter()
        .map(|c| ConflictEntry {
            name: c.name.clone(),
            branches: c
                .branches
                .iter()
                .map(|b| ConflictBranch {
                    path: b.path.clone(),
                    size: b.size,
                    mtime: b.mtime,
                    ctime: b.ctime,
                })
                .collect(),
        })
        .collect();

    let output = LsOutput {
        path: path.to_string(),
        entries: json_entries,
        conflicts: json_conflicts,
    };

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

/// Output text format
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
fn output_text(
    entries: &[(std::path::PathBuf, String)],
    long: bool,
    conflict_names: &std::collections::HashSet<&str>,
) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for (entry_path, file_name) in entries {
        let is_conflict = conflict_names.contains(file_name.as_str());

        if long {
            // Long format: show details
            if let Ok(metadata) = fs::metadata(entry_path) {
                let file_type = if metadata.is_dir() {
                    "d"
                } else if metadata.is_symlink() {
                    "l"
                } else {
                    "-"
                };

                let permissions = format_permissions(get_mode(&metadata));
                let size = metadata.len();

                let conflict_marker = if is_conflict { " !" } else { "" };
                writeln!(
                    handle,
                    "{} {} {:>8} {}{}",
                    file_type,
                    permissions,
                    crate::utils::format_size(size),
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
    let mut handles = Vec::new();

    for branch_ref in branches {
        let branch = (*branch_ref).clone();
        let p_path = pool_path.to_path_buf();
        let handle = std::thread::spawn(move || {
            let mut entries = Vec::new();
            let branch_path = branch.path.join(&p_path);

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
            entries
        });
        handles.push(handle);
    }

    let mut all_entries = Vec::new();
    for handle in handles {
        if let Ok(entries) = handle.join() {
            all_entries.extend(entries);
        }
    }

    all_entries
}

/// Returns the file mode (permission bits) from metadata.
#[cfg(unix)]
fn get_mode(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode()
}

/// Returns the file mode (permission bits) from metadata.
#[cfg(not(unix))]
fn get_mode(metadata: &fs::Metadata) -> u32 {
    let mode = if metadata.permissions().readonly() {
        0o444
    } else {
        0o666
    };
    if metadata.is_dir() {
        mode | 0o111
    } else {
        mode
    }
}

/// Format file permissions as rwx string
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
