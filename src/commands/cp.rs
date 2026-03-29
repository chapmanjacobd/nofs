//! Copy command for nofs
//!
//! Implements cp/mv-like functionality with support for nofs context paths,
//! conflict resolution strategies, and parallel operations.

use crate::branch::Branch;
use crate::cache::OperationCache;
use crate::error::{NofsError, Result};
use crate::pool::Pool;
use crate::utils;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[cfg(unix)]
use std::ffi::OsStr;
#[cfg(unix)]
use std::os::unix::ffi::OsStrExt;

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
fn resolve_path(
    path_str: &str,
    share: Option<&Pool>,
    for_create: bool,
    cache: &OperationCache,
) -> Result<ResolvedPath> {
    if let Some(colon_idx) = path_str.find(':') {
        let potential_prefix = &path_str[..colon_idx];
        // Check for path separators (both Unix / and Windows \) to distinguish
        // share paths (e.g., "media:/movies") from Windows drive letters (e.g., "C:\")
        if !potential_prefix.contains('/') && !potential_prefix.contains('\\') {
            let share_name = potential_prefix;
            let relative_path = &path_str[colon_idx + 1..];

            if let Some(pool) = share {
                if pool.name == share_name {
                    let branch = if for_create {
                        // For create operations, use policy-based selection
                        select_branch_for_create(pool, Some(relative_path.as_ref()), cache)?
                    } else {
                        // For existing files, find the branch containing the file
                        select_branch_for_read(pool, Path::new(relative_path), cache)?
                    };

                    // Get branch index using O(1) HashMap lookup
                    let branch_idx = pool
                        .get_branch_index(&branch.path)
                        .map_err(|e| NofsError::CopyMove(format!("Failed to resolve branch: {e}")))?;

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
    cache: &'a OperationCache,
) -> Result<&'a Branch> {
    use crate::policy::CreatePolicy;

    let policy_executor = CreatePolicy::with_cache(&pool.branches, pool.minfreespace, cache);
    policy_executor
        .select(pool.create_policy, relative_path)
        .map_err(|e| NofsError::CopyMove(format!("No suitable branch: {e}")))
}

/// Select a branch for read operations (finds the branch containing the file)
fn select_branch_for_read<'a>(pool: &'a Pool, relative_path: &Path, cache: &'a OperationCache) -> Result<&'a Branch> {
    // For reads, find the branch that has the file
    // Prefer RW branches, then NC, then RO
    for branch in &pool.branches {
        if branch.path_exists_cached(relative_path, cache) {
            return Ok(branch);
        }
    }
    // File not found in any branch
    Err(NofsError::CopyMove(format!("File not found in share '{}'", pool.name)))
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
#[allow(clippy::arithmetic_side_effects)]
fn resolve_dest_path(
    dest_str: &str,
    share: Option<&Pool>,
    source_branch_index: Option<usize>,
    cache: &OperationCache,
) -> Result<PathBuf> {
    // Check for share prefix format: "share_name:relative/path"
    let Some(colon_idx) = dest_str.find(':') else {
        // No share prefix, return as-is
        return Ok(PathBuf::from(dest_str));
    };

    let potential_prefix = &dest_str[..colon_idx];
    // If prefix contains path separators, it's not a share name (likely a Windows path like C:)
    if potential_prefix.contains('/') || potential_prefix.contains('\\') {
        return Ok(PathBuf::from(dest_str));
    }

    let share_name = potential_prefix;
    let relative_path = &dest_str[colon_idx + 1..];
    let Some(pool) = share else {
        return Err(NofsError::CopyMove(format!(
            "Share '{share_name}' not found or has no branches"
        )));
    };

    if pool.name != share_name {
        return Err(NofsError::CopyMove(format!(
            "Share '{share_name}' not found or has no branches"
        )));
    }

    // Try to use the same branch as the source (for efficient same-branch operations)
    if let Some(src_idx) = source_branch_index {
        if let Some(branch) = pool.branches.get(src_idx) {
            if branch.can_create() {
                return Ok(branch.path.join(relative_path));
            }
        }
    }

    // Fallback: use policy-based branch selection
    let branch = select_branch_for_create(pool, Some(relative_path.as_ref()), cache)?;
    Ok(branch.path.join(relative_path))
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
    /// Skip the source item (don't copy/move it)
    Skip,
    /// Rename the source item with a _N suffix
    RenameSrc,
    /// Rename the destination item with a _N suffix
    RenameDest,
    /// Delete the source item
    DeleteSrc,
    /// Delete the destination item
    DeleteDest,
    /// Merge: for folder-over-file, rename file and create folder; for file-over-folder, place file inside folder
    Merge,
}

/// Attribute to compare in a conflict resolution rule
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribute {
    /// Compare file hashes (content comparison)
    Hash,
    /// Compare file sizes in bytes
    Size,
    /// Compare file modification times
    Modified,
    /// Compare file creation times
    Created,
}

/// Comparison operator in a conflict resolution rule
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    /// Check if attributes are equal
    Equal,
    /// Check if source attribute is greater than destination
    Greater,
    /// Check if source attribute is less than destination
    Less,
}

/// Target of comparison in a conflict resolution rule
///
/// Determines which file's attribute is being compared against the source.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Compare against the source file's attribute
    Src,
    /// Compare against the destination file's attribute
    Dest,
}

/// Action to take when a rule matches
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleAction {
    /// Skip the operation (don't copy/move the file)
    Skip,
    /// Delete the source file and skip the operation
    DeleteSrc,
    /// Delete the destination file before proceeding with the operation
    DeleteDest,
}

/// A single rule for file-over-file conflict resolution
///
/// Rules are evaluated in order. When a rule's condition matches, its action
/// is executed immediately and no further rules are checked.
///
/// # Example
///
/// A rule that skips copying if file hashes match:
/// ```
/// Rule {
///     action: RuleAction::Skip,
///     attribute: Attribute::Hash,
///     comparison: Comparison::Equal,
///     target: Target::Dest,
/// }
/// ```
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct Rule {
    /// The action to execute when this rule's condition matches
    pub action: RuleAction,
    /// The file attribute to compare (hash, size, modified time, or created time)
    pub attribute: Attribute,
    /// The comparison operator (equal, greater than, or less than)
    pub comparison: Comparison,
    /// Which file's attribute to compare against (source or destination)
    pub target: Target,
}

impl Rule {
    #[must_use]
    pub fn display(&self) -> String {
        match self.attribute {
            Attribute::Hash => "hashes match".to_string(),
            Attribute::Size => match self.comparison {
                Comparison::Equal => "sizes match".to_string(),
                Comparison::Greater => "source is larger than destination".to_string(),
                Comparison::Less => "source is smaller than destination".to_string(),
            },
            Attribute::Modified => match self.comparison {
                Comparison::Equal => "modification times match".to_string(),
                Comparison::Greater => "source is newer than destination".to_string(),
                Comparison::Less => "source is older than destination".to_string(),
            },
            Attribute::Created => match self.comparison {
                Comparison::Equal => "creation times match".to_string(),
                Comparison::Greater => "source was created more recently than destination".to_string(),
                Comparison::Less => "source was created earlier than destination".to_string(),
            },
        }
    }
}

/// File-over-file strategy with optional conditions
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct FileOverFileStrategy {
    pub rules: Vec<Rule>,
    pub required: FileOverFileMode,
}

impl Default for FileOverFileStrategy {
    fn default() -> Self {
        Self {
            rules: Vec::new(),
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
    let mut rules = Vec::new();
    let parts: Vec<&str> = spec.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(FileOverFileStrategy {
            rules,
            required: FileOverFileMode::RenameDest,
        });
    }

    // Last part is the required mode
    let required_str = parts.last().copied().unwrap_or("skip");
    let required = match required_str {
        "skip" => FileOverFileMode::Skip,
        "rename-src" => FileOverFileMode::RenameSrc,
        "rename-dest" => FileOverFileMode::RenameDest,
        "delete-src" => FileOverFileMode::DeleteSrc,
        "delete-dest" => FileOverFileMode::DeleteDest,
        _ => return Err(NofsError::Parse(format!("Unknown file-over-file mode: {required_str}"))),
    };

    // Previous parts are optional conditions - convert to rules
    for opt in parts.iter().take(parts.len().saturating_sub(1)) {
        let rule = parse_rule_token(opt)?;
        rules.push(rule);
    }

    Ok(FileOverFileStrategy { rules, required })
}

/// Parse folder conflict mode from string
///
/// # Errors
///
/// Returns an error if the mode string is not recognized.
pub fn parse_folder_conflict_mode(s: &str) -> Result<FolderConflictMode> {
    match s.to_lowercase().as_str() {
        "skip" => Ok(FolderConflictMode::Skip),
        "rename-src" => Ok(FolderConflictMode::RenameSrc),
        "rename-dest" => Ok(FolderConflictMode::RenameDest),
        "delete-src" => Ok(FolderConflictMode::DeleteSrc),
        "delete-dest" => Ok(FolderConflictMode::DeleteDest),
        "merge" => Ok(FolderConflictMode::Merge),
        _ => Err(NofsError::Parse(format!("Unknown folder conflict mode: {s}"))),
    }
}

/// Helper to create a rule with hash attribute
const fn make_hash_rule(action: RuleAction) -> Rule {
    Rule {
        action,
        attribute: Attribute::Hash,
        comparison: Comparison::Equal,
        target: Target::Dest,
    }
}

/// Helper to create a rule with size attribute
const fn make_size_rule(action: RuleAction, comparison: Comparison, target: Target) -> Rule {
    Rule {
        action,
        attribute: Attribute::Size,
        comparison,
        target,
    }
}

/// Helper to create a rule with modified attribute
const fn make_modified_rule(action: RuleAction, comparison: Comparison, target: Target) -> Rule {
    Rule {
        action,
        attribute: Attribute::Modified,
        comparison,
        target,
    }
}

/// Helper to create a rule with created attribute
const fn make_created_rule(action: RuleAction, comparison: Comparison, target: Target) -> Rule {
    Rule {
        action,
        attribute: Attribute::Created,
        comparison,
        target,
    }
}

/// Parse a single rule token into a Rule struct
///
/// # Errors
///
/// Returns an error if the token is not recognized.
#[allow(clippy::too_many_lines)]
fn parse_rule_token(token: &str) -> Result<Rule> {
    match token {
        // Skip rules
        "skip-hash" => Ok(make_hash_rule(RuleAction::Skip)),
        "skip-size" => Ok(make_size_rule(RuleAction::Skip, Comparison::Equal, Target::Dest)),
        "skip-larger" => Ok(make_size_rule(RuleAction::Skip, Comparison::Greater, Target::Src)),
        "skip-smaller" => Ok(make_size_rule(RuleAction::Skip, Comparison::Less, Target::Src)),
        "skip-modified-newer" => Ok(make_modified_rule(RuleAction::Skip, Comparison::Greater, Target::Src)),
        "skip-modified-older" => Ok(make_modified_rule(RuleAction::Skip, Comparison::Less, Target::Src)),
        "skip-created-newer" => Ok(make_created_rule(RuleAction::Skip, Comparison::Greater, Target::Src)),
        "skip-created-older" => Ok(make_created_rule(RuleAction::Skip, Comparison::Less, Target::Src)),
        // Delete-dest rules
        "delete-dest-hash" => Ok(make_hash_rule(RuleAction::DeleteDest)),
        "delete-dest-size" => Ok(make_size_rule(RuleAction::DeleteDest, Comparison::Equal, Target::Dest)),
        "delete-dest-larger" => Ok(make_size_rule(RuleAction::DeleteDest, Comparison::Greater, Target::Src)),
        "delete-dest-smaller" => Ok(make_size_rule(RuleAction::DeleteDest, Comparison::Less, Target::Src)),
        "delete-dest-modified-newer" => Ok(make_modified_rule(
            RuleAction::DeleteDest,
            Comparison::Greater,
            Target::Src,
        )),
        "delete-dest-modified-older" => Ok(make_modified_rule(
            RuleAction::DeleteDest,
            Comparison::Less,
            Target::Src,
        )),
        "delete-dest-created-newer" => Ok(make_created_rule(
            RuleAction::DeleteDest,
            Comparison::Greater,
            Target::Src,
        )),
        "delete-dest-created-older" => Ok(make_created_rule(RuleAction::DeleteDest, Comparison::Less, Target::Src)),
        // Delete-src rules
        "delete-src-hash" => Ok(make_hash_rule(RuleAction::DeleteSrc)),
        "delete-src-size" => Ok(make_size_rule(RuleAction::DeleteSrc, Comparison::Equal, Target::Dest)),
        "delete-src-larger" => Ok(make_size_rule(RuleAction::DeleteSrc, Comparison::Greater, Target::Src)),
        "delete-src-smaller" => Ok(make_size_rule(RuleAction::DeleteSrc, Comparison::Less, Target::Src)),
        "delete-src-modified-newer" => Ok(make_modified_rule(
            RuleAction::DeleteSrc,
            Comparison::Greater,
            Target::Src,
        )),
        "delete-src-modified-older" => Ok(make_modified_rule(RuleAction::DeleteSrc, Comparison::Less, Target::Src)),
        "delete-src-created-newer" => Ok(make_created_rule(
            RuleAction::DeleteSrc,
            Comparison::Greater,
            Target::Src,
        )),
        "delete-src-created-older" => Ok(make_created_rule(RuleAction::DeleteSrc, Comparison::Less, Target::Src)),
        _ => Err(NofsError::Parse(format!("Unknown file-over-file option: {token}"))),
    }
}

/// Metadata for comparing source and destination files
#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct FileComparison {
    pub hashes_match: bool,
    pub source_size: u64,
    pub dest_size: u64,
    pub source_modified: Option<u64>,
    pub dest_modified: Option<u64>,
    pub source_created: Option<u64>,
    pub dest_created: Option<u64>,
}

/// Evaluate a single rule against file metadata
///
/// Returns true if the rule condition matches.
#[must_use]
pub(crate) fn evaluate_rule(rule: &Rule, cmp: &FileComparison) -> bool {
    match rule.attribute {
        Attribute::Hash => {
            // For hash, we only have a boolean match, not a numeric value
            // The comparison is always "Equal" for hash
            rule.comparison == Comparison::Equal && cmp.hashes_match
        }
        Attribute::Size => match rule.comparison {
            Comparison::Equal => cmp.source_size == cmp.dest_size,
            Comparison::Greater => cmp.source_size > cmp.dest_size,
            Comparison::Less => cmp.source_size < cmp.dest_size,
        },
        Attribute::Modified => cmp_opt(cmp.source_modified, cmp.dest_modified, rule.comparison),
        Attribute::Created => cmp_opt(cmp.source_created, cmp.dest_created, rule.comparison),
    }
}

/// Compare two `Option<T>` values using the specified comparison operator.
///
/// Returns `true` if the comparison holds, `false` if either value is `None`
/// or the comparison fails.
#[must_use]
fn cmp_opt<T: Ord>(a: Option<T>, b: Option<T>, cmp: Comparison) -> bool {
    let (Some(a_val), Some(b_val)) = (a, b) else {
        return false;
    };
    match cmp {
        Comparison::Equal => a_val == b_val,
        Comparison::Greater => a_val > b_val,
        Comparison::Less => a_val < b_val,
    }
}

/// Execute a rule action (Skip, `DeleteSrc`, `DeleteDest`) with common logic.
///
/// Handles logging, stat increment, and the actual delete operation.
///
/// # Errors
///
/// Returns an error if file deletion fails.
fn execute_action(
    action: RuleAction,
    source: &Path,
    dest: &Path,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
    reason: &str,
) -> Result<()> {
    match action {
        RuleAction::Skip => {
            if config.verbose {
                eprintln!(
                    "Skipping {} ({})",
                    shell_quote(source.to_string_lossy().as_ref()),
                    reason
                );
            }
            stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        }
        RuleAction::DeleteSrc => {
            if config.verbose {
                eprintln!(
                    "Deleting source {} ({})",
                    shell_quote(source.to_string_lossy().as_ref()),
                    reason
                );
            }
            if !config.dry_run {
                fs::remove_file(source)?;
            }
            stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        }
        RuleAction::DeleteDest => {
            if config.verbose {
                eprintln!(
                    "Deleting destination {} ({})",
                    shell_quote(dest.to_string_lossy().as_ref()),
                    reason
                );
            }
            if !config.dry_run {
                fs::remove_file(dest)?;
            }
        }
    }
    Ok(())
}

/// Result of evaluating rules
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleResult {
    Skip,
    DeleteSrc,
    DeleteDest,
    NoMatch,
}

/// Evaluate all rules and return the first matching action
///
/// # Errors
///
/// Returns an error if file operations fail during rule evaluation.
#[allow(clippy::too_many_arguments)]
fn evaluate_rules(
    strategy: &FileOverFileStrategy,
    hashes_match: bool,
    source_size: u64,
    dest_size: u64,
    source_modified: Option<u64>,
    dest_modified: Option<u64>,
    source_created: Option<u64>,
    dest_created: Option<u64>,
    config: &CopyConfig,
    source: &Path,
    dest: &Path,
    stats: &Arc<CopyStats>,
) -> Result<RuleResult> {
    let cmp = FileComparison {
        hashes_match,
        source_size,
        dest_size,
        source_modified,
        dest_modified,
        source_created,
        dest_created,
    };
    for rule in &strategy.rules {
        if evaluate_rule(rule, &cmp) {
            let reason = rule.display();
            match rule.action {
                RuleAction::Skip => {
                    execute_action(RuleAction::Skip, source, dest, config, stats, &reason)?;
                    return Ok(RuleResult::Skip);
                }
                RuleAction::DeleteDest => {
                    execute_action(RuleAction::DeleteDest, source, dest, config, stats, &reason)?;
                    return process_file(source, dest, config, stats).map(|()| RuleResult::DeleteDest);
                }
                RuleAction::DeleteSrc => {
                    execute_action(RuleAction::DeleteSrc, source, dest, config, stats, &reason)?;
                    return Ok(RuleResult::DeleteSrc);
                }
            }
        }
    }
    Ok(RuleResult::NoMatch)
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
    pub files_reserved: AtomicU64,
    pub bytes_reserved: AtomicU64,
}

/// Copy operation configuration
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct CopyConfig {
    pub is_copy: bool,  // true = copy, false = move
    pub dry_run: bool,  // dry-run mode (simulate)
    pub workers: usize, // number of parallel workers
    pub verbose: bool,  // verbose output
    pub file_over_file: FileOverFileStrategy,
    pub file_over_folder: FolderConflictMode,
    pub folder_over_file: FolderConflictMode,
    pub extensions: Vec<String>,  // filter by extension
    pub exclude: Vec<String>,     // exclude patterns
    pub include: Vec<String>,     // include patterns
    pub limit: Option<u64>,       // limit number of files
    pub size_limit: Option<u64>,  // limit total size in bytes
    pub size: Option<SizeFilter>, // filter by individual file size
}

/// Size filter for individual files
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct SizeFilter {
    pub min: Option<u64>, // minimum size in bytes
    pub max: Option<u64>, // maximum size in bytes
}

impl Default for CopyConfig {
    fn default() -> Self {
        Self {
            is_copy: true,
            dry_run: false,
            // Default of 4 workers balances parallelism with overhead.
            // This works well for most systems (typically 4+ cores) while
            // avoiding excessive thread contention on smaller machines.
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
            size: None,
        }
    }
}

/// Execute the copy command
///
/// # Errors
///
/// Returns an error if sources are empty or if destination is invalid.
///
/// # Panics
///
/// May panic if mutex poisoning occurs during worker thread synchronization.
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
        return Err(NofsError::CopyMove("At least one source is required".to_string()));
    }

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    // Resolve and validate destination
    let (dest_path, dest_exists, dest_is_dir) = resolve_and_validate_dest(destination, share, &cache)?;

    // Validate multiple sources require directory destination
    if sources.len() > 1 && dest_exists && !dest_is_dir {
        return Err(NofsError::CopyMove(
            "Destination must be a directory when copying multiple sources".to_string(),
        ));
    }

    // Create destination directory if needed for multiple sources
    if !config.dry_run && !dest_exists && sources.len() > 1 {
        fs::create_dir_all(&dest_path)?;
    }

    // Process sources with worker pool
    process_sources_parallel(sources, destination, share, &cache, config, &stats)?;

    if config.verbose {
        let elapsed = start_time.elapsed();
        eprintln!("\nCompleted in {elapsed:.2?}");
        print_stats(&stats);
    }

    Ok(stats)
}

