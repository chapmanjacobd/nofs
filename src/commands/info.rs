//! info command - Show pool configuration and status

use std::io::{self, Write};
use crate::pool::Pool;
use crate::error::Result;

pub fn execute(pool: &Pool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    writeln!(handle, "Pool Information").ok();
    writeln!(handle, "================").ok();
    writeln!(handle).ok();

    if let Some(name) = &pool.name {
        writeln!(handle, "Name:         {}", name).ok();
    } else {
        writeln!(handle, "Name:         (unnamed)").ok();
    }

    if let Some(mp) = &pool.mountpoint {
        writeln!(handle, "Mountpoint:   {}", mp.display()).ok();
    } else {
        writeln!(handle, "Mountpoint:   (ad-hoc mode)").ok();
    }

    writeln!(handle, "Branches:     {}", pool.branch_count()).ok();
    writeln!(handle, "  Writable:   {}", pool.writable_branch_count()).ok();
    writeln!(handle, "  Read-only:  {}", pool.branch_count() - pool.writable_branch_count()).ok();
    writeln!(handle).ok();

    writeln!(handle, "Policies:").ok();
    writeln!(handle, "  Create:     {}", pool.create_policy).ok();
    writeln!(handle, "  Search:     {}", pool.search_policy).ok();
    writeln!(handle, "  Action:     {}", pool.action_policy).ok();
    writeln!(handle).ok();

    writeln!(handle, "Min Free Space: {} bytes", pool.minfreespace).ok();
    writeln!(handle).ok();

    writeln!(handle, "Branch List:").ok();
    for (i, branch) in pool.branches.iter().enumerate() {
        let mode = branch.mode;
        let minfree = branch.minfreespace
            .as_ref()
            .map(|s| format!(" (min: {})", s))
            .unwrap_or_default();
        
        writeln!(handle, "  {}. {} [{}]{}", 
            i + 1,
            branch.path.display(),
            mode,
            minfree
        ).ok();
    }

    Ok(())
}
