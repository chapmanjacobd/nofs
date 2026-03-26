//! info command - Show pool configuration and status

use crate::error::Result;
use crate::pool::{Pool, PoolManager};
use std::io::{self, Write};

pub fn execute_single(pool: &Pool, _verbose: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    writeln!(handle, "Union Context: {}", pool.name).ok();
    writeln!(handle, "================").ok();
    writeln!(handle).ok();

    writeln!(handle, "Branches:     {}", pool.branch_count()).ok();
    writeln!(handle, "  Writable:   {}", pool.writable_branch_count()).ok();
    writeln!(
        handle,
        "  Read-only:  {}",
        pool.branch_count() - pool.writable_branch_count()
    )
    .ok();
    writeln!(handle).ok();

    writeln!(handle, "Policies:").ok();
    writeln!(handle, "  Create:     {}", pool.create_policy).ok();
    writeln!(handle, "  Search:     {}", pool.search_policy).ok();
    writeln!(handle).ok();

    writeln!(handle, "Min Free Space: {} bytes", pool.minfreespace).ok();
    writeln!(handle).ok();

    writeln!(handle, "Branch List:").ok();
    for (i, branch) in pool.branches.iter().enumerate() {
        let mode = branch.mode;
        let minfree = branch
            .minfreespace
            .as_ref()
            .map(|s| format!(" (min: {})", s))
            .unwrap_or_default();

        writeln!(
            handle,
            "  {}. {} [{}]{}",
            i + 1,
            branch.path.display(),
            mode,
            minfree
        )
        .ok();
    }

    Ok(())
}

pub fn execute_all(pool_mgr: &PoolManager, _verbose: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    writeln!(handle, "Union Contexts").ok();
    writeln!(handle, "==============").ok();
    writeln!(handle).ok();

    // Get all pool names
    for name in pool_mgr.pool_names() {
        if let Ok(pool) = pool_mgr.get_pool(name) {
            writeln!(handle, "{}:", name).ok();
            writeln!(
                handle,
                "  Branches: {} ({} writable)",
                pool.branch_count(),
                pool.writable_branch_count()
            )
            .ok();
            writeln!(
                handle,
                "  Policy: {} / {}",
                pool.create_policy, pool.search_policy
            )
            .ok();
            writeln!(handle).ok();
        }
    }

    Ok(())
}