/// Resolve and validate destination path
///
/// # Errors
///
/// Returns an error if path resolution fails.
fn resolve_and_validate_dest(
    destination: &str,
    share: Option<&Pool>,
    cache: &OperationCache,
) -> Result<(PathBuf, bool, bool)> {
    let dest_path = resolve_path(destination, share, true, cache)?.path;
    let dest_exists = dest_path.exists();
    let dest_is_dir = dest_exists && dest_path.is_dir();
    Ok((dest_path, dest_exists, dest_is_dir))
}

/// Process source files using a worker pool
///
/// # Errors
///
/// Returns an error if work items cannot be processed.
///
/// # Panics
///
/// May panic if mutex poisoning occurs.
fn process_sources_parallel(
    sources: &[String],
    destination: &str,
    share: Option<&Pool>,
    cache: &OperationCache,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
) -> Result<()> {
    let workers = config.workers;

    // Create a channel for work items
    let (tx, work_channel) = std::sync::mpsc::channel::<(PathBuf, PathBuf)>();
    let work_rx = Arc::new(std::sync::Mutex::new(work_channel));

    // Spawn worker threads using scoped threads for automatic joining
    std::thread::scope(|s| -> Result<()> {
        spawn_workers(workers, &work_rx, stats, config, s);
        dispatch_work_items(sources, destination, share, cache, &tx, stats)?;
        drop(tx);
        Ok(())
    })?;

    Ok(())
}

