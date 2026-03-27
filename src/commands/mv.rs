//! Move command for nofs
//!
//! Wrapper around the copy command with move semantics (copy=false).

use crate::commands::cp::{
    execute as copy_execute, parse_file_over_file, CopyConfig, CopyStats, FolderConflictMode,
};
use crate::error::{NofsError, Result};
use crate::pool::Pool;
use std::sync::Arc;

/// Execute the move command
///
/// This is essentially a copy operation with `copy=false`, meaning files are
/// moved (renamed) instead of copied. If rename fails, falls back to copy+delete.
///
/// # Errors
///
/// Returns an error if parsing of conflict modes fails or if the copy operation fails.
#[allow(clippy::too_many_arguments)]
pub fn execute(
    sources: &[String],
    destination: &str,
    file_over_file: &str,
    file_over_folder: &str,
    folder_over_file: &str,
    simulate: bool,
    workers: usize,
    verbose: bool,
    extensions: Vec<String>,
    exclude: Vec<String>,
    include: Vec<String>,
    limit: Option<u64>,
    size_limit: Option<u64>,
    share: Option<&Pool>,
) -> Result<Arc<CopyStats>> {
    // Parse conflict resolution strategies
    let file_over_file_strategy = parse_file_over_file(file_over_file)?;
    let file_over_folder_mode = parse_folder_conflict_mode(file_over_folder)?;
    let folder_over_file_mode = parse_folder_conflict_mode(folder_over_file)?;

    let config = CopyConfig {
        copy: false, // Move mode
        simulate,
        workers,
        verbose,
        file_over_file: file_over_file_strategy,
        file_over_folder: file_over_folder_mode,
        folder_over_file: folder_over_file_mode,
        extensions,
        exclude,
        include,
        limit,
        size_limit,
    };

    copy_execute(sources, destination, &config, share)
}

fn parse_folder_conflict_mode(s: &str) -> Result<FolderConflictMode> {
    match s.to_lowercase().as_str() {
        "skip" => Ok(FolderConflictMode::Skip),
        "rename-src" => Ok(FolderConflictMode::RenameSrc),
        "rename-dest" => Ok(FolderConflictMode::RenameDest),
        "delete-src" => Ok(FolderConflictMode::DeleteSrc),
        "delete-dest" => Ok(FolderConflictMode::DeleteDest),
        "merge" => Ok(FolderConflictMode::Merge),
        _ => Err(NofsError::Parse(format!(
            "Unknown folder conflict mode: {s}"
        ))),
    }
}
