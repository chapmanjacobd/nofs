//! Move command for nofs
//!
//! Wrapper around the copy command with move semantics (copy=false).

use crate::commands::cp::{
    execute as copy_execute, parse_file_over_file, parse_folder_conflict_mode, CopyConfig, CopyStats,
};
use crate::error::Result;
use crate::pool::Pool;
use std::sync::Arc;

/// Configuration for move command
#[non_exhaustive]
pub struct MoveConfig<'a> {
    /// Source paths to move
    pub sources: &'a [String],
    /// Destination path
    pub destination: &'a str,
    /// Strategy for file-over-file conflicts
    pub file_over_file: &'a str,
    /// Strategy for file-over-folder conflicts
    pub file_over_folder: &'a str,
    /// Strategy for folder-over-file conflicts
    pub folder_over_file: &'a str,
    /// Simulate without making changes
    pub simulate: bool,
    /// Number of parallel workers
    pub workers: usize,
    /// Enable verbose output
    pub verbose: bool,
    /// Filter by file extensions
    pub extensions: Vec<String>,
    /// Exclude files matching these patterns
    pub exclude: Vec<String>,
    /// Include only files matching these patterns
    pub include: Vec<String>,
    /// Limit number of files to move
    pub limit: Option<u64>,
    /// Limit total size to move
    pub size_limit: Option<u64>,
    /// Size filter for files
    pub size: Option<crate::commands::cp::SizeFilter>,
    /// Share context for path resolution
    pub share: Option<&'a Pool>,
}

/// Execute the move command
///
/// This is essentially a copy operation with `copy=false`, meaning files are
/// moved (renamed) instead of copied. If rename fails, falls back to copy+delete.
///
/// # Errors
///
/// Returns an error if parsing of conflict modes fails or if the copy operation fails.
pub fn execute(config: &MoveConfig<'_>) -> Result<Arc<CopyStats>> {
    execute_with_config(config)
}

/// Execute move with a config struct
///
/// # Errors
///
/// Returns an error if parsing of conflict modes fails or if the copy operation fails.
fn execute_with_config(config: &MoveConfig<'_>) -> Result<Arc<CopyStats>> {
    // Parse conflict resolution strategies
    let file_over_file_strategy = parse_file_over_file(config.file_over_file)?;
    let file_over_folder_mode = parse_folder_conflict_mode(config.file_over_folder)?;
    let folder_over_file_mode = parse_folder_conflict_mode(config.folder_over_file)?;

    let copy_config = CopyConfig {
        is_copy: false, // Move mode
        dry_run: config.simulate,
        workers: config.workers,
        verbose: config.verbose,
        file_over_file: file_over_file_strategy,
        file_over_folder: file_over_folder_mode,
        folder_over_file: folder_over_file_mode,
        extensions: config.extensions.clone(),
        exclude: config.exclude.clone(),
        include: config.include.clone(),
        limit: config.limit,
        size_limit: config.size_limit,
        size: config.size.clone(),
    };

    copy_execute(config.sources, config.destination, &copy_config, config.share)
}
