//! ls command - List directory contents

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
pub fn execute(pool: &Pool, path: &str, long: bool, all: bool, verbose: bool) -> Result<()> {
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

    // Collect all entries from all branches
    let mut entries: Vec<(std::path::PathBuf, String)> = Vec::new();

    for branch in &branches {
        let branch_path = branch.path.join(pool_path);

        match fs::read_dir(&branch_path) {
            Ok(read_dir) => {
                for entry in read_dir.flatten() {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy().to_string();

                    // Skip hidden files unless --all
                    if !all && file_name_str.starts_with('.') {
                        continue;
                    }

                    entries.push((entry.path(), file_name_str));
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

    // Sort entries by name
    entries.sort_by(|a, b| a.1.cmp(&b.1));

    // Remove duplicates (same filename from multiple branches)
    let mut seen = std::collections::HashSet::new();
    let unique_entries: Vec<_> = entries
        .into_iter()
        .filter(|(_, name)| seen.insert(name.clone()))
        .collect();

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for (entry_path, file_name) in unique_entries {
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

                writeln!(
                    handle,
                    "{} {} {:>8} {}",
                    file_type,
                    permissions,
                    human_size(size),
                    file_name
                )?;
            }
        }
        // Short format: just the name
        // Add trailing slash for directories
        else if entry_path.is_dir() {
            writeln!(handle, "{file_name}/")?;
        } else {
            writeln!(handle, "{file_name}")?;
        }
    }

    Ok(())
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
