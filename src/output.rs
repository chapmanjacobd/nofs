//! JSON output types for nofs commands
//!
//! This module provides serializable types for JSON output mode.

use serde::Serialize;

/// Output from the `ls` command
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct LsOutput {
    pub path: String,
    pub entries: Vec<LsEntry>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<ConflictEntry>,
}

/// A single entry in an `ls` output
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct LsEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub entry_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<String>,
}

/// Output from the `find` command
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct FindOutput {
    pub path: String,
    pub files: Vec<String>,
}

/// Output from the `which` command
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct WhichOutput {
    pub path: String,
    pub locations: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub conflict: Option<ConflictEntry>,
}

/// Output from the `create` command
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct CreateOutput {
    pub path: String,
    pub selected_branch: String,
    pub policy: String,
}

/// Output from the `stat` command
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct StatOutput {
    pub share: String,
    pub branch_count: usize,
    pub writable_branch_count: usize,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_percent: Option<f64>,
    pub branches: Vec<BranchStat>,
}

/// Statistics for a single branch
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct BranchStat {
    pub path: String,
    pub mode: String,
    pub total: u64,
    pub used: u64,
    pub available: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_percent: Option<f64>,
}

/// Output from the `info` command for a single share
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct InfoOutput {
    pub share: String,
    pub branch_count: usize,
    pub writable_branch_count: usize,
    pub read_only_branch_count: usize,
    pub policies: Policies,
    pub min_free_space: u64,
    pub branches: Vec<BranchInfo>,
}

/// Output from the `info` command for all shares
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct InfoAllOutput {
    pub shares: Vec<ShareSummary>,
}

/// Summary of a single share
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct ShareSummary {
    pub name: String,
    pub branch_count: usize,
    pub writable_branch_count: usize,
    pub create_policy: String,
    pub search_policy: String,
}

/// Policy configuration
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct Policies {
    pub create: String,
    pub search: String,
}

/// Information about a single branch
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct BranchInfo {
    pub path: String,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_free_space: Option<String>,
}

/// Output from the `exists` command
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct ExistsOutput {
    pub exists: bool,
    pub path: Option<String>,
}

/// Conflict information
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct ConflictEntry {
    pub name: String,
    pub branches: Vec<ConflictBranch>,
}

/// Information about a conflicting branch
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct ConflictBranch {
    pub path: String,
    pub size: u64,
}

/// Generic error output
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct ErrorOutput {
    pub error: String,
}