/// Spawn worker threads to process work items
fn spawn_workers<'scope, 'a>(
    workers: usize,
    work_rx: &'a Arc<std::sync::Mutex<std::sync::mpsc::Receiver<(PathBuf, PathBuf)>>>,
    stats: &'a Arc<CopyStats>,
    config: &'a CopyConfig,
    scope: &'scope std::thread::Scope<'scope, 'a>,
) {
    for _ in 0..workers {
        let work_rx_clone = Arc::clone(work_rx);
        let stats_clone = Arc::clone(stats);
        let config_clone = config.clone();

        scope.spawn(move || loop {
            let work = {
                let Ok(guard) = work_rx_clone.lock() else {
                    // Mutex was poisoned (a worker thread panicked)
                    eprintln!("Worker thread: mutex poisoned, shutting down");
                    stats_clone.errors.fetch_add(1, Ordering::Relaxed);
                    break;
                };
                guard.recv()
            };

            match work {
                Ok((source_path, final_dest)) => {
                    if let Err(e) = process_source(&source_path, &final_dest, &config_clone, &stats_clone) {
                        eprintln!("Error processing {}: {}", source_path.display(), e);
                        stats_clone.errors.fetch_add(1, Ordering::Relaxed);
                    }
                }
                Err(_) => break,
            }
        });
    }
}

