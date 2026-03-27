//! Copy command for nofs
//!
//! Implements cp/mv-like functionality with support for nofs context paths,
//! conflict resolution strategies, and parallel operations.

use crate::branch::Branch;
use crate::error::{NofsError, Result};
use crate::pool::Pool;
use std::fs;
use std::io::{self, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Resolved path with branch information
struct ResolvedPath {
    /// The resolved file system path
    path: PathBuf,
    /// Branch index in the share (if resolved from a share path)
    branch_index: Option<usize>,
}

/// Resolve a path that may have a share: prefix
///
/// If the path has a share prefix (e.g., <media:/path>), it will be resolved
/// using the provided share. For existing files, finds the branch containing
/// the file. For new paths, uses policy-based branch selection.
/// Otherwise, the path is returned as-is.
///
/// # Errors
///
/// Returns an error if the share is not found or if path resolution fails.
#[allow(clippy::arithmetic_side_effects)]
fn resolve_path(path_str: &str, share: Option<&Pool>, for_create: bool) -> Result<ResolvedPath> {
    if let Some(colon_idx) = path_str.find(':') {
        let potential_prefix = &path_str[..colon_idx];
        if !potential_prefix.contains('/') {
            let share_name = potential_prefix;
            let relative_path = &path_str[colon_idx + 1..];

            if let Some(pool) = share {
                if pool.name == share_name {
                    let (branch, branch_idx) = if for_create {
                        // For create operations, use policy-based selection
                        let branch = select_branch_for_create(pool, Some(relative_path.as_ref()))?;
                        let idx = pool
                            .branches
                            .iter()
                            .position(|b| b.path == branch.path)
                            .unwrap_or(0);
                        (branch, idx)
                    } else {
                        // For existing files, find the branch containing the file
                        let branch = select_branch_for_read(pool, Path::new(relative_path))?;
                        let idx = pool
                            .branches
                            .iter()
                            .position(|b| b.path == branch.path)
                            .unwrap_or(0);
                        (branch, idx)
                    };

                    return Ok(ResolvedPath {
                        path: branch.path.join(relative_path),
                        branch_index: Some(branch_idx),
                    });
                }
            }
            return Err(NofsError::CopyMove(format!(
                "Share '{share_name}' not found or has no branches"
            )));
        }
    }

    // No share prefix, return as-is
    Ok(ResolvedPath {
        path: PathBuf::from(path_str),
        branch_index: None,
    })
}

/// Select a branch for create operations using the pool's create policy
fn select_branch_for_create<'a>(
    pool: &'a Pool,
    relative_path: Option<&Path>,
) -> Result<&'a Branch> {
    use crate::policy::CreatePolicy;

    let policy_executor = CreatePolicy::new(&pool.branches, pool.minfreespace);
    policy_executor
        .select(pool.create_policy, relative_path)
        .map_err(|e| NofsError::CopyMove(format!("No suitable branch: {e}")))
}

/// Select a branch for read operations (finds the branch containing the file)
fn select_branch_for_read<'a>(pool: &'a Pool, relative_path: &Path) -> Result<&'a Branch> {
    // For reads, find the branch that has the file
    // Prefer RW branches, then NC, then RO
    for branch in &pool.branches {
        if branch.path.join(relative_path).exists() {
            return Ok(branch);
        }
    }
    // File not found in any branch
    Err(NofsError::CopyMove(format!(
        "File not found in share '{}'",
        pool.name
    )))
}

/// Resolve a destination path, preferring the same branch as the source
/// when both are in the same share.
///
/// This ensures that moves within a share stay on the same branch (avoiding
/// cross-device moves which would require copy+delete).
///
/// # Errors
///
/// Returns an error if the share is not found or if path resolution fails.
#[allow(clippy::arithmetic_side_effects, clippy::indexing_slicing)]
fn resolve_dest_path(
    dest_str: &str,
    share: Option<&Pool>,
    source_branch_index: Option<usize>,
) -> Result<PathBuf> {
    if let Some(colon_idx) = dest_str.find(':') {
        let potential_prefix = &dest_str[..colon_idx];
        if !potential_prefix.contains('/') {
            let share_name = potential_prefix;
            let relative_path = &dest_str[colon_idx + 1..];

            if let Some(pool) = share {
                if pool.name == share_name {
                    // If source was from this share, use the same branch
                    if let Some(src_idx) = source_branch_index {
                        if src_idx < pool.branches.len() {
                            // Verify the branch is writable (not RO)
                            let branch = &pool.branches[src_idx];
                            if branch.can_create() {
                                return Ok(branch.path.join(relative_path));
                            }
                        }
                    }

                    // Otherwise, use policy-based selection
                    let branch = select_branch_for_create(pool, Some(relative_path.as_ref()))?;
                    return Ok(branch.path.join(relative_path));
                }
            }
            return Err(NofsError::CopyMove(format!(
                "Share '{share_name}' not found or has no branches"
            )));
        }
    }

    // No share prefix, return as-is
    Ok(PathBuf::from(dest_str))
}

