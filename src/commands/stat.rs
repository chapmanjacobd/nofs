//! stat command - Show filesystem statistics

use crate::cache::OperationCache;
use crate::error::Result;
use crate::output::{BranchStat, StatOutput};
use crate::pool::Pool;
use serde_json;
use std::io::{self, Write};

/// Execute the stat command
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
#[allow(clippy::fn_params_excessive_bools, clippy::too_many_lines)]
pub fn execute(pool: &Pool, human: bool, verbose: bool, json: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    if verbose {
        writeln!(handle, "Resolving share: {}", pool.name)?;
        writeln!(handle, "Branch count: {}", pool.branch_count())?;
        writeln!(handle, "Writable branches: {}", pool.writable_branch_count())?;
    }

    let total = pool.total_space_cached(&cache);
    let used = pool.total_used_space_cached(&cache);
    let available = pool.total_available_space_cached(&cache);

    if verbose {
        writeln!(
            handle,
            "Aggregating space statistics from {} branches...",
            pool.branch_count()
        )?;
    }

    let use_percent = if total > 0 {
        #[allow(clippy::cast_precision_loss, clippy::as_conversions, clippy::float_arithmetic)]
        {
            Some((used as f64 / total as f64) * 100.0)
        }
    } else {
        None
    };

    if json {
        let mut branches: Vec<BranchStat> = Vec::new();
        for branch in &pool.branches {
            if verbose {
                writeln!(handle, "  Collecting stats for branch: {}", branch.path.display())?;
            }
            // For stat display, we silently use 0 for branches that fail.
            // This allows the command to succeed even if some branches are inaccessible.
            let branch_total = branch.total_space_cached(&cache).unwrap_or(0);
            let branch_available = branch.available_space_cached(&cache).unwrap_or(0);
            let branch_used = branch_total.saturating_sub(branch_available);

            let percent = if branch_total > 0 {
                #[allow(clippy::cast_precision_loss, clippy::as_conversions, clippy::float_arithmetic)]
                {
                    Some((branch_used as f64 / branch_total as f64) * 100.0)
                }
            } else {
                None
            };

            branches.push(BranchStat {
                path: branch.path.display().to_string(),
                mode: branch.mode.to_string(),
                total: branch_total,
                used: branch_used,
                available: branch_available,
                use_percent: percent,
            });
        }

        let output = StatOutput {
            share: pool.name.clone(),
            branch_count: pool.branch_count(),
            writable_branch_count: pool.writable_branch_count(),
            total,
            used,
            available,
            use_percent,
            branches,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        writeln!(handle, "Share: {}", pool.name)?;
        writeln!(
            handle,
            "Branches: {} ({} writable)",
            pool.branch_count(),
            pool.writable_branch_count()
        )?;
        writeln!(handle)?;

        if human {
            writeln!(handle, "Total:     {}", crate::utils::format_size(total))?;
            writeln!(handle, "Used:      {}", crate::utils::format_size(used))?;
            writeln!(handle, "Available: {}", crate::utils::format_size(available))?;
        } else {
            writeln!(handle, "Total:     {total} bytes")?;
            writeln!(handle, "Used:      {used} bytes")?;
            writeln!(handle, "Available: {available} bytes")?;
        }

        if let Some(percent) = use_percent {
            writeln!(handle, "Use%:      {percent:.1}%")?;
        }

        // Show per-branch stats
        writeln!(handle)?;
        writeln!(handle, "Per-branch statistics:")?;
        writeln!(
            handle,
            "{:<40} {:>12} {:>12} {:>12} {:>8}",
            "Branch", "Total", "Used", "Available", "Use%"
        )?;

        for branch in &pool.branches {
            if verbose {
                writeln!(handle, "  Querying branch: {}", branch.path.display())?;
            }
            // For stat display, we silently use 0 for branches that fail.
            // This allows the command to succeed even if some branches are inaccessible.
            let branch_total = branch.total_space_cached(&cache).unwrap_or(0);
            let branch_available = branch.available_space_cached(&cache).unwrap_or(0);
            let branch_used = branch_total.saturating_sub(branch_available);

            let percent = if branch_total > 0 {
                #[allow(clippy::cast_precision_loss, clippy::as_conversions, clippy::float_arithmetic)]
                {
                    (branch_used as f64 / branch_total as f64) * 100.0
                }
            } else {
                0.0
            };

            let mode_str = format!("[{}]", branch.mode);
            let path_str = format!("{} {}", branch.path.display(), mode_str);

            if human {
                writeln!(
                    handle,
                    "{:<40} {:>12} {:>12} {:>12} {:>7.1}%",
                    truncate_path(&path_str, 40),
                    crate::utils::format_size(branch_total),
                    crate::utils::format_size(branch_used),
                    crate::utils::format_size(branch_available),
                    percent
                )?;
            } else {
                writeln!(
                    handle,
                    "{:<40} {:>12} {:>12} {:>12} {:>7.1}%",
                    truncate_path(&path_str, 40),
                    branch_total,
                    branch_used,
                    branch_available,
                    percent
                )?;
            }
        }
    }

    Ok(())
}

/// Truncate a path string to a maximum length
#[allow(clippy::arithmetic_side_effects)]
fn truncate_path(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - max_len + 3..])
    }
}