/// Prepare and dispatch work items to workers
///
/// # Errors
///
/// Returns an error if work items cannot be sent.
fn dispatch_work_items(
    sources: &[String],
    destination: &str,
    share: Option<&Pool>,
    cache: &OperationCache,
    tx: &std::sync::mpsc::Sender<(PathBuf, PathBuf)>,
    stats: &Arc<CopyStats>,
) -> Result<()> {
    let dest_exists = destination_exists(destination, share, cache)?;
    let dest_is_dir = dest_exists && PathBuf::from(destination).exists() && PathBuf::from(destination).is_dir();

    for source in sources {
        let source_resolved = resolve_path(source, share, false, cache)?;
        let source_path = source_resolved.path;
        let source_branch_index = source_resolved.branch_index;

        if !source_path.exists() {
            eprintln!("Source {} does not exist", shell_quote(source));
            stats.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        let final_dest = compute_final_dest(
            &source_path,
            destination,
            share,
            source_branch_index,
            cache,
            sources.len(),
            dest_exists,
            dest_is_dir,
        )?;

        tx.send((source_path, final_dest))
            .map_err(|e| NofsError::CopyMove(format!("Failed to send work item: {e}")))?;
    }

    Ok(())
}

/// Check if destination exists
fn destination_exists(destination: &str, share: Option<&Pool>, cache: &OperationCache) -> Result<bool> {
    Ok(resolve_path(destination, share, true, cache)?.path.exists())
}

/// Compute final destination path for a source
///
/// # Errors
///
/// Returns an error if path resolution fails.
#[allow(clippy::too_many_arguments)]
fn compute_final_dest(
    source_path: &Path,
    destination: &str,
    share: Option<&Pool>,
    source_branch_index: Option<usize>,
    cache: &OperationCache,
    sources_len: usize,
    dest_exists: bool,
    dest_is_dir: bool,
) -> Result<PathBuf> {
    if sources_len > 1 || (dest_exists && dest_is_dir) {
        let source_name = source_path
            .file_name()
            .unwrap_or(source_path.as_os_str())
            .to_os_string();
        let dest_base = resolve_dest_path(destination, share, source_branch_index, cache)?;
        Ok(dest_base.join(source_name))
    } else {
        resolve_dest_path(destination, share, source_branch_index, cache)
    }
}

#[allow(
    clippy::too_many_lines,
    clippy::used_underscore_binding,
    clippy::only_used_in_recursion
)]
/// Process a source path and copy/move to destination
///
/// Recursively handles directories and applies conflict resolution strategies.
/// Note: This runs in a worker thread from the pool, so recursive directory
/// processing is done sequentially to avoid thread explosion.
///
/// # Errors
///
/// Returns an error if the file/folder cannot be copied or moved.
fn process_source(source: &Path, dest: &Path, config: &CopyConfig, stats: &Arc<CopyStats>) -> Result<()> {
    let source_is_dir = source.is_dir();
    let dest_exists = dest.exists();

    // Check limits
    if let Some(limit) = config.limit {
        if stats.files_reserved.load(Ordering::Relaxed) >= limit {
            return Ok(());
        }
    }

    if source_is_dir {
        process_directory(source, dest, config, stats)
    } else {
        process_file_source(source, dest, config, stats, dest_exists)
    }
}

