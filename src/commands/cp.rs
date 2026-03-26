//! Copy command for nofs
//!
//! Implements cp/mv-like functionality with support for nofs context paths,
//! conflict resolution strategies, and parallel operations.

use crate::error::{NofsError, Result};
use crate::pool::Pool;
use std::fs;
use std::io::{self, Read, Seek};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Conflict resolution mode for file-over-file conflicts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileOverFileMode {
    Skip,
    RenameSrc,
    RenameDest,
    DeleteSrc,
    DeleteDest,
}

/// Conflict resolution mode for folder conflicts
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
#[derive(Debug, Clone)]
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
/// Format: "skip-hash rename-dest" or "delete-src-smaller skip"
pub fn parse_file_over_file(spec: &str) -> Result<FileOverFileStrategy> {
    let mut strategy = FileOverFileStrategy::default();
    let parts: Vec<&str> = spec.split_whitespace().collect();

    if parts.is_empty() {
        return Ok(strategy);
    }

    // Last part is the required mode
    let required_str = parts[parts.len() - 1];
    strategy.required = match required_str {
        "skip" => FileOverFileMode::Skip,
        "rename-src" => FileOverFileMode::RenameSrc,
        "rename-dest" => FileOverFileMode::RenameDest,
        "delete-src" => FileOverFileMode::DeleteSrc,
        "delete-dest" => FileOverFileMode::DeleteDest,
        _ => {
            return Err(NofsError::Parse(format!(
                "Unknown file-over-file mode: {}",
                required_str
            )))
        }
    };

    // Previous parts are optional conditions
    for opt in &parts[..parts.len() - 1] {
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
                    "Unknown file-over-file option: {}",
                    opt
                )))
            }
        }
    }

    Ok(strategy)
}

