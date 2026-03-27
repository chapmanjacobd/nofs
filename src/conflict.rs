//! Conflict detection utilities for nofs
//!
//! Provides functions to detect when the same file exists in multiple
//! branches with different content.

use crate::branch::Branch;
use crate::error::{NofsError, Result};
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek};
use std::path::{Path, PathBuf};

/// Represents a conflict between files in different branches
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct FileConflict {
    /// Filename
    pub name: String,
    /// Branches that contain this file
    pub branches: Vec<BranchConflict>,
}

/// A file in a specific branch
#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct BranchConflict {
    /// Branch information
    pub branch_name: String,
    /// Full path to the file
    pub path: String,
    /// File size in bytes
    pub size: u64,
    /// File hash (if computed)
    pub hash: Option<String>,
    /// File modification time (mtime) as Unix timestamp
    pub mtime: Option<u64>,
    /// File creation time (ctime) as Unix timestamp
    pub ctime: Option<u64>,
}

/// Detect conflicts in a directory - files that exist in multiple branches
/// with different content
///
/// # Errors
///
/// Returns an error if there is an IO error reading files.
#[allow(clippy::missing_panics_doc)]
pub fn detect_conflicts(
    branches: &[&Branch],
    relative_path: &Path,
    use_hash: bool,
) -> Result<Vec<FileConflict>> {
    let mut conflicts = Vec::new();

    // Collect all files from all branches
    let mut file_map: std::collections::HashMap<String, Vec<BranchConflict>> =
        std::collections::HashMap::new();

    for branch in branches {
        let branch_path = branch.path.join(relative_path);

        if !branch_path.exists() || !branch_path.is_dir() {
            continue;
        }

        let Ok(entries) = fs::read_dir(&branch_path) else {
            continue;
        };

        for entry in entries.flatten() {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy().to_string();

            // Skip directories
            if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                continue;
            }

            let Ok(metadata) = entry.metadata() else {
                continue;
            };

            let size = metadata.len();
            let mtime = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            let ctime = metadata
                .created()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            let hash = if use_hash {
                compute_file_hash(&entry.path()).ok()
            } else {
                None
            };

            file_map
                .entry(file_name_str)
                .or_default()
                .push(BranchConflict {
                    branch_name: branch.path.to_string_lossy().to_string(),
                    path: entry.path().to_string_lossy().to_string(),
                    size,
                    hash,
                    mtime,
                    ctime,
                });
        }
    }

    // Find files that exist in multiple branches with different content
    for (name, mut branch_files) in file_map {
        if branch_files.len() < 2 {
            continue;
        }

        // Check if files have different content
        if files_differ(&branch_files, use_hash) {
            // Sort branches by mtime (newest first), then by path for consistent output
            branch_files.sort_by(|a, b| b.mtime.cmp(&a.mtime).then_with(|| a.path.cmp(&b.path)));

            conflicts.push(FileConflict {
                name,
                branches: branch_files,
            });
        }
    }

    conflicts.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(conflicts)
}

/// Check if files in different branches have different content
fn files_differ(branch_files: &[BranchConflict], use_hash: bool) -> bool {
    if branch_files.len() < 2 {
        return false;
    }

    // First check sizes - if sizes differ, content definitely differs
    let Some(first) = branch_files.first() else {
        return false;
    };
    let first_size = first.size;
    for bf in branch_files.iter().skip(1) {
        if bf.size != first_size {
            return true;
        }
    }

    // If sizes match but we're using hash comparison, check hashes
    if use_hash {
        // Compute hashes if not already computed
        let mut hashes: Vec<Option<String>> =
            branch_files.iter().map(|bf| bf.hash.clone()).collect();

        // Compute missing hashes
        for (i, bf) in branch_files.iter().enumerate() {
            if hashes.get(i).is_some_and(Option::is_none) {
                if let Ok(h) = compute_file_hash(&PathBuf::from(&bf.path)) {
                    if let Some(hash_entry) = hashes.get_mut(i) {
                        *hash_entry = Some(h);
                    }
                }
            }
        }

        // Compare hashes
        let Some(first_hash) = hashes.first() else {
            return false;
        };
        if let Some(h0) = first_hash {
            for h in hashes.iter().skip(1) {
                if Some(h0) != h.as_ref() {
                    return true;
                }
            }
        }
    }

    false
}