/// Process a directory source
///
/// # Errors
///
/// Returns an error if the directory cannot be processed.
fn process_directory(source: &Path, dest: &Path, config: &CopyConfig, stats: &Arc<CopyStats>) -> Result<()> {
    let dest_exists = dest.exists();

    if dest_exists {
        if !dest.is_dir() {
            stats.conflicts_resolved.fetch_add(1, Ordering::Relaxed);
            return handle_folder_over_file(dest, source, config, stats);
        }
    } else {
        if !config.dry_run {
            fs::create_dir_all(dest)?;
        }
    }
    stats.folders_created.fetch_add(1, Ordering::Relaxed);

    process_source_contents(source, dest, config, stats)
}

/// Process a file source (handles filtering and conflicts)
///
/// # Errors
///
/// Returns an error if the file cannot be processed.
fn process_file_source(
    source: &Path,
    dest: &Path,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
    dest_exists: bool,
) -> Result<()> {
    if !matches_extension(source, &config.extensions) {
        return Ok(());
    }

    if !matches_filters(source, &config.include, &config.exclude) {
        return Ok(());
    }

    if !matches_size_filter(source, config.size.as_ref()) {
        return Ok(());
    }

    if dest_exists {
        handle_file_conflicts(source, dest, config, stats)
    } else {
        process_file(source, dest, config, stats)?;
        Ok(())
    }
}

/// Check if file matches extension filter
fn matches_extension(source: &Path, extensions: &[String]) -> bool {
    if extensions.is_empty() {
        return true;
    }
    let ext = source.extension().and_then(|s| s.to_str()).unwrap_or("");
    extensions.iter().any(|e| e.trim_start_matches('.') == ext)
}

/// Check if file matches size filter
fn matches_size_filter(source: &Path, size_filter: Option<&SizeFilter>) -> bool {
    let Some(filter) = size_filter else {
        return true;
    };

    let Ok(metadata) = std::fs::metadata(source) else {
        return false;
    };
    let size = metadata.len();

    if let Some(min) = filter.min {
        if size < min {
            return false;
        }
    }
    if let Some(max) = filter.max {
        if size > max {
            return false;
        }
    }
    true
}

/// Check if file matches include/exclude filters
fn matches_filters(source: &Path, include: &[String], exclude: &[String]) -> bool {
    let file_name = source.file_name().unwrap_or(source.as_os_str());
    let file_name_str = file_name.to_string_lossy();

    if !include.is_empty() {
        let matches = include.iter().any(|p| {
            glob::Pattern::new(p)
                .map(|pat| pat.matches(&file_name_str))
                .unwrap_or(false)
        });
        if !matches {
            return false;
        }
    }

    if !exclude.is_empty() {
        let matches = exclude.iter().any(|p| {
            glob::Pattern::new(p)
                .map(|pat| pat.matches(&file_name_str))
                .unwrap_or(false)
        });
        if matches {
            return false;
        }
    }

    true
}

