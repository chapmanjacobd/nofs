//! info command - Show share configuration and status

use crate::error::Result;
use crate::output::{BranchInfo, InfoAllOutput, InfoOutput, Policies, ShareSummary};
use crate::pool::{Pool, PoolManager};
use serde_json;
use std::io::{self, Write};

/// Execute info command for a single share
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
#[allow(clippy::too_many_lines)]
pub fn execute_single(pool: &Pool, _verbose: bool, json: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    if json {
        let branches: Vec<BranchInfo> = pool
            .branches
            .iter()
            .map(|branch| {
                let minfree = branch.minfreespace.clone();
                BranchInfo {
                    path: branch.path.display().to_string(),
                    mode: branch.mode.to_string(),
                    min_free_space: minfree,
                }
            })
            .collect();

        let output = InfoOutput {
            share: pool.name.clone(),
            branch_count: pool.branch_count(),
            writable_branch_count: pool.writable_branch_count(),
            read_only_branch_count: pool
                .branch_count()
                .saturating_sub(pool.writable_branch_count()),
            policies: Policies {
                create: pool.create_policy.to_string(),
                search: pool.search_policy.to_string(),
            },
            min_free_space: pool.minfreespace,
            branches,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        writeln!(handle, "Share: {}", pool.name)?;
        writeln!(handle, "======")?;
        writeln!(handle)?;

        writeln!(handle, "Branches:     {}", pool.branch_count())?;
        writeln!(handle, "  Writable:   {}", pool.writable_branch_count())?;
        writeln!(
            handle,
            "  Read-only:  {}",
            pool.branch_count()
                .saturating_sub(pool.writable_branch_count())
        )?;
        writeln!(handle)?;

        writeln!(handle, "Policies:")?;
        writeln!(handle, "  Create:     {}", pool.create_policy)?;
        writeln!(handle, "  Search:     {}", pool.search_policy)?;
        writeln!(handle)?;

        writeln!(handle, "Min Free Space: {} bytes", pool.minfreespace)?;
        writeln!(handle)?;

        writeln!(handle, "Branch List:")?;
        for (i, branch) in pool.branches.iter().enumerate() {
            let mode = branch.mode;
            let minfree = branch
                .minfreespace
                .as_ref()
                .map(|s| format!(" (min: {s})"))
                .unwrap_or_default();

            writeln!(
                handle,
                "  {}. {} [{}]{}",
                i.saturating_add(1),
                branch.path.display(),
                mode,
                minfree
            )?;
        }
    }

    Ok(())
}

/// Execute info command for all shares
///
/// # Errors
///
/// Returns an error if there is an IO error during output.
pub fn execute_all(pool_mgr: &PoolManager, _verbose: bool, json: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    if json {
        let mut shares: Vec<ShareSummary> = Vec::new();
        for name in pool_mgr.pool_names() {
            if let Ok(pool) = pool_mgr.get_pool(name) {
                shares.push(ShareSummary {
                    name: name.to_string(),
                    branch_count: pool.branch_count(),
                    writable_branch_count: pool.writable_branch_count(),
                    create_policy: pool.create_policy.to_string(),
                    search_policy: pool.search_policy.to_string(),
                });
            }
        }

        let output = InfoAllOutput { shares };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        writeln!(handle, "Shares")?;
        writeln!(handle, "======")?;
        writeln!(handle)?;

        // Get all share names
        for name in pool_mgr.pool_names() {
            if let Ok(pool) = pool_mgr.get_pool(name) {
                writeln!(handle, "{name}:")?;
                writeln!(
                    handle,
                    "  Branches: {} ({} writable)",
                    pool.branch_count(),
                    pool.writable_branch_count()
                )?;
                writeln!(
                    handle,
                    "  Policy: {} / {}",
                    pool.create_policy, pool.search_policy
                )?;
                writeln!(handle)?;
            }
        }
    }

    Ok(())
}
