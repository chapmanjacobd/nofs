//! df command - Show disk free space
//!
//! This command displays filesystem disk space usage in a standard df-like format.

use crate::cache::OperationCache;
use crate::error::Result;
use crate::pool::PoolManager;
use serde::Serialize;
use std::io::{self, Write};

/// Output from the `df` command
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct DfOutput {
    pub filesystems: Vec<DfEntry>,
}

/// A single df entry
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct DfEntry {
    pub filesystem: String,
    pub blocks: u64,
    pub used: u64,
    pub available: u64,
    pub use_percent: Option<f64>,
    pub mounted_on: String,
}

/// Execute the df command
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
#[allow(clippy::fn_params_excessive_bools)]
pub fn execute(
    pool_mgr: &PoolManager,
    context: Option<&str>,
    human: bool,
    total: bool,
    _verbose: bool,
    json: bool,
) -> Result<()> {
    let mut entries = Vec::new();
    let cache = OperationCache::new();

    if let Some(ctx) = context {
        // Specific context/share
        let pool = pool_mgr.get_pool(ctx)?;

        for branch in &pool.branches {
            let branch_total = branch.total_space_cached(&cache).unwrap_or(0);
            let branch_available = branch.available_space_cached(&cache).unwrap_or(0);
            let branch_used = branch_total.saturating_sub(branch_available);
            let use_percent = if branch_total > 0 {
                #[allow(clippy::cast_precision_loss, clippy::as_conversions, clippy::float_arithmetic)]
                {
                    Some((branch_used as f64 / branch_total as f64) * 100.0)
                }
            } else {
                None
            };

            entries.push(DfEntry {
                filesystem: branch.path.to_string_lossy().to_string(),
                blocks: branch_total,
                used: branch_used,
                available: branch_available,
                use_percent,
                mounted_on: ctx.to_string(),
            });
        }
    } else {
        // All shares - iterate through all pools
        for (name, pool) in pool_mgr.pools() {
            for branch in &pool.branches {
                let branch_total = branch.total_space_cached(&cache).unwrap_or(0);
                let branch_available = branch.available_space_cached(&cache).unwrap_or(0);
                let branch_used = branch_total.saturating_sub(branch_available);
                let use_percent = if branch_total > 0 {
                    #[allow(clippy::cast_precision_loss, clippy::as_conversions, clippy::float_arithmetic)]
                    {
                        Some((branch_used as f64 / branch_total as f64) * 100.0)
                    }
                } else {
                    None
                };

                entries.push(DfEntry {
                    filesystem: branch.path.to_string_lossy().to_string(),
                    blocks: branch_total,
                    used: branch_used,
                    available: branch_available,
                    use_percent,
                    mounted_on: name.clone(),
                });
            }
        }
    }

    // Calculate total if requested
    let total_entry = (total && entries.len() > 1).then(|| {
        let total_blocks: u64 = entries.iter().map(|e| e.blocks).sum();
        let total_used: u64 = entries.iter().map(|e| e.used).sum();
        let total_available: u64 = entries.iter().map(|e| e.available).sum();
        let total_percent = if total_blocks > 0 {
            #[allow(clippy::cast_precision_loss, clippy::as_conversions, clippy::float_arithmetic)]
            {
                Some((total_used as f64 / total_blocks as f64) * 100.0)
            }
        } else {
            None
        };

        DfEntry {
            filesystem: "total".to_string(),
            blocks: total_blocks,
            used: total_used,
            available: total_available,
            use_percent: total_percent,
            mounted_on: "-".to_string(),
        }
    });

    if json {
        let output = DfOutput { filesystems: entries };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        output_text(&entries, total_entry, human)?;
    }

    Ok(())
}

/// Output in text format
#[allow(clippy::integer_division)]
fn output_text(entries: &[DfEntry], total: Option<DfEntry>, human: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Print header
    if human {
        writeln!(
            handle,
            "{:<25} {:>8} {:>8} {:>8} {:>6} Mounted on",
            "Filesystem", "Size", "Used", "Avail", "Use%"
        )?;
    } else {
        writeln!(
            handle,
            "{:<25} {:>12} {:>12} {:>12} {:>6} Mounted on",
            "Filesystem", "1K-blocks", "Used", "Available", "Use%"
        )?;
    }

    // Print entries
    for entry in entries {
        let blocks_str = if human {
            format_size(entry.blocks)
        } else {
            format!("{}", entry.blocks / 1024)
        };
        let used_str = if human {
            format_size(entry.used)
        } else {
            format!("{}", entry.used / 1024)
        };
        let avail_str = if human {
            format_size(entry.available)
        } else {
            format!("{}", entry.available / 1024)
        };
        let percent_str = entry
            .use_percent.map_or_else(|| "-".to_string(), |p| format!("{p:.0}%"));

        writeln!(
            handle,
            "{:<25} {:>8} {:>8} {:>8} {:>6} {}",
            truncate_str(&entry.filesystem, 25),
            blocks_str,
            used_str,
            avail_str,
            percent_str,
            entry.mounted_on
        )?;
    }

    // Print total
    if let Some(total_entry) = total {
        let blocks_str = if human {
            format_size(total_entry.blocks)
        } else {
            format!("{}", total_entry.blocks / 1024)
        };
        let used_str = if human {
            format_size(total_entry.used)
        } else {
            format!("{}", total_entry.used / 1024)
        };
        let avail_str = if human {
            format_size(total_entry.available)
        } else {
            format!("{}", total_entry.available / 1024)
        };
        let percent_str = total_entry
            .use_percent.map_or_else(|| "-".to_string(), |p| format!("{p:.0}%"));

        writeln!(
            handle,
            "{:<25} {:>8} {:>8} {:>8} {:>6} -",
            "total", blocks_str, used_str, avail_str, percent_str
        )?;
    }

    Ok(())
}

/// Format size in human-readable format
fn format_size(size: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    const TB: u64 = GB * 1024;

    #[allow(clippy::float_arithmetic, clippy::cast_precision_loss, clippy::as_conversions)]
    if size >= TB {
        format!("{:.1}T", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1}G", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}M", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}K", size as f64 / KB as f64)
    } else {
        format!("{size}")
    }
}

/// Truncate string to max length with ellipsis
#[allow(clippy::arithmetic_side_effects)]
fn truncate_str(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - (max_len - 3)..])
    }
}