/// Handle file conflicts (destination exists)
///
/// # Errors
///
/// Returns an error if conflict resolution fails.
fn handle_file_conflicts(source: &Path, dest: &Path, config: &CopyConfig, stats: &Arc<CopyStats>) -> Result<()> {
    stats.conflicts_resolved.fetch_add(1, Ordering::Relaxed);

    if dest.is_dir() {
        handle_file_over_folder(source, dest, config, stats)
    } else {
        handle_file_over_file(source, dest, config, stats)
    }
}
#[allow(clippy::too_many_lines)]
/// Process a single file copy/move operation
///
/// Handles size limits, simulate mode, and actual file transfer.
///
/// # Errors
///
/// Returns an error if the file cannot be read or written.
fn process_file(source: &Path, dest: &Path, config: &CopyConfig, stats: &Arc<CopyStats>) -> Result<()> {
    let file_size = fs::metadata(source)?.len();

    // Check size limit
    if let Some(limit) = config.size_limit {
        let mut current = stats.bytes_reserved.load(Ordering::Relaxed);
        loop {
            if current.saturating_add(file_size) > limit {
                return Ok(());
            }
            match stats.bytes_reserved.compare_exchange(
                current,
                current.saturating_add(file_size),
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => {
                    current = actual;
                    // Hint to the CPU that we're in a spin-wait loop
                    std::hint::spin_loop();
                }
            }
        }
    }

    // Check file count limit
    if let Some(limit) = config.limit {
        let mut current = stats.files_reserved.load(Ordering::Relaxed);
        loop {
            if current >= limit {
                return Ok(());
            }
            match stats.files_reserved.compare_exchange(
                current,
                current.saturating_add(1),
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => {
                    current = actual;
                    // Hint to the CPU that we're in a spin-wait loop
                    std::hint::spin_loop();
                }
            }
        }
    }

    if config.dry_run {
        if config.verbose {
            let action = if config.is_copy { "copy" } else { "move" };
            eprintln!(
                "[SIMULATE] {} {} -> {} ({})",
                action,
                shell_quote(source.to_string_lossy().as_ref()),
                shell_quote(dest.to_string_lossy().as_ref()),
                crate::utils::format_size(file_size)
            );
        }
        return Ok(());
    }

    // Ensure parent directory exists
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }

    if config.is_copy {
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
        let action = if config.is_copy { "copy" } else { "move" };
        eprintln!(
            "{} {} -> {} ({})",
            action,
            shell_quote(source.to_string_lossy().as_ref()),
            shell_quote(dest.to_string_lossy().as_ref()),
            crate::utils::format_size(file_size)
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
fn handle_file_over_file(source: &Path, dest: &Path, config: &CopyConfig, stats: &Arc<CopyStats>) -> Result<()> {
    let strategy = &config.file_over_file;

    // Get metadata for both files
    let source_metadata = fs::metadata(source)?;
    let dest_metadata = fs::metadata(dest)?;
    let source_size = source_metadata.len();
    let dest_size = dest_metadata.len();
    let source_modified = source_metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let dest_modified = dest_metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let source_created = source_metadata
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());
    let dest_created = dest_metadata
        .created()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs());

    // Check hash-based conditions if needed
    let hashes_match = if strategy.rules.iter().any(|r| r.attribute == Attribute::Hash) {
        files_match_by_hash(source, dest)?
    } else {
        false
    };

    // Evaluate all rules
    match evaluate_rules(
        strategy,
        hashes_match,
        source_size,
        dest_size,
        source_modified,
        dest_modified,
        source_created,
        dest_created,
        config,
        source,
        dest,
        stats,
    )? {
        RuleResult::Skip | RuleResult::DeleteDest | RuleResult::DeleteSrc => return Ok(()),
        RuleResult::NoMatch => {
            // No rules matched, apply required fallback
        }
    }

    // Apply required fallback
    apply_required_strategy(strategy, source, dest, config, stats)
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
) -> Result<()> {
    match strategy.required {
        FileOverFileMode::Skip => {
            execute_action(RuleAction::Skip, source, dest, config, stats, "strategy: skip")?;
        }
        FileOverFileMode::DeleteSrc => {
            execute_action(
                RuleAction::DeleteSrc,
                source,
                dest,
                config,
                stats,
                "strategy: delete-src",
            )?;
        }
        FileOverFileMode::DeleteDest => {
            execute_action(
                RuleAction::DeleteDest,
                source,
                dest,
                config,
                stats,
                "strategy: delete-dest",
            )?;
            return process_file(source, dest, config, stats);
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
            return process_file(source, &new_dest, config, stats);
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
            if !config.dry_run {
                // First rename the existing destination
                fs::rename(dest, &renamed_dest)?;
            }
            // Then copy/move source to original destination
            return process_file(source, dest, config, stats);
        }
    }

    Ok(())
}

/// Handle file-over-folder conflict resolution
///
/// # Errors
///
/// Returns an error if file operations fail.
fn handle_file_over_folder(source: &Path, dest: &Path, config: &CopyConfig, stats: &Arc<CopyStats>) -> Result<()> {
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
            if !config.dry_run {
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
            if !config.dry_run {
                fs::remove_dir_all(dest)?;
            }
            // Now copy file to original path
            return process_file(source, dest, config, stats);
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
            return process_file(source, &new_dest, config, stats);
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
            if !config.dry_run {
                fs::rename(dest, &renamed_dest)?;
            }
            // Copy file to original path
            return process_file(source, dest, config, stats);
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
            return process_file(source, &new_dest, config, stats);
        }
    }

    Ok(())
}

/// Handle folder-over-file conflict resolution
///
/// # Errors
///
/// Returns an error if file operations fail.
fn handle_folder_over_file(dest: &Path, source: &Path, config: &CopyConfig, stats: &Arc<CopyStats>) -> Result<()> {
    match config.folder_over_file {
        FolderConflictMode::Skip => {
            if config.verbose {
                eprintln!(
                    "Skipping folder {} (strategy: skip)",
                    shell_quote(source.to_string_lossy().as_ref())
                );
            }
            Ok(())
        }
        FolderConflictMode::DeleteSrc => {
            if config.verbose {
                eprintln!(
                    "Deleting source folder {} (strategy: delete-src)",
                    shell_quote(source.to_string_lossy().as_ref())
                );
            }
            if !config.dry_run {
                fs::remove_dir_all(source)?;
            }
            Ok(())
        }
        FolderConflictMode::DeleteDest => {
            if config.verbose {
                eprintln!(
                    "Deleting destination file {} (strategy: delete-dest)",
                    shell_quote(dest.to_string_lossy().as_ref())
                );
            }
            if !config.dry_run {
                fs::remove_file(dest)?;
            }
            // Now create the folder and copy contents
            if !config.dry_run {
                fs::create_dir_all(dest)?;
            }
            stats.folders_created.fetch_add(1, Ordering::Relaxed);
            process_source_contents(source, dest, config, stats)
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
            if !config.dry_run {
                fs::create_dir_all(&new_dest)?;
            }
            stats.folders_created.fetch_add(1, Ordering::Relaxed);
            // Process source contents into new destination
            process_source_contents(source, &new_dest, config, stats)
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
            if !config.dry_run {
                fs::rename(dest, &renamed_dest)?;
                fs::create_dir_all(dest)?;
            }
            stats.folders_created.fetch_add(1, Ordering::Relaxed);
            process_source_contents(source, dest, config, stats)
        }
        FolderConflictMode::Merge => {
            // Folder over file with merge: rename the file and create the folder at the original path
            // This places the file inside the new folder, preserving both
            let renamed_dest = get_unique_folder_name(dest);
            if config.verbose {
                eprintln!(
                    "Renaming destination file {} -> {} (strategy: merge)",
                    shell_quote(dest.to_string_lossy().as_ref()),
                    shell_quote(renamed_dest.to_string_lossy().as_ref())
                );
            }
            if !config.dry_run {
                fs::rename(dest, &renamed_dest)?;
                fs::create_dir_all(dest)?;
            }
            stats.folders_created.fetch_add(1, Ordering::Relaxed);
            process_source_contents(source, dest, config, stats)
        }
    }
}