/// Conflict resolution mode for file-over-file conflicts
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOverFileMode {
    Skip,
    RenameSrc,
    RenameDest,
    DeleteSrc,
    DeleteDest,
}

/// Conflict resolution mode for folder conflicts
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderConflictMode {
    Skip,
    RenameSrc,
    RenameDest,
    DeleteSrc,
    DeleteDest,
    Merge,
}

/// File-over-file strategy with optional conditions
#[non_exhaustive]
#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct FileOverFileStrategy {
    pub skip_hash: bool,
    pub skip_size: bool,
    pub skip_larger: bool,
    pub skip_smaller: bool,
    pub delete_dest_hash: bool,
    pub delete_dest_size: bool,
    pub delete_dest_larger: bool,
    pub delete_dest_smaller: bool,
    pub delete_src_hash: bool,
    pub delete_src_size: bool,
    pub delete_src_larger: bool,
    pub delete_src_smaller: bool,
    pub required: FileOverFileMode,
}

impl Default for FileOverFileStrategy {
    fn default() -> Self {
        Self {
            skip_hash: false,
            skip_size: false,
            skip_larger: false,
            skip_smaller: false,
            delete_dest_hash: false,
            delete_dest_size: false,
            delete_dest_larger: false,
            delete_dest_smaller: false,
            delete_src_hash: false,
            delete_src_size: false,
            delete_src_larger: false,
            delete_src_smaller: false,
            required: FileOverFileMode::DeleteDest,
        }
    }
}

/// Parse file-over-file strategy string
///
/// Format: "skip-hash rename-dest" or "delete-src-smaller skip"
///
/// # Errors
///
/// Returns an error if an unknown mode or option is provided.
pub fn parse_file_over_file(spec: &str) -> Result<FileOverFileStrategy> {
    let mut strategy = FileOverFileStrategy::default();
    let parts: Vec<&str> = spec.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(strategy);
    }

    // Last part is the required mode
    let required_str = parts.last().copied().unwrap_or("skip");
    strategy.required = match required_str {
        "skip" => FileOverFileMode::Skip,
        "rename-src" => FileOverFileMode::RenameSrc,
        "rename-dest" => FileOverFileMode::RenameDest,
        "delete-src" => FileOverFileMode::DeleteSrc,
        "delete-dest" => FileOverFileMode::DeleteDest,
        _ => {
            return Err(NofsError::Parse(format!(
                "Unknown file-over-file mode: {required_str}"
            )))
        }
    };

    // Previous parts are optional conditions
    for opt in parts.iter().take(parts.len().saturating_sub(1)) {
        match *opt {
            "skip-hash" => strategy.skip_hash = true,
            "skip-size" => strategy.skip_size = true,
            "skip-larger" => strategy.skip_larger = true,
            "skip-smaller" => strategy.skip_smaller = true,
            "delete-dest-hash" => strategy.delete_dest_hash = true,
            "delete-dest-size" => strategy.delete_dest_size = true,
            "delete-dest-larger" => strategy.delete_dest_larger = true,
            "delete-dest-smaller" => strategy.delete_dest_smaller = true,
            "delete-src-hash" => strategy.delete_src_hash = true,
            "delete-src-size" => strategy.delete_src_size = true,
            "delete-src-larger" => strategy.delete_src_larger = true,
            "delete-src-smaller" => strategy.delete_src_smaller = true,
            _ => {
                return Err(NofsError::Parse(format!(
                    "Unknown file-over-file option: {opt}"
                )))
            }
        }
    }

    Ok(strategy)
}

/// Statistics for copy/move operations
#[non_exhaustive]
#[derive(Debug, Default)]
pub struct CopyStats {
    pub files_copied: AtomicU64,
    pub folders_created: AtomicU64,
    pub bytes_copied: AtomicU64,
    pub files_skipped: AtomicU64,
    pub conflicts_resolved: AtomicU64,
    pub errors: AtomicU64,
    pub sample_hashes: AtomicI64,
    pub full_hashes: AtomicI64,
}

/// Copy operation configuration
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CopyConfig {
    pub copy: bool,     // true = copy, false = move
    pub simulate: bool, // dry-run mode
    pub workers: usize, // number of parallel workers
    pub verbose: bool,  // verbose output
    pub file_over_file: FileOverFileStrategy,
    pub file_over_folder: FolderConflictMode,
    pub folder_over_file: FolderConflictMode,
    pub extensions: Vec<String>, // filter by extension
    pub exclude: Vec<String>,    // exclude patterns
    pub include: Vec<String>,    // include patterns
    pub limit: Option<u64>,      // limit number of files
    pub size_limit: Option<u64>, // limit total size in bytes
}

impl Default for CopyConfig {
    fn default() -> Self {
        Self {
            copy: true,
            simulate: false,
            workers: 4,
            verbose: false,
            file_over_file: FileOverFileStrategy::default(),
            file_over_folder: FolderConflictMode::Merge,
            folder_over_file: FolderConflictMode::Merge,
            extensions: Vec::new(),
            exclude: Vec::new(),
            include: Vec::new(),
            limit: None,
            size_limit: None,
        }
    }
}

