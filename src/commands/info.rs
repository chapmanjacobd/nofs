//! info command - Show pool configuration and status

use crate::error::Result;
use crate::pool::{Pool, PoolManager};
use std::io::{self, Write};

/// Execute info command for a single pool
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
#[allow(clippy::too_many_lines)]
pub fn execute_single(pool: &Pool, _verbose: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let _ = writeln!(handle, "Union Context: {}", pool.name);
    let _ = writeln!(handle, "================");
    let _ = writeln!(handle);

    let _ = writeln!(handle, "Branches:     {}", pool.branch_count());
    let _ = writeln!(handle, "  Writable:   {}", pool.writable_branch_count());
    let _ = writeln!(
        handle,
        "  Read-only:  {}",
        pool.branch_count() - pool.writable_branch_count()
    );
    let _ = writeln!(handle);

    let _ = writeln!(handle, "Policies:");
    let _ = writeln!(handle, "  Create:     {}", pool.create_policy);
    let _ = writeln!(handle, "  Search:     {}", pool.search_policy);
    let _ = writeln!(handle);

    let _ = writeln!(handle, "Min Free Space: {} bytes", pool.minfreespace);
    let _ = writeln!(handle);

    let _ = writeln!(handle, "Branch List:");
    for (i, branch) in pool.branches.iter().enumerate() {
        let mode = branch.mode;
        let minfree = branch
            .minfreespace
            .as_ref()
            .map(|s| format!(" (min: {s})"))
            .unwrap_or_default();

        let _ = writeln!(
            handle,
            "  {}. {} [{}]{}",
            i + 1,
            branch.path.display(),
            mode,
            minfree
        );
    }

    Ok(())
}

/// Execute info command for all pools
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
pub fn execute_all(pool_mgr: &PoolManager, _verbose: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let _ = writeln!(handle, "Union Contexts");
    let _ = writeln!(handle, "==============");
    let _ = writeln!(handle);

    // Get all pool names
    for name in pool_mgr.pool_names() {
        if let Ok(pool) = pool_mgr.get_pool(name) {
            let _ = writeln!(handle, "{name}:");
            let _ = writeln!(
                handle,
                "  Branches: {} ({} writable)",
                pool.branch_count(),
                pool.writable_branch_count()
            );
            let _ = writeln!(
                handle,
                "  Policy: {} / {}",
                pool.create_policy, pool.search_policy
            );
            let _ = writeln!(handle);
        }
    }

    Ok(())
}