/// Process directory contents recursively
///
/// # Errors
///
/// Returns an error if directory cannot be read or entries cannot be processed.
fn process_source_contents(source: &Path, dest: &Path, config: &CopyConfig, stats: &Arc<CopyStats>) -> Result<()> {
    let entries = fs::read_dir(source)?;
    for entry_result in entries {
        let entry = entry_result?;
        let entry_path = entry.path();
        let entry_name = entry.file_name();
        let entry_dest = dest.join(&entry_name);

        if let Err(e) = process_source(&entry_path, &entry_dest, config, stats) {
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
/// Uses a regex-like approach to find existing _N suffixes and increment them.
/// Thread-safe: uses atomic file creation to prevent race conditions.
///
/// # Returns
///
/// Returns a path with a unique filename that doesn't exist on disk.
///
/// Uses atomic file creation to avoid TOCTOU race conditions when multiple
/// threads are generating unique filenames concurrently.
#[allow(
    clippy::option_if_let_else,
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::redundant_closure_for_method_calls,
    clippy::doc_markdown
)]
fn get_unique_filename(base: &Path) -> PathBuf {
    // Quick check: if base doesn't exist, use it directly
    if !base.exists() {
        return base.to_path_buf();
    }

    let dir = base.parent().unwrap_or_else(|| Path::new("."));
    let Some(file_name) = base.file_name() else {
        // No filename (e.g., root directory), append timestamp to full path
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        return dir.join(format!("{}_{}", base.display(), timestamp));
    };

    #[cfg(unix)]
    {
        // Unix: work with raw bytes to handle any valid filename
        let file_name_bytes = file_name.as_bytes();

        // Find extension (last '.' in filename)
        let (stem_bytes, ext_bytes) = if let Some(last_dot) = file_name_bytes.iter().rposition(|&b| b == b'.') {
            if last_dot > 0 {
                (&file_name_bytes[..last_dot], &file_name_bytes[last_dot..])
            } else {
                (file_name_bytes, &file_name_bytes[0..0])
            }
        } else {
            (file_name_bytes, &file_name_bytes[0..0])
        };

        // Try to parse existing _N suffix from stem
        let (base_stem, start_idx) = find_suffix_and_index_bytes(stem_bytes);

        // Try sequential numbers starting from start_idx
        for i in start_idx.. {
            let mut new_name = Vec::with_capacity(base_stem.len() + ext_bytes.len() + 10);
            new_name.extend_from_slice(base_stem);
            new_name.extend_from_slice(b"_");
            new_name.extend_from_slice(i.to_string().as_bytes());
            new_name.extend_from_slice(ext_bytes);

            // Convert back to OsString
            let new_name_os = OsStr::from_bytes(&new_name).to_os_string();
            let new_path = dir.join(new_name_os);

            // Use atomic file creation to avoid TOCTOU race conditions
            if std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&new_path)
                .is_ok()
            {
                let _ = std::fs::remove_file(&new_path);
                return new_path;
            }
        }

        // Fallback: use timestamp to guarantee uniqueness
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let mut fallback_name = file_name_bytes.to_vec();
        fallback_name.extend_from_slice(b"_");
        fallback_name.extend_from_slice(timestamp.to_string().as_bytes());
        let fallback_name_os = OsStr::from_bytes(&fallback_name).to_os_string();
        dir.join(fallback_name_os)
    }

    #[cfg(not(unix))]
    {
        // Non-Unix (Windows): use UTF-8 strings
        let Some(file_stem) = file_name.to_str() else {
            // Invalid UTF-8 on Windows is rare; use lossy conversion as fallback
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            return dir.join(format!("{}_{}", file_name.to_string_lossy(), timestamp));
        };
        let extension = std::path::Path::new(file_name)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("");

        let (base_name, start_idx) = find_suffix_and_index(file_stem);

        for i in start_idx.. {
            let new_name = if extension.is_empty() {
                format!("{base_name}_{i}")
            } else {
                format!("{base_name}_{i}.{extension}")
            };
            let new_path = dir.join(&new_name);

            if std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&new_path)
                .is_ok()
            {
                let _ = std::fs::remove_file(&new_path);
                return new_path;
            }
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let fallback_name = if extension.is_empty() {
            format!("{base_name}_{timestamp}")
        } else {
            format!("{base_name}_{timestamp}.{extension}")
        };
        return dir.join(fallback_name);
    }
}