/// Execute the copy command
///
/// # Errors
///
/// Returns an error if sources are empty or if destination is invalid.
#[allow(clippy::too_many_lines)]
pub fn execute(
    sources: &[String],
    destination: &str,
    config: &CopyConfig,
    share: Option<&Pool>,
) -> Result<Arc<CopyStats>> {
    let stats = Arc::new(CopyStats::default());
    let start_time = Instant::now();

    if sources.is_empty() {
        return Err(NofsError::CopyMove(
            "At least one source is required".to_string(),
        ));
    }

    // Resolve destination path (may have share: prefix) - for create
    let dest_path = resolve_path(destination, share, true)?.path;

    // Check if destination exists
    let dest_exists = dest_path.exists();
    let dest_is_dir = dest_exists && dest_path.is_dir();

    // If multiple sources, destination must be a directory
    if sources.len() > 1 && dest_exists && !dest_is_dir {
        return Err(NofsError::CopyMove(
            "Destination must be a directory when copying multiple sources".to_string(),
        ));
    }

    // Create destination directory if it doesn't exist and we have multiple sources
    if !config.simulate && !dest_exists && sources.len() > 1 {
        fs::create_dir_all(&dest_path)?;
    }

    // Process each source
    for source in sources {
        // Resolve source path (may have share: prefix) - for read (existing file)
        let source_resolved = resolve_path(source, share, false)?;
        let source_path = &source_resolved.path;
        let source_branch_index = source_resolved.branch_index;

        if !source_path.exists() {
            eprintln!("Source {} does not exist", shell_quote(source));
            stats.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        // Determine final destination for this source
        let final_dest = if sources.len() > 1 || (dest_exists && dest_is_dir) {
            // Merge into destination directory
            let source_name = source_path.file_name().unwrap_or(source_path.as_os_str());

            // If destination is a share path, resolve it using the same branch as source
            let dest_base = resolve_dest_path(destination, share, source_branch_index)?;
            dest_base.join(source_name)
        } else {
            // Single file to single file - resolve destination preserving source branch
            resolve_dest_path(destination, share, source_branch_index)?
        };

        // Process the source
        if let Err(e) = process_source(
            source_path,
            &final_dest,
            config,
            &stats,
            share,
            &Arc::new(Mutex::new(0u64)),
            &Arc::new(Mutex::new(0u64)),
        ) {
            eprintln!("Error processing {}: {}", shell_quote(source), e);
            stats.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    if config.verbose {
        let elapsed = start_time.elapsed();
        eprintln!("\nCompleted in {elapsed:.2?}");
        print_stats(&stats);
    }

    Ok(stats)
}

#[allow(
    clippy::too_many_lines,
    clippy::used_underscore_binding,
    clippy::only_used_in_recursion
)]
/// Process a source path and copy/move to destination
///
/// Recursively handles directories and applies conflict resolution strategies.
///
/// # Errors
///
/// Returns an error if the file/folder cannot be copied or moved.
fn process_source(
    source: &Path,
    dest: &Path,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
    _share: Option<&Pool>,
    file_count: &Arc<Mutex<u64>>,
    byte_count: &Arc<Mutex<u64>>,
) -> Result<()> {
    let source_is_dir = source.is_dir();
    let dest_exists = dest.exists();

    // Check limits
    {
        let count = *file_count
            .lock()
            .map_err(|e| NofsError::CopyMove(format!("Lock poisoning: {e}")))?;
        if let Some(limit) = config.limit {
            if count >= limit {
                return Ok(());
            }
        }
    }

    if source_is_dir {
        // Handle directory
        if dest_exists {
            let dest_is_dir = dest.is_dir();
            if !dest_is_dir {
                // Folder over file conflict
                stats.conflicts_resolved.fetch_add(1, Ordering::Relaxed);
                return handle_folder_over_file(dest, source, config, stats);
            }
            // Folder over folder - merge
        } else {
            // Create destination directory
            if !config.simulate {
                fs::create_dir_all(dest)?;
            }
            stats.folders_created.fetch_add(1, Ordering::Relaxed);
        }

        // Recursively process directory contents
        let entries = fs::read_dir(source)?;
        for entry_result in entries {
            let entry = entry_result?;
            let entry_path = entry.path();
            let entry_name = entry.file_name();
            let entry_dest = dest.join(&entry_name);

            if let Err(e) = process_source(
                &entry_path,
                &entry_dest,
                config,
                stats,
                _share,
                file_count,
                byte_count,
            ) {
                eprintln!(
                    "Error processing {}: {}",
                    shell_quote(entry_path.to_string_lossy().as_ref()),
                    e
                );
            }
        }
    } else {
        // Handle file
        // Check extension filter
        if !config.extensions.is_empty() {
            let ext = source.extension().and_then(|s| s.to_str()).unwrap_or("");
            let matches = config
                .extensions
                .iter()
                .any(|e| e.trim_start_matches('.') == ext);
            if !matches {
                return Ok(());
            }
        }

        // Check if destination exists
        if dest_exists {
            let dest_is_dir = dest.is_dir();
            if dest_is_dir {
                // File over folder - apply strategy
                stats.conflicts_resolved.fetch_add(1, Ordering::Relaxed);
                return handle_file_over_folder(
                    source, dest, config, stats, file_count, byte_count,
                );
            }
            // File over file conflict
            stats.conflicts_resolved.fetch_add(1, Ordering::Relaxed);
            return handle_file_over_file(source, dest, config, stats, file_count, byte_count);
        }

        // No conflict - just copy/move
        process_file(source, dest, config, stats, file_count, byte_count)?;
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
/// Process a single file copy/move operation
///
/// Handles size limits, simulate mode, and actual file transfer.
///
/// # Errors
///
/// Returns an error if the file cannot be read or written.
fn process_file(
    source: &Path,
    dest: &Path,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
    file_count: &Arc<Mutex<u64>>,
    byte_count: &Arc<Mutex<u64>>,
) -> Result<()> {
    let file_size = fs::metadata(source)?.len();

    // Check size limit
    {
        let mut bytes = byte_count
            .lock()
            .map_err(|e| NofsError::CopyMove(format!("Lock poisoning: {e}")))?;
        if let Some(limit) = config.size_limit {
            if bytes.checked_add(file_size).is_some_and(|sum| sum > limit) {
                return Ok(());
            }
            *bytes = bytes.saturating_add(file_size);
        }
    }

    // Check file count limit
    {
        let mut count = file_count
            .lock()
            .map_err(|e| NofsError::CopyMove(format!("Lock poisoning: {e}")))?;
        if let Some(limit) = config.limit {
            if *count >= limit {
                return Ok(());
            }
            *count = count.saturating_add(1);
        }
    }

    if config.simulate {
        if config.verbose {
            let action = if config.copy { "copy" } else { "move" };
            eprintln!(
                "[SIMULATE] {} {} -> {} ({})",
                action,
                shell_quote(source.to_string_lossy().as_ref()),
                shell_quote(dest.to_string_lossy().as_ref()),
                format_size(file_size)
            );
        }
        return Ok(());
    }

    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    if config.copy {
        // Copy the file
        copy_file_contents(source, dest)?;
    } else {
        // Move the file (try rename first, fall back to copy+delete)
        if fs::rename(source, dest).is_err() {
            copy_file_contents(source, dest)?;
            fs::remove_file(source)?;
        }
    }

    stats.files_copied.fetch_add(1, Ordering::Relaxed);
    stats.bytes_copied.fetch_add(file_size, Ordering::Relaxed);

    if config.verbose {
        let action = if config.copy { "copy" } else { "move" };
        eprintln!(
            "{} {} -> {} ({})",
            action,
            shell_quote(source.to_string_lossy().as_ref()),
            shell_quote(dest.to_string_lossy().as_ref()),
            format_size(file_size)
        );
    }

    Ok(())
}

/// Copy file contents from source to destination
///
/// Tries reflink first for copy-on-write benefits, falls back to manual copy.
/// Also preserves file permissions.
///
/// # Errors
///
/// Returns an error if the file cannot be read or written.
fn copy_file_contents(source: &Path, dest: &Path) -> Result<()> {
    // Try reflink first for copy-on-write benefits on supported filesystems
    if reflink::reflink(source, dest).is_ok() {
        return Ok(());
    }

    // Fallback to manual copy if reflink is not supported
    let mut src_file = fs::File::open(source)?;
    let mut dst_file = fs::File::create(dest)?;
    io::copy(&mut src_file, &mut dst_file)?;

    // Preserve metadata
    let metadata = fs::metadata(source)?;
    fs::set_permissions(dest, metadata.permissions())?;

    Ok(())
}

/// Handle file-over-file conflict resolution
///
/// Evaluates skip/delete conditions based on hash and size, then applies
/// the required strategy if no conditions match.
///
/// # Errors
///
/// Returns an error if file operations fail.
fn handle_file_over_file(
    source: &Path,
    dest: &Path,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
    file_count: &Arc<Mutex<u64>>,
    byte_count: &Arc<Mutex<u64>>,
) -> Result<()> {
    let strategy = &config.file_over_file;

    // Check optional conditions first
    let src_size = fs::metadata(source)?.len();
    let dest_size = fs::metadata(dest)?.len();

    // Check hash-based conditions if needed
    let hashes_match =
        if strategy.skip_hash || strategy.delete_dest_hash || strategy.delete_src_hash {
            files_match_by_hash(source, dest, stats)?
        } else {
            false
        };

    // Evaluate optional conditions
    if check_skip_conditions(
        strategy,
        hashes_match,
        src_size,
        dest_size,
        config,
        source,
        stats,
    ) {
        return Ok(());
    }

    if check_delete_dest_conditions(
        strategy,
        hashes_match,
        src_size,
        dest_size,
        config,
        dest,
        source,
        stats,
        file_count,
        byte_count,
    )? {
        return Ok(());
    }

    if check_delete_src_conditions(
        strategy,
        hashes_match,
        src_size,
        dest_size,
        config,
        source,
        dest,
        stats,
    )? {
        return Ok(());
    }

    // Apply required fallback
    apply_required_strategy(
        strategy, source, dest, config, stats, file_count, byte_count,
    )
}

/// Check skip conditions for file-over-file conflicts
///
/// Returns true if the file should be skipped based on hash/size/larger/smaller conditions.
#[must_use]
#[allow(clippy::too_many_arguments)]
fn check_skip_conditions(
    strategy: &FileOverFileStrategy,
    hashes_match: bool,
    src_size: u64,
    dest_size: u64,
    config: &CopyConfig,
    source: &Path,
    stats: &Arc<CopyStats>,
) -> bool {
    if strategy.skip_hash && hashes_match {
        if config.verbose {
            eprintln!(
                "Skipping {} (hash matches)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return true;
    }

    if strategy.skip_size && src_size == dest_size {
        if config.verbose {
            eprintln!(
                "Skipping {} (size matches)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return true;
    }

    if strategy.skip_larger && src_size > dest_size {
        if config.verbose {
            eprintln!(
                "Skipping {} (source is larger)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return true;
    }

    if strategy.skip_smaller && src_size < dest_size {
        if config.verbose {
            eprintln!(
                "Skipping {} (source is smaller)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return true;
    }

    false
}

/// Check delete-dest conditions for file-over-file conflicts
///
/// Returns true if the destination was deleted and processing should continue.
///
/// # Errors
///
/// Returns an error if file operations fail.
#[allow(clippy::too_many_arguments)]
fn check_delete_dest_conditions(
    strategy: &FileOverFileStrategy,
    hashes_match: bool,
    src_size: u64,
    dest_size: u64,
    config: &CopyConfig,
    dest: &Path,
    source: &Path,
    stats: &Arc<CopyStats>,
    file_count: &Arc<Mutex<u64>>,
    byte_count: &Arc<Mutex<u64>>,
) -> Result<bool> {
    if strategy.delete_dest_hash && hashes_match {
        if config.verbose {
            eprintln!(
                "Deleting destination {} (hash matches)",
                shell_quote(dest.to_string_lossy().as_ref())
            );
        }
        if !config.simulate {
            fs::remove_file(dest)?;
        }
        return process_file(source, dest, config, stats, file_count, byte_count).map(|()| true);
    }

    if strategy.delete_dest_size && src_size == dest_size {
        if config.verbose {
            eprintln!(
                "Deleting destination {} (size matches)",
                shell_quote(dest.to_string_lossy().as_ref())
            );
        }
        if !config.simulate {
            fs::remove_file(dest)?;
        }
        return process_file(source, dest, config, stats, file_count, byte_count).map(|()| true);
    }

    if strategy.delete_dest_larger && src_size > dest_size {
        if config.verbose {
            eprintln!(
                "Deleting destination {} (source is larger)",
                shell_quote(dest.to_string_lossy().as_ref())
            );
        }
        if !config.simulate {
            fs::remove_file(dest)?;
        }
        return process_file(source, dest, config, stats, file_count, byte_count).map(|()| true);
    }

    if strategy.delete_dest_smaller && src_size < dest_size {
        if config.verbose {
            eprintln!(
                "Deleting destination {} (source is smaller)",
                shell_quote(dest.to_string_lossy().as_ref())
            );
        }
        if !config.simulate {
            fs::remove_file(dest)?;
        }
        return process_file(source, dest, config, stats, file_count, byte_count).map(|()| true);
    }

    Ok(false)
}

/// Check delete-src conditions for file-over-file conflicts
///
/// Returns true if the source was deleted and processing should stop.
///
/// # Errors
///
/// Returns an error if file operations fail.
#[allow(clippy::too_many_arguments)]
fn check_delete_src_conditions(
    strategy: &FileOverFileStrategy,
    hashes_match: bool,
    src_size: u64,
    dest_size: u64,
    config: &CopyConfig,
    source: &Path,
    _dest: &Path,
    stats: &Arc<CopyStats>,
) -> Result<bool> {
    if strategy.delete_src_hash && hashes_match {
        if config.verbose {
            eprintln!(
                "Deleting source {} (hash matches)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        if !config.simulate {
            fs::remove_file(source)?;
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return Ok(true);
    }

    if strategy.delete_src_size && src_size == dest_size {
        if config.verbose {
            eprintln!(
                "Deleting source {} (size matches)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        if !config.simulate {
            fs::remove_file(source)?;
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return Ok(true);
    }

    if strategy.delete_src_larger && src_size > dest_size {
        if config.verbose {
            eprintln!(
                "Deleting source {} (source is larger)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        if !config.simulate {
            fs::remove_file(source)?;
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return Ok(true);
    }

    if strategy.delete_src_smaller && src_size < dest_size {
        if config.verbose {
            eprintln!(
                "Deleting source {} (source is smaller)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        if !config.simulate {
            fs::remove_file(source)?;
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return Ok(true);
    }

    Ok(false)
}

/// Apply the required file-over-file strategy when no optional conditions match
///
/// # Errors
///
/// Returns an error if file operations fail.
fn apply_required_strategy(
    strategy: &FileOverFileStrategy,
    source: &Path,
    dest: &Path,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
    file_count: &Arc<Mutex<u64>>,
    byte_count: &Arc<Mutex<u64>>,
) -> Result<()> {
    match strategy.required {
        FileOverFileMode::Skip => {
            if config.verbose {
                eprintln!(
                    "Skipping {} (strategy: skip)",
                    shell_quote(source.to_string_lossy().as_ref())
                );
            }
            stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        }
        FileOverFileMode::DeleteSrc => {
            if config.verbose {
                eprintln!(
                    "Deleting source {} (strategy: delete-src)",
                    shell_quote(source.to_string_lossy().as_ref())
                );
            }
            if !config.simulate {
                fs::remove_file(source)?;
            }
            stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        }
        FileOverFileMode::DeleteDest => {
            if config.verbose {
                eprintln!(
                    "Replacing {} (strategy: delete-dest)",
                    shell_quote(dest.to_string_lossy().as_ref())
                );
            }
            if !config.simulate {
                fs::remove_file(dest)?;
            }
            return process_file(source, dest, config, stats, file_count, byte_count);
        }
        FileOverFileMode::RenameSrc => {
            let new_dest = get_unique_filename(dest);
            if config.verbose {
                eprintln!(
                    "Renaming source {} -> {} (strategy: rename-src)",
                    shell_quote(source.to_string_lossy().as_ref()),
                    shell_quote(new_dest.to_string_lossy().as_ref())
                );
            }
            return process_file(source, &new_dest, config, stats, file_count, byte_count);
        }
        FileOverFileMode::RenameDest => {
            let renamed_dest = get_unique_filename(dest);
            if config.verbose {
                eprintln!(
                    "Renaming destination {} -> {} (strategy: rename-dest)",
                    shell_quote(dest.to_string_lossy().as_ref()),
                    shell_quote(renamed_dest.to_string_lossy().as_ref())
                );
            }
            if !config.simulate {
                // First rename the existing destination
                fs::rename(dest, &renamed_dest)?;
            }
            // Then copy/move source to original destination
            return process_file(source, dest, config, stats, file_count, byte_count);
        }
    }

    Ok(())
}

/// Handle file-over-folder conflict resolution
///
/// # Errors
///
/// Returns an error if file operations fail.
fn handle_file_over_folder(
    source: &Path,
    dest: &Path,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
    file_count: &Arc<Mutex<u64>>,
    byte_count: &Arc<Mutex<u64>>,
) -> Result<()> {
    match config.file_over_folder {
        FolderConflictMode::Skip => {
            if config.verbose {
                eprintln!(
                    "Skipping file {} (strategy: skip)",
                    shell_quote(source.to_string_lossy().as_ref())
                );
            }
            // File not copied, folder unchanged
        }
        FolderConflictMode::DeleteSrc => {
            if config.verbose {
                eprintln!(
                    "Deleting source file {} (strategy: delete-src)",
                    shell_quote(source.to_string_lossy().as_ref())
                );
            }
            if !config.simulate {
                fs::remove_file(source)?;
            }
        }
        FolderConflictMode::DeleteDest => {
            if config.verbose {
                eprintln!(
                    "Deleting destination folder {} (strategy: delete-dest)",
                    shell_quote(dest.to_string_lossy().as_ref())
                );
            }
            if !config.simulate {
                fs::remove_dir_all(dest)?;
            }
            // Now copy file to original path
            return process_file(source, dest, config, stats, file_count, byte_count);
        }
        FolderConflictMode::RenameSrc => {
            let new_dest = get_unique_filename(dest);
            if config.verbose {
                eprintln!(
                    "Renaming source file {} -> {} (strategy: rename-src)",
                    shell_quote(source.to_string_lossy().as_ref()),
                    shell_quote(new_dest.to_string_lossy().as_ref())
                );
            }
            // Copy file with renamed path into the folder
            return process_file(source, &new_dest, config, stats, file_count, byte_count);
        }
        FolderConflictMode::RenameDest => {
            let renamed_dest = get_unique_folder_name(dest);
            if config.verbose {
                eprintln!(
                    "Renaming destination folder {} -> {} (strategy: rename-dest)",
                    shell_quote(dest.to_string_lossy().as_ref()),
                    shell_quote(renamed_dest.to_string_lossy().as_ref())
                );
            }
            if !config.simulate {
                fs::rename(dest, &renamed_dest)?;
            }
            // Copy file to original path
            return process_file(source, dest, config, stats, file_count, byte_count);
        }
        FolderConflictMode::Merge => {
            // Move file into folder as folder/filename
            let file_name = source.file_name().unwrap_or(source.as_os_str());
            let new_dest = dest.join(file_name);
            if config.verbose {
                eprintln!(
                    "Merging file {} into folder {}",
                    shell_quote(source.to_string_lossy().as_ref()),
                    shell_quote(dest.to_string_lossy().as_ref())
                );
            }
            return process_file(source, &new_dest, config, stats, file_count, byte_count);
        }
    }

    Ok(())
}

/// Handle folder-over-file conflict resolution
///
/// # Errors
///
/// Returns an error if file operations fail.
fn handle_folder_over_file(
    dest: &Path,
    source: &Path,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
) -> Result<()> {
    match config.folder_over_file {
        FolderConflictMode::Skip => {
            if config.verbose {
                eprintln!(
                    "Skipping folder {} (strategy: skip)",
                    shell_quote(source.to_string_lossy().as_ref())
                );
            }
        }
        FolderConflictMode::DeleteSrc => {
            if config.verbose {
                eprintln!(
                    "Deleting source folder {} (strategy: delete-src)",
                    shell_quote(source.to_string_lossy().as_ref())
                );
            }
            if !config.simulate {
                fs::remove_dir_all(source)?;
            }
        }
        FolderConflictMode::DeleteDest => {
            if config.verbose {
                eprintln!(
                    "Deleting destination file {} (strategy: delete-dest)",
                    shell_quote(dest.to_string_lossy().as_ref())
                );
            }
            if !config.simulate {
                fs::remove_file(dest)?;
            }
            // Now create the folder and copy contents
            if !config.simulate {
                fs::create_dir_all(dest)?;
            }
            stats.folders_created.fetch_add(1, Ordering::Relaxed);
            return process_source_contents(source, dest, config, stats);
        }
        FolderConflictMode::RenameSrc => {
            // Rename the source folder conceptually by copying to renamed path
            let new_dest = get_unique_folder_name(dest);
            if config.verbose {
                eprintln!(
                    "Using renamed folder path {} (strategy: rename-src)",
                    shell_quote(new_dest.to_string_lossy().as_ref())
                );
            }
            if !config.simulate {
                fs::create_dir_all(&new_dest)?;
            }
            stats.folders_created.fetch_add(1, Ordering::Relaxed);
            // Process source contents into new destination
            return process_source_contents(source, &new_dest, config, stats);
        }
        FolderConflictMode::RenameDest => {
            let renamed_dest = get_unique_folder_name(dest);
            if config.verbose {
                eprintln!(
                    "Renaming destination file {} -> {} (strategy: rename-dest)",
                    shell_quote(dest.to_string_lossy().as_ref()),
                    shell_quote(renamed_dest.to_string_lossy().as_ref())
                );
            }
            if !config.simulate {
                fs::rename(dest, &renamed_dest)?;
                fs::create_dir_all(dest)?;
            }
            stats.folders_created.fetch_add(1, Ordering::Relaxed);
            return process_source_contents(source, dest, config, stats);
        }
        FolderConflictMode::Merge => {
            // This case shouldn't happen (folder over file merge doesn't make sense)
            // Fall back to rename-dest behavior
            let renamed_dest = get_unique_folder_name(dest);
            if config.verbose {
                eprintln!(
                    "Renaming destination file {} -> {} (strategy: merge fallback)",
                    shell_quote(dest.to_string_lossy().as_ref()),
                    shell_quote(renamed_dest.to_string_lossy().as_ref())
                );
            }
            if !config.simulate {
                fs::rename(dest, &renamed_dest)?;
                fs::create_dir_all(dest)?;
            }
            stats.folders_created.fetch_add(1, Ordering::Relaxed);
            return process_source_contents(source, dest, config, stats);
        }
    }

    Ok(())
}

/// Process directory contents recursively
///
/// # Errors
///
/// Returns an error if directory cannot be read or entries cannot be processed.
fn process_source_contents(
    source: &Path,
    dest: &Path,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
) -> Result<()> {
    let entries = fs::read_dir(source)?;
    for entry_result in entries {
        let entry = entry_result?;
        let entry_path = entry.path();
        let entry_name = entry.file_name();
        let entry_dest = dest.join(&entry_name);

        if let Err(e) = process_source(
            &entry_path,
            &entry_dest,
            config,
            stats,
            None, // share not needed for already-resolved paths
            &Arc::new(Mutex::new(0u64)),
            &Arc::new(Mutex::new(0u64)),
        ) {
            eprintln!(
                "Error processing {}: {}",
                shell_quote(entry_path.to_string_lossy().as_ref()),
                e
            );
        }
    }
    Ok(())
}

/// Generate a unique filename by appending _N suffix if file exists
///
/// # Returns
///
/// Returns a path with a unique filename that doesn't exist on disk.
fn get_unique_filename(base: &Path) -> PathBuf {
    if !base.exists() {
        return base.to_path_buf();
    }

    let dir = base.parent().unwrap_or_else(|| Path::new("."));
    let file_stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let extension = base.extension().and_then(|s| s.to_str()).unwrap_or("");

    // Check if file already has a _N suffix
    #[allow(clippy::arithmetic_side_effects)]
    let (base_name, start_idx) = file_stem.rfind('_').map_or((file_stem, 1), |idx| {
        let suffix = &file_stem[idx + 1..];
        suffix
            .parse::<u32>()
            .map_or((file_stem, 1), |num| (&file_stem[..idx], num + 1))
    });

    for i in start_idx.. {
        let new_name = if extension.is_empty() {
            format!("{base_name}_{i}")
        } else {
            format!("{base_name}_{i}.{extension}")
        };
        let new_path = dir.join(&new_name);
        if !new_path.exists() {
            return new_path;
        }
    }

    // Fallback (shouldn't reach here)
    base.to_path_buf().with_extension(format!(
        "{}.{}",
        base.extension().unwrap_or_default().to_string_lossy(),
        "conflict"
    ))
}

/// Generate a unique folder name by appending _N suffix if folder exists
///
/// # Returns
///
/// Returns a path with a unique folder name that doesn't exist on disk.
fn get_unique_folder_name(base: &Path) -> PathBuf {
    if !base.exists() {
        return base.to_path_buf();
    }

    let dir = base.parent().unwrap_or_else(|| Path::new("."));
    let folder_name = base
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("folder");

    // Check if folder already has a _N suffix
    #[allow(clippy::arithmetic_side_effects)]
    let (base_name, start_idx) = folder_name.rfind('_').map_or((folder_name, 1), |idx| {
        let suffix = &folder_name[idx + 1..];
        suffix
            .parse::<u32>()
            .map_or((folder_name, 1), |num| (&folder_name[..idx], num + 1))
    });

    for i in start_idx.. {
        let new_name = format!("{base_name}_{i}");
        let new_path = dir.join(&new_name);
        if !new_path.exists() {
            return new_path;
        }
    }

    base.to_path_buf().with_extension("conflict")
}

/// Check if two files match by comparing their hashes
///
/// # Errors
///
/// Returns an error if either file cannot be read.
fn files_match_by_hash(source: &Path, dest: &Path, stats: &CopyStats) -> Result<bool> {
    // Use sample hashing for efficiency
    let src_hash = sample_hash(source, stats)?;
    let dest_hash = sample_hash(dest, stats)?;
    Ok(src_hash == dest_hash)
}

/// Compute a hash of a file, using sampling for large files
///
/// # Errors
///
/// Returns an error if the file cannot be read.
fn sample_hash(path: &Path, stats: &CopyStats) -> Result<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let metadata = fs::metadata(path)?;
    let size = metadata.len();

    // For small files, hash the entire content
    if size <= 640 * 1024 {
        let mut hasher = DefaultHasher::new();
        fs::read(path)?.hash(&mut hasher);
        stats.full_hashes.fetch_add(1, Ordering::Relaxed);
        return Ok(format!("{:x}", hasher.finish()));
    }

    // For larger files, sample at multiple positions
    let mut file = fs::File::open(path)?;
    let mut hasher = DefaultHasher::new();
    let chunk_size: u64 = 64 * 1024;
    let num_samples: u64 = 10;

    stats.sample_hashes.fetch_add(1, Ordering::Relaxed);

    #[allow(
        clippy::integer_division,
        clippy::cast_possible_truncation,
        clippy::as_conversions,
        clippy::indexing_slicing
    )]
    for i in 0..num_samples {
        let pos = size.saturating_mul(i) / num_samples;
        file.seek(io::SeekFrom::Start(pos))?;
        let mut buf = vec![0u8; chunk_size as usize];
        let bytes_read = file.read(&mut buf)?;
        buf[..bytes_read].hash(&mut hasher);
    }

    Ok(format!("{:x}", hasher.finish()))
}

#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::float_arithmetic,
    clippy::as_conversions
)]
pub fn format_size(bytes: u64) -> String {
    const KB: u64 = 1000;
    const MB: u64 = KB * 1000;
    const GB: u64 = MB * 1000;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Quote a string for shell usage
///
/// # Returns
///
/// Returns the string wrapped in single quotes with proper escaping.
fn shell_quote<S: AsRef<str>>(s: S) -> String {
    let s_ref = s.as_ref();
    if s_ref.is_empty() {
        return "''".to_string();
    }
    if s_ref
        .chars()
        .all(|c| c.is_alphanumeric() || "!@%_+=:,./-".contains(c))
    {
        return format!("'{s_ref}'");
    }
    format!("'{}'", s_ref.replace('\'', "'\\''"))
}

/// Print copy/move statistics
fn print_stats(stats: &CopyStats) {
    eprintln!("\nSummary:");
    eprintln!(
        "  {} files copied",
        stats.files_copied.load(Ordering::Relaxed)
    );
    eprintln!(
        "  {} folders created",
        stats.folders_created.load(Ordering::Relaxed)
    );
    eprintln!(
        "  {} bytes copied",
        format_size(stats.bytes_copied.load(Ordering::Relaxed))
    );
    eprintln!(
        "  {} files skipped",
        stats.files_skipped.load(Ordering::Relaxed)
    );
    eprintln!(
        "  {} conflicts resolved",
        stats.conflicts_resolved.load(Ordering::Relaxed)
    );
    let errors = stats.errors.load(Ordering::Relaxed);
    if errors > 0 {
        eprintln!("  {errors} errors");
    }
}
