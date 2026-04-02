//! stat command - Show filesystem statistics

use crate::cache::OperationCache;
use crate::error::Result;
use crate::output::{BranchStat, StatOutput};
use crate::pool::Pool;
use serde_json;
use std::io::{self, Write};

/// Configuration for stat command output
#[non_exhaustive]
#[derive(Clone, Copy)]
pub struct StatOptions {
    /// Show human-readable sizes
    pub human: bool,
    /// Enable verbose output
    pub verbose: bool,
    /// Output in JSON format
    pub json: bool,
}

/// Execute the stat command
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
pub fn execute(pool: &Pool, options: StatOptions) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    if options.verbose {
        writeln!(handle, "Resolving share: {}", pool.name)?;
        writeln!(handle, "Branch count: {}", pool.branch_count())?;
        writeln!(handle, "Writable branches: {}", pool.writable_branch_count())?;
    }

    let total = pool.total_space_cached(&cache).unwrap_or(0);
    let used = pool.total_used_space_cached(&cache).unwrap_or(0);
    let available = pool.total_available_space_cached(&cache).unwrap_or(0);

    if options.verbose {
        writeln!(
            handle,
            "Aggregating space statistics from {} branches...",
            pool.branch_count()
        )?;
    }

    let use_percent = if total > 0 {
        #[allow(clippy::as_conversions, clippy::cast_precision_loss, clippy::float_arithmetic)]
        {
            Some((used as f64 / total as f64) * 100.0)
        }
    } else {
        None
    };

    if options.json {
        output_json(pool, &cache, options.verbose, total, used, available, use_percent)?;
    } else {
        let stats = AggregatedStats {
            total,
            used,
            available,
            use_percent,
        };
        output_text(pool, &cache, options.human, options.verbose, &stats)?;
    }

    Ok(())
}

/// Output statistics in JSON format
fn output_json(
    pool: &Pool,
    cache: &OperationCache,
    verbose: bool,
    total: u64,
    used: u64,
    available: u64,
    use_percent: Option<f64>,
) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let mut branches: Vec<BranchStat> = Vec::new();
    for branch in &pool.branches {
        if verbose {
            writeln!(handle, "  Collecting stats for branch: {}", branch.path.display())?;
        }
        let branch_stat = collect_branch_stat(branch, cache);
        branches.push(branch_stat);
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
    Ok(())
}

/// Aggregated statistics for output
struct AggregatedStats {
    /// Total space in bytes
    total: u64,
    /// Used space in bytes
    used: u64,
    /// Available space in bytes
    available: u64,
    /// Usage percentage
    use_percent: Option<f64>,
}

/// Output statistics in text format
fn output_text(pool: &Pool, cache: &OperationCache, human: bool, verbose: bool, stats: &AggregatedStats) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    writeln!(handle, "Share: {}", pool.name)?;
    writeln!(
        handle,
        "Branches: {} ({} writable)",
        pool.branch_count(),
        pool.writable_branch_count()
    )?;
    writeln!(handle)?;

    if human {
        writeln!(handle, "Total:     {}", crate::utils::format_size(stats.total))?;
        writeln!(handle, "Used:      {}", crate::utils::format_size(stats.used))?;
        writeln!(handle, "Available: {}", crate::utils::format_size(stats.available))?;
    } else {
        writeln!(handle, "Total:     {} bytes", stats.total)?;
        writeln!(handle, "Used:      {} bytes", stats.used)?;
        writeln!(handle, "Available: {} bytes", stats.available)?;
    }

    if let Some(percent) = stats.use_percent {
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
        let branch_stat = collect_branch_stat(branch, cache);
        let percent = branch_stat.use_percent.unwrap_or(0.0);
        let mode_str = format!("[{}]", branch.mode);
        let path_str = format!("{} {}", branch.path.display(), mode_str);

        if human {
            writeln!(
                handle,
                "{:<40} {:>12} {:>12} {:>12} {:>7.1}%",
                truncate_path(&path_str, 40),
                crate::utils::format_size(branch_stat.total),
                crate::utils::format_size(branch_stat.used),
                crate::utils::format_size(branch_stat.available),
                percent
            )?;
        } else {
            writeln!(
                handle,
                "{:<40} {:>12} {:>12} {:>12} {:>7.1}%",
                truncate_path(&path_str, 40),
                branch_stat.total,
                branch_stat.used,
                branch_stat.available,
                percent
            )?;
        }
    }

    Ok(())
}

/// Collect statistics for a single branch
fn collect_branch_stat(branch: &crate::branch::Branch, cache: &OperationCache) -> BranchStat {
    let branch_total = branch.total_space_cached(cache).unwrap_or(0);
    let branch_available = branch.available_space_cached(cache).unwrap_or(0);
    let branch_used = branch_total.saturating_sub(branch_available);

    let percent = if branch_total > 0 {
        #[allow(clippy::as_conversions, clippy::cast_precision_loss, clippy::float_arithmetic)]
        {
            (branch_used as f64 / branch_total as f64) * 100.0
        }
    } else {
        0.0
    };

    BranchStat {
        path: branch.path.display().to_string(),
        mode: branch.mode.to_string(),
        total: branch_total,
        used: branch_used,
        available: branch_available,
        use_percent: Some(percent),
    }
}

/// Truncate a path string to a maximum length
fn truncate_path(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        let start = s.len().saturating_sub(max_len.saturating_sub(3));
        format!("...{}", &s[start..])
    }
}