/// Find the base name and next index from a file stem (as bytes) that may have a _N suffix
///
/// Handles cases like:
/// - "file" -> ("file", 1)
/// - "file_1" -> ("file", 2)
/// - "file_1_2" -> ("file_1", 3)
/// - "file_name_999" -> ("file_name", 1000)
#[must_use]
#[cfg(unix)]
#[allow(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::redundant_closure_for_method_calls,
    clippy::doc_markdown
)]
fn find_suffix_and_index_bytes(stem: &[u8]) -> (&[u8], u32) {
    // Find the last underscore and check if what follows is a number
    if let Some(last_underscore) = stem.iter().rposition(|&b| b == b'_') {
        let suffix = &stem[last_underscore + 1..];
        // Check if suffix is all ASCII digits
        if !suffix.is_empty() && suffix.iter().all(u8::is_ascii_digit) {
            if let Some(num) = std::str::from_utf8(suffix).ok().and_then(|s| s.parse::<u32>().ok()) {
                return (&stem[..last_underscore], num + 1);
            }
        }
    }
    // No valid numeric suffix found, start from 1
    (stem, 1)
}

/// Find the base name and next index from a file stem (as string) that may have a _N suffix
///
/// Handles cases like:
/// - "file" -> ("file", 1)
/// - "file_1" -> ("file", 2)
/// - "file_1_2" -> ("file_1", 3)
/// - "file_name_999" -> ("file_name", 1000)
#[must_use]
#[cfg(not(unix))]
fn find_suffix_and_index(file_stem: &str) -> (&str, u32) {
    // Find the last underscore and check if what follows is a number
    if let Some(last_underscore) = file_stem.rfind('_') {
        let suffix = &file_stem[last_underscore + 1..];
        if let Ok(num) = suffix.parse::<u32>() {
            return (&file_stem[..last_underscore], num + 1);
        }
    }
    // No valid numeric suffix found, start from 1
    (file_stem, 1)
}

/// Generate a unique folder name by appending _N suffix if folder exists
///
/// Uses the same suffix parsing logic as `get_unique_filename` for consistency.
///
/// # Returns
///
/// Returns a path with a unique folder name that doesn't exist on disk.
///
/// Uses atomic directory creation to avoid TOCTOU race conditions.
#[allow(
    clippy::indexing_slicing,
    clippy::arithmetic_side_effects,
    clippy::redundant_closure_for_method_calls
)]
fn get_unique_folder_name(base: &Path) -> PathBuf {
    if !base.exists() {
        return base.to_path_buf();
    }

    let dir = base.parent().unwrap_or_else(|| Path::new("."));
    let Some(folder_name) = base.file_name() else {
        // No filename (e.g., root directory), append timestamp to full path
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        return dir.join(format!("{}_{}", base.display(), timestamp));
    };

    #[cfg(unix)]
    {
        // Unix: work with raw bytes to handle any valid folder name
        let folder_name_bytes = folder_name.as_bytes();

        // Try to parse existing _N suffix from folder name
        let (base_name, start_idx) = find_suffix_and_index_bytes(folder_name_bytes);

        for i in start_idx.. {
            let mut new_name = Vec::with_capacity(base_name.len() + 10);
            new_name.extend_from_slice(base_name);
            new_name.extend_from_slice(b"_");
            new_name.extend_from_slice(i.to_string().as_bytes());

            let new_name_os = OsStr::from_bytes(&new_name).to_os_string();
            let new_path = dir.join(&new_name_os);

            // Use atomic directory creation to avoid TOCTOU race conditions
            match std::fs::create_dir(&new_path) {
                Ok(()) => {
                    // Successfully created, remove it so caller can use the name
                    let _ = std::fs::remove_dir(&new_path);
                    return new_path;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => {}
            }
        }

        // Fallback: use timestamp to guarantee uniqueness
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let mut fallback_name = folder_name_bytes.to_vec();
        fallback_name.extend_from_slice(b"_");
        fallback_name.extend_from_slice(timestamp.to_string().as_bytes());
        let fallback_name_os = OsStr::from_bytes(&fallback_name).to_os_string();
        dir.join(fallback_name_os)
    }

    #[cfg(not(unix))]
    {
        // Non-Unix (Windows): use UTF-8 strings
        let Some(folder_name_str) = folder_name.to_str() else {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis();
            return dir.join(format!("{}_{}", folder_name.to_string_lossy(), timestamp));
        };

        let (base_name, start_idx) = find_suffix_and_index(folder_name_str);

        for i in start_idx.. {
            let new_name = format!("{base_name}_{i}");
            let new_path = dir.join(&new_name);

            match std::fs::create_dir(&new_path) {
                Ok(()) => {
                    let _ = std::fs::remove_dir(&new_path);
                    return new_path;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(_) => {}
            }
        }

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        return dir.join(format!("{base_name}_{timestamp}"));
    }
}

/// Check if two files match by comparing their hashes
///
/// # Errors
///
/// Returns an error if either file cannot be read.
fn files_match_by_hash(source: &Path, dest: &Path) -> Result<bool> {
    // Use sample hashing for efficiency
    let src_hash = utils::sample_hash(source)?;
    let dest_hash = utils::sample_hash(dest)?;
    Ok(src_hash == dest_hash)
}

/// Quote a path for shell usage
///
/// # Returns
///
/// Returns the path wrapped in single quotes with proper escaping.
/// Handles non-UTF8 paths by using lossy conversion for display.
fn shell_quote<P: AsRef<std::path::Path>>(path: P) -> String {
    let path_ref = path.as_ref();
    let s = path_ref.to_string_lossy();
    if s.is_empty() {
        return "''".to_string();
    }
    if s.chars().all(|c| c.is_alphanumeric() || "!@%_+=:,./-".contains(c)) {
        return format!("'{s}'");
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Print copy/move statistics
fn print_stats(stats: &CopyStats) {
    eprintln!("\nSummary:");
    eprintln!("  {} files copied", stats.files_copied.load(Ordering::Relaxed));
    eprintln!("  {} folders created", stats.folders_created.load(Ordering::Relaxed));
    eprintln!(
        "  {} bytes copied",
        crate::utils::format_size(stats.bytes_copied.load(Ordering::Relaxed))
    );
    eprintln!("  {} files skipped", stats.files_skipped.load(Ordering::Relaxed));
    eprintln!(
        "  {} conflicts resolved",
        stats.conflicts_resolved.load(Ordering::Relaxed)
    );
    let errors = stats.errors.load(Ordering::Relaxed);
    if errors > 0 {
        eprintln!("  {errors} errors");
    }
}