/// Statistics for copy/move operations
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
#[derive(Debug, Clone)]
pub struct CopyConfig {
    pub copy: bool,              // true = copy, false = move
    pub simulate: bool,          // dry-run mode
    pub workers: usize,          // number of parallel workers
    pub verbose: bool,           // verbose output
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
#[allow(clippy::too_many_lines)]
pub fn execute(
    sources: &[String],
    destination: &str,
    config: CopyConfig,
    pool: Option<&Pool>,
) -> Result<CopyStats> {
    let stats = Arc::new(CopyStats::default());
    let start_time = Instant::now();

    if sources.is_empty() {
        return Err(NofsError::CopyMove(
            "At least one source is required".to_string(),
        ));
    }

    // Normalize destination path
    let dest_path = PathBuf::from(destination);

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
        let source_path = PathBuf::from(source);

        if !source_path.exists() {
            eprintln!("Source {} does not exist", shell_quote(source));
            stats.errors.fetch_add(1, Ordering::Relaxed);
            continue;
        }

        // Determine final destination for this source
        let final_dest = if sources.len() > 1 || (dest_exists && dest_is_dir) {
            // Merge into destination directory
            let source_name = source_path
                .file_name()
                .unwrap_or(source_path.as_os_str());
            dest_path.join(source_name)
        } else {
            dest_path.clone()
        };

        // Process the source
        if let Err(e) = process_source(
            &source_path,
            &final_dest,
            &config,
            &stats,
            pool,
            &Arc::new(Mutex::new(0u64)),
            &Arc::new(Mutex::new(0u64)),
        ) {
            eprintln!("Error processing {}: {}", shell_quote(source), e);
            stats.errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    if config.verbose {
        let elapsed = start_time.elapsed();
        eprintln!("\nCompleted in {:.2?}", elapsed);
        print_stats(&stats);
    }

    Ok(CopyStats::default())
}

#[allow(clippy::too_many_lines)]
fn process_source(
    source: &Path,
    dest: &Path,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
    pool: Option<&Pool>,
    file_count: &Arc<Mutex<u64>>,
    byte_count: &Arc<Mutex<u64>>,
) -> Result<()> {
    let source_is_dir = source.is_dir();
    let dest_exists = dest.exists();

    // Check limits
    {
        let count = *file_count.lock().map_err(|e| {
            NofsError::CopyMove(format!("Lock poisoning: {}", e))
        })?;
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
        for entry in entries {
            let entry = entry?;
            let entry_path = entry.path();
            let entry_name = entry.file_name();
            let entry_dest = dest.join(&entry_name);

            if let Err(e) = process_source(
                &entry_path,
                &entry_dest,
                config,
                stats,
                pool,
                file_count,
                byte_count,
            ) {
                eprintln!("Error processing {}: {}", shell_quote(entry_path.to_string_lossy().as_ref()), e);
            }
        }
    } else {
        // Handle file
        // Check extension filter
        if !config.extensions.is_empty() {
            let ext = source
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("");
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
                // File over folder - move file into folder
                let file_name = source
                    .file_name()
                    .unwrap_or(source.as_os_str());
                let new_dest = dest.join(file_name);
                return process_file(
                    source,
                    &new_dest,
                    config,
                    stats,
                    file_count,
                    byte_count,
                );
            } else {
                // File over file conflict
                stats.conflicts_resolved.fetch_add(1, Ordering::Relaxed);
                return handle_file_over_file(source, dest, config, stats, file_count, byte_count);
            }
        }

        // No conflict - just copy/move
        process_file(source, dest, config, stats, file_count, byte_count)?;
    }

    Ok(())
}

#[allow(clippy::too_many_lines)]
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
        let mut bytes = byte_count.lock().map_err(|e| {
            NofsError::CopyMove(format!("Lock poisoning: {}", e))
        })?;
        if let Some(limit) = config.size_limit {
            if *bytes + file_size > limit {
                return Ok(());
            }
            *bytes += file_size;
        }
    }

    // Check file count limit
    {
        let mut count = file_count.lock().map_err(|e| {
            NofsError::CopyMove(format!("Lock poisoning: {}", e))
        })?;
        if let Some(limit) = config.limit {
            if *count >= limit {
                return Ok(());
            }
            *count += 1;
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
        stats.files_copied.fetch_add(1, Ordering::Relaxed);
        stats.bytes_copied.fetch_add(file_size, Ordering::Relaxed);
    } else {
        // Move the file (try rename first, fall back to copy+delete)
        if fs::rename(source, dest).is_err() {
            copy_file_contents(source, dest)?;
            fs::remove_file(source)?;
        }
        stats.files_copied.fetch_add(1, Ordering::Relaxed);
        stats.bytes_copied.fetch_add(file_size, Ordering::Relaxed);
    }

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

fn copy_file_contents(source: &Path, dest: &Path) -> Result<()> {
    let mut src_file = fs::File::open(source)?;
    let mut dst_file = fs::File::create(dest)?;
    io::copy(&mut src_file, &mut dst_file)?;

    // Preserve metadata
    let metadata = fs::metadata(source)?;
    fs::set_permissions(dest, metadata.permissions())?;

    Ok(())
}

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
    let hashes_match = if strategy.skip_hash
        || strategy.delete_dest_hash
        || strategy.delete_src_hash
    {
        files_match_by_hash(source, dest, stats)?
    } else {
        false
    };

    // Evaluate optional conditions
    if strategy.skip_hash && hashes_match {
        if config.verbose {
            eprintln!(
                "Skipping {} (hash matches)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }

    if strategy.skip_size && src_size == dest_size {
        if config.verbose {
            eprintln!(
                "Skipping {} (size matches)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }

    if strategy.skip_larger && src_size > dest_size {
        if config.verbose {
            eprintln!(
                "Skipping {} (source is larger)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }

    if strategy.skip_smaller && src_size < dest_size {
        if config.verbose {
            eprintln!(
                "Skipping {} (source is smaller)",
                shell_quote(source.to_string_lossy().as_ref())
            );
        }
        stats.files_skipped.fetch_add(1, Ordering::Relaxed);
        return Ok(());
    }

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
        return process_file(source, dest, config, stats, file_count, byte_count);
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
        return process_file(source, dest, config, stats, file_count, byte_count);
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
        return process_file(source, dest, config, stats, file_count, byte_count);
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
        return process_file(source, dest, config, stats, file_count, byte_count);
    }

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
        return Ok(());
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
        return Ok(());
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
        return Ok(());
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
        return Ok(());
    }

    // Apply required fallback
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
            let new_dest = get_unique_filename(dest)?;
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
            let renamed_dest = get_unique_filename(dest)?;
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
            // Now create the folder
            if !config.simulate {
                fs::create_dir_all(dest)?;
            }
            stats.folders_created.fetch_add(1, Ordering::Relaxed);
        }
        FolderConflictMode::RenameSrc => {
            // Rename the source folder conceptually by copying to renamed path
            let new_dest = get_unique_folder_name(dest)?;
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
            let renamed_dest = get_unique_folder_name(dest)?;
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
            let renamed_dest = get_unique_folder_name(dest)?;
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

fn process_source_contents(
    source: &Path,
    dest: &Path,
    config: &CopyConfig,
    stats: &Arc<CopyStats>,
) -> Result<()> {
    let entries = fs::read_dir(source)?;
    for entry in entries {
        let entry = entry?;
        let entry_path = entry.path();
        let entry_name = entry.file_name();
        let entry_dest = dest.join(&entry_name);

        if let Err(e) = process_source(
            &entry_path,
            &entry_dest,
            config,
            stats,
            None, // pool not used for local paths
            &Arc::new(Mutex::new(0u64)),
            &Arc::new(Mutex::new(0u64)),
        ) {
            eprintln!("Error processing {}: {}", shell_quote(entry_path.to_string_lossy().as_ref()), e);
        }
    }
    Ok(())
}

fn get_unique_filename(base: &Path) -> Result<PathBuf> {
    if !base.exists() {
        return Ok(base.to_path_buf());
    }

    let dir = base.parent().unwrap_or(Path::new("."));
    let file_stem = base
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    let extension = base.extension().and_then(|s| s.to_str()).unwrap_or("");

    // Check if file already has a _N suffix
    let (base_name, start_idx) = if let Some(idx) = file_stem.rfind('_') {
        let suffix = &file_stem[idx + 1..];
        if let Ok(num) = suffix.parse::<u32>() {
            (&file_stem[..idx], num + 1)
        } else {
            (file_stem, 1)
        }
    } else {
        (file_stem, 1)
    };

    for i in start_idx.. {
        let new_name = if extension.is_empty() {
            format!("{}_{}", base_name, i)
        } else {
            format!("{}_{}.{}", base_name, i, extension)
        };
        let new_path = dir.join(&new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }

    // Fallback (shouldn't reach here)
    Ok(base.to_path_buf().with_extension(format!(
        "{}.{}",
        base.extension().unwrap_or_default().to_string_lossy(),
        "conflict"
    )))
}

fn get_unique_folder_name(base: &Path) -> Result<PathBuf> {
    if !base.exists() {
        return Ok(base.to_path_buf());
    }

    let dir = base.parent().unwrap_or(Path::new("."));
    let folder_name = base
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("folder");

    // Check if folder already has a _N suffix
    let (base_name, start_idx) = if let Some(idx) = folder_name.rfind('_') {
        let suffix = &folder_name[idx + 1..];
        if let Ok(num) = suffix.parse::<u32>() {
            (&folder_name[..idx], num + 1)
        } else {
            (folder_name, 1)
        }
    } else {
        (folder_name, 1)
    };

    for i in start_idx.. {
        let new_name = format!("{}_{}", base_name, i);
        let new_path = dir.join(&new_name);
        if !new_path.exists() {
            return Ok(new_path);
        }
    }

    Ok(base.to_path_buf().with_extension("conflict"))
}

fn files_match_by_hash(source: &Path, dest: &Path, stats: &CopyStats) -> Result<bool> {
    // Use sample hashing for efficiency
    let src_hash = sample_hash(source, stats)?;
    let dest_hash = sample_hash(dest, stats)?;
    Ok(src_hash == dest_hash)
}

fn sample_hash(path: &Path, stats: &CopyStats) -> Result<String> {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let metadata = fs::metadata(path)?;
    let size = metadata.len();

    // For small files, hash the entire content
    if size <= 640 * 1024 {
        let content = fs::read(path)?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        stats.full_hashes.fetch_add(1, Ordering::Relaxed);
        return Ok(format!("{:x}", hasher.finish()));
    }

    // For larger files, sample at multiple positions
    let mut file = fs::File::open(path)?;
    let mut hasher = DefaultHasher::new();
    let chunk_size = 64 * 1024;
    let num_samples = 10;

    stats.sample_hashes.fetch_add(1, Ordering::Relaxed);

    for i in 0..num_samples {
        let pos = (size * i as u64) / num_samples as u64;
        file.seek(io::SeekFrom::Start(pos))?;
        let mut buf = vec![0u8; chunk_size as usize];
        let bytes_read = file.read(&mut buf).unwrap_or(0);
        buf[..bytes_read].hash(&mut hasher);
    }

    Ok(format!("{:x}", hasher.finish()))
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn shell_quote<S: AsRef<str>>(s: S) -> String {
    let s = s.as_ref();
    if s.is_empty() {
        return "''".to_string();
    }
    if s.chars().all(|c| c.is_alphanumeric() || "!@%_+=:,./-".contains(c)) {
        return format!("'{}'", s);
    }
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn print_stats(stats: &CopyStats) {
    eprintln!("\nSummary:");
    eprintln!("  {} files copied", stats.files_copied.load(Ordering::Relaxed));
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
        eprintln!("  {} errors", errors);
    }
}
