//! ls command - List directory contents

use std::path::Path;
use std::fs;
use std::io::{self, Write};
use std::os::linux::fs::MetadataExt;
use crate::pool::Pool;
use crate::error::Result;

pub fn execute(pool: &Pool, path: &str, long: bool, all: bool, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);
    
    // Find all branches with this path
    let branches = pool.find_all_branches(pool_path);
    
    if branches.is_empty() {
        eprintln!("nofs: cannot access '{}': No such file or directory", path);
        return Ok(());
    }

    if verbose {
        let stderr = io::stderr();
        let mut h = stderr.lock();
        writeln!(h, "found in:").ok();
        for branch in &branches {
            writeln!(h, "  {}", branch.path.join(pool_path).display()).ok();
        }
    }

    // Collect all entries from all branches
    let mut entries: Vec<(std::path::PathBuf, String)> = Vec::new();
    
    for branch in &branches {
        let branch_path = branch.path.join(pool_path);
        
        if let Ok(read_dir) = fs::read_dir(&branch_path) {
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

                writeln!(handle, "{} {} {:>8} {}", 
                    file_type,
                    permissions,
                    human_size(size),
                    file_name
                ).ok();
            }
        } else {
            // Short format: just the name
            // Add trailing slash for directories
            if entry_path.is_dir() {
                writeln!(handle, "{}/", file_name).ok();
            } else {
                writeln!(handle, "{}", file_name).ok();
            }
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

    if size >= TB {
        format!("{:.1}T", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{}B", size)
    }
}