/// Compute a hash of a file's content
///
/// # Errors
///
/// Returns an error if the file cannot be read.
#[allow(clippy::missing_panics_doc, clippy::integer_division)]
pub fn compute_file_hash(path: &Path) -> Result<String> {
    const SMALL_FILE_THRESHOLD: u64 = crate::utils::MB; // 1MB

    let mut file = File::open(path).map_err(|e| {
        NofsError::Conflict(format!("Failed to open file {}: {}", path.display(), e))
    })?;

    // For small files, hash the entire content
    let metadata = file.metadata().map_err(|e| {
        NofsError::Conflict(format!(
            "Failed to get metadata for {}: {}",
            path.display(),
            e
        ))
    })?;

    if metadata.len() <= SMALL_FILE_THRESHOLD {
        let mut content = Vec::new();
        file.read_to_end(&mut content).map_err(|e| {
            NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e))
        })?;
        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        return Ok(format!("{:x}", hasher.finish()));
    }

    // For larger files, sample beginning, middle, and end
    let mut hasher = DefaultHasher::new();
    let mut buf = vec![0u8; (8 * crate::utils::KB) as usize];

    // Sample beginning (first 8KB)
    let bytes_read = file.read(&mut buf).map_err(|e| {
        NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e))
    })?;
    if let Some(buf_slice) = buf.get(..bytes_read) {
        buf_slice.hash(&mut hasher);
    }

    // Sample middle
    let file_size = metadata.len();
    let middle_pos = file_size / 2;
    file.seek(std::io::SeekFrom::Start(middle_pos))
        .map_err(|e| {
            NofsError::Conflict(format!("Failed to seek in file {}: {}", path.display(), e))
        })?;
    let bytes_read_middle = file.read(&mut buf).map_err(|e| {
        NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e))
    })?;
    if let Some(buf_slice) = buf.get(..bytes_read_middle) {
        buf_slice.hash(&mut hasher);
    }

    // Sample end (last 8KB)
    let end_pos = file_size.saturating_sub(8 * crate::utils::KB);
    file.seek(std::io::SeekFrom::Start(end_pos)).map_err(|e| {
        NofsError::Conflict(format!("Failed to seek in file {}: {}", path.display(), e))
    })?;
    let bytes_read_end = file.read(&mut buf).map_err(|e| {
        NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e))
    })?;
    if let Some(buf_slice) = buf.get(..bytes_read_end) {
        buf_slice.hash(&mut hasher);
    }

    Ok(format!("{:x}", hasher.finish()))
}

/// Detect conflict for a single file across branches
///
/// # Errors
///
/// Returns an error if there is an IO error reading files.
#[allow(clippy::missing_panics_doc)]
pub fn detect_single_file_conflict(
    branches: &[&Branch],
    relative_path: &Path,
    use_hash: bool,
) -> Result<Option<FileConflict>> {
    let mut branch_files = Vec::new();

    for branch in branches {
        let file_path = branch.path.join(relative_path);

        if !file_path.exists() || !file_path.is_file() {
            continue;
        }

        let Ok(metadata) = fs::metadata(&file_path) else {
            continue;
        };

        let size = metadata.len();
        let mtime = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let ctime = metadata
            .created()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        let hash = if use_hash {
            compute_file_hash(&file_path).ok()
        } else {
            None
        };

        branch_files.push(BranchConflict {
            branch_name: branch.path.to_string_lossy().to_string(),
            path: file_path.to_string_lossy().to_string(),
            size,
            hash,
            mtime,
            ctime,
        });
    }

    if branch_files.len() < 2 {
        return Ok(None);
    }

    if files_differ(&branch_files, use_hash) {
        // Sort branches by mtime (newest first), then by path for consistent output
        branch_files.sort_by(|a, b| b.mtime.cmp(&a.mtime).then_with(|| a.path.cmp(&b.path)));

        let file_name = relative_path.file_name().map_or_else(
            || relative_path.to_string_lossy().to_string(),
            |s| s.to_string_lossy().to_string(),
        );

        Ok(Some(FileConflict {
            name: file_name,
            branches: branch_files,
        }))
    } else {
        Ok(None)
    }
}
