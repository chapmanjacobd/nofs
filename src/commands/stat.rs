//! stat command - Show filesystem statistics

use crate::error::Result;
use crate::pool::Pool;
use std::io::{self, Write};

/// Execute the stat command
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
#[allow(clippy::too_many_lines)]
pub fn execute(pool: &Pool, human: bool, _verbose: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let total = pool.total_space();
    let used = pool.total_used_space();
    let available = pool.total_available_space();

    writeln!(handle, "Share: {}", pool.name)?;
    writeln!(
        handle,
        "Branches: {} ({} writable)",
        pool.branch_count(),
        pool.writable_branch_count()
    )?;
    writeln!(handle)?;

    if human {
        writeln!(handle, "Total:     {}", format_size(total))?;
        writeln!(handle, "Used:      {}", format_size(used))?;
        writeln!(handle, "Available: {}", format_size(available))?;
    } else {
        writeln!(handle, "Total:     {total} bytes")?;
        writeln!(handle, "Used:      {used} bytes")?;
        writeln!(handle, "Available: {available} bytes")?;
    }

    if total > 0 {
        #[allow(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            clippy::float_arithmetic
        )]
        let percent_used = (used as f64 / total as f64) * 100.0;
        writeln!(handle, "Use%:      {percent_used:.1}%")?;
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
        let branch_total = branch.total_space().unwrap_or(0);
        let branch_used = branch.used_space().unwrap_or(0);
        let branch_available = branch.available_space().unwrap_or(0);

        #[allow(
            clippy::cast_precision_loss,
            clippy::as_conversions,
            clippy::float_arithmetic
        )]
        let percent = if branch_total > 0 {
            (branch_used as f64 / branch_total as f64) * 100.0
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
                format_size(branch_total),
                format_size(branch_used),
                format_size(branch_available),
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

    Ok(())
}

fn format_size(size: u64) -> String {
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

#[allow(clippy::arithmetic_side_effects)]
fn truncate_path(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - max_len + 3..])
    }
}
