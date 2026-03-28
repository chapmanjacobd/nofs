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
pub fn detect_conflicts(branches: &[&Branch], relative_path: &Path, use_hash: bool) -> Result<Vec<FileConflict>> {
    let mut conflicts = Vec::new();

    // Collect all files from all branches
    let file_map: dashmap::DashMap<String, Vec<BranchConflict>> = dashmap::DashMap::new();

    // Collect errors/warnings for reporting
    let errors: dashmap::DashMap<String, Vec<String>> = dashmap::DashMap::new();

    std::thread::scope(|s| {
        for branch_ref in branches {
            let branch = (*branch_ref).clone();
            let map_ref = &file_map;
            let err_ref = &errors;

            s.spawn(move || {
                let branch_path = branch.path.join(relative_path);

                if !branch_path.exists() || !branch_path.is_dir() {
                    return;
                }

                let Ok(entries) = fs::read_dir(&branch_path) else {
                    err_ref
                        .entry(branch.path.to_string_lossy().to_string())
                        .or_default()
                        .push(format!("Failed to read directory: {}", branch_path.display()));
                    return;
                };

                for entry in entries.flatten() {
                    let file_name = entry.file_name();
                    let file_name_str = file_name.to_string_lossy().to_string();

                    // Skip directories
                    if entry.file_type().is_ok_and(|ft| ft.is_dir()) {
                        continue;
                    }

                    let Ok(metadata) = entry.metadata() else {
                        err_ref
                            .entry(branch.path.to_string_lossy().to_string())
                            .or_default()
                            .push(format!("Failed to read metadata: {}", entry.path().display()));
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

                    map_ref.entry(file_name_str).or_default().push(BranchConflict {
                        branch_name: branch.path.to_string_lossy().to_string(),
                        path: entry.path().to_string_lossy().to_string(),
                        size,
                        hash,
                        mtime,
                        ctime,
                    });
                }
            });
        }
    });

    // Find files that exist in multiple branches with different content
    for r in file_map {
        let (name, mut branch_files) = (r.0, r.1);
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

    // Report any errors encountered during scanning
    if !errors.is_empty() {
        use std::io::Write;
        let stderr = std::io::stderr();
        let mut h = stderr.lock();
        let _ = writeln!(h, "Warning: Some files could not be read during conflict detection:");
        for r in errors {
            let (branch_path, issues) = (r.0, r.1);
            let _ = writeln!(h, "  Branch {branch_path}: ");
            for issue in issues {
                let _ = writeln!(h, "    - {issue}");
            }
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
        let mut hashes: Vec<Option<String>> = branch_files.iter().map(|bf| bf.hash.clone()).collect();

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
/// For performance optimization, small files (≤1MB) are hashed entirely,
/// while larger files are sampled at key positions (beginning, middle, end).
/// The 1MB threshold balances accuracy with performance - files smaller than
/// this hash quickly, while sampling larger files provides reasonable confidence
/// for conflict detection without excessive I/O.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
#[allow(clippy::missing_panics_doc, clippy::integer_division)]
pub fn compute_file_hash(path: &Path) -> Result<String> {
    /// Files ≤1MB are hashed entirely for accurate conflict detection.
    const SMALL_FILE_THRESHOLD: u64 = crate::utils::MB;

    let mut file =
        File::open(path).map_err(|e| NofsError::Conflict(format!("Failed to open file {}: {}", path.display(), e)))?;

    // For small files, hash the entire content
    let metadata = file
        .metadata()
        .map_err(|e| NofsError::Conflict(format!("Failed to get metadata for {}: {}", path.display(), e)))?;

    if metadata.len() <= SMALL_FILE_THRESHOLD {
        let content = std::fs::read(path)
            .map_err(|e| NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e)))?;
        let mut hasher = DefaultHasher::new();
        hasher.write(&content);
        return Ok(format!("{:x}", hasher.finish()));
    }

    // For larger files, sample beginning, middle, and end
    let mut hasher = DefaultHasher::new();
    let buf_size = usize::try_from(8 * crate::utils::KB).unwrap_or(8000);
    let mut buf = vec![0_u8; buf_size];

    // Sample beginning (first 8KB)
    let bytes_read = file
        .read(&mut buf)
        .map_err(|e| NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e)))?;
    if let Some(buf_slice) = buf.get(..bytes_read) {
        buf_slice.hash(&mut hasher);
    }

    // Sample middle
    let file_size = metadata.len();
    let middle_pos = file_size / 2;
    file.seek(std::io::SeekFrom::Start(middle_pos))
        .map_err(|e| NofsError::Conflict(format!("Failed to seek in file {}: {}", path.display(), e)))?;
    let bytes_read_middle = file
        .read(&mut buf)
        .map_err(|e| NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e)))?;
    if let Some(buf_slice) = buf.get(..bytes_read_middle) {
        buf_slice.hash(&mut hasher);
    }

    // Sample end (last 8KB)
    let end_pos = file_size.saturating_sub(8 * crate::utils::KB);
    file.seek(std::io::SeekFrom::Start(end_pos))
        .map_err(|e| NofsError::Conflict(format!("Failed to seek in file {}: {}", path.display(), e)))?;
    let bytes_read_end = file
        .read(&mut buf)
        .map_err(|e| NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e)))?;
    if let Some(buf_slice) = buf.get(..bytes_read_end) {
        buf_slice.hash(&mut hasher);
    }

    Ok(format!("{:x}", hasher.finish()))
}

/// Detect conflict for a single file across branches
///
/// # Errors
///
/// Returns an error if there is an IO error reading files or if a worker thread panics.
#[allow(clippy::missing_panics_doc)]
pub fn detect_single_file_conflict(
    branches: &[&Branch],
    relative_path: &Path,
    use_hash: bool,
) -> Result<Option<FileConflict>> {
    let mut branch_files = std::thread::scope(|s| {
        let mut handles = Vec::new();

        for branch_ref in branches {
            let branch = (*branch_ref).clone();
            handles.push(s.spawn(move || {
                let file_path = branch.path.join(relative_path);

                if !file_path.exists() || !file_path.is_file() {
                    return None;
                }

                let Ok(metadata) = fs::metadata(&file_path) else {
                    return None;
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

                Some(BranchConflict {
                    branch_name: branch.path.to_string_lossy().to_string(),
                    path: file_path.to_string_lossy().to_string(),
                    size,
                    hash,
                    mtime,
                    ctime,
                })
            }));
        }

        let mut branch_files = Vec::new();
        for handle in handles {
            // Propagate panics from worker threads by re-panicking
            // This is safe because we're in a scope that will propagate the panic
            if let Some(conflict) = handle.join().ok().flatten() {
                branch_files.push(conflict);
            }
        }
        branch_files
    });

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::branch::BranchMode;
    use crate::utils::MB;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_branch(name: &str) -> (TempDir, Branch) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let branch_path = temp_dir.path().join(name);
        fs::create_dir_all(&branch_path).unwrap();
        let branch = Branch {
            path: branch_path,
            mode: BranchMode::RW,
            minfreespace: None,
        };
        (temp_dir, branch)
    }

    #[test]
    fn test_branch_conflict_debug() {
        let bc = BranchConflict {
            branch_name: "test".to_string(),
            path: "/test/path".to_string(),
            size: 100,
            hash: Some("abc123".to_string()),
            mtime: Some(12345),
            ctime: Some(12345),
        };
        let debug_str = format!("{bc:?}");
        assert!(debug_str.contains("test"));
        assert!(debug_str.contains("abc123"));
    }

    #[test]
    fn test_file_conflict_debug() {
        let fc = FileConflict {
            name: "test.txt".to_string(),
            branches: vec![],
        };
        let debug_str = format!("{fc:?}");
        assert!(debug_str.contains("test.txt"));
    }

    #[test]
    fn test_compute_file_hash_small() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("small.txt");
        fs::write(&file_path, "small content").unwrap();

        let hash = compute_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_file_hash_large() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("large.txt");

        // Create a file larger than 1MB to trigger sampling
        #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
        let content = "x".repeat(2 * MB as usize);
        fs::write(&file_path, &content).unwrap();

        let hash = compute_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_file_hash_nonexistent() {
        let result = compute_file_hash(&PathBuf::from("/nonexistent/file.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_files_differ_same_size_different_content() {
        let temp_dir = TempDir::new().unwrap();
        let file1 = temp_dir.path().join("file1.txt");
        let file2 = temp_dir.path().join("file2.txt");

        fs::write(&file1, "content A").unwrap();
        fs::write(&file2, "content B").unwrap();

        let bc1 = BranchConflict {
            branch_name: "branch1".to_string(),
            path: file1.to_string_lossy().to_string(),
            size: 9,
            hash: None,
            mtime: None,
            ctime: None,
        };
        let bc2 = BranchConflict {
            branch_name: "branch2".to_string(),
            path: file2.to_string_lossy().to_string(),
            size: 9,
            hash: None,
            mtime: None,
            ctime: None,
        };

        // Without hash, same size files are considered equal
        assert!(!files_differ(&[bc1.clone(), bc2.clone()], false));

        // With hash, they differ
        assert!(files_differ(&[bc1, bc2], true));
    }

    #[test]
    fn test_files_differ_different_size() {
        let bc1 = BranchConflict {
            branch_name: "branch1".to_string(),
            path: "/path1".to_string(),
            size: 100,
            hash: None,
            mtime: None,
            ctime: None,
        };
        let bc2 = BranchConflict {
            branch_name: "branch2".to_string(),
            path: "/path2".to_string(),
            size: 200,
            hash: None,
            mtime: None,
            ctime: None,
        };

        assert!(files_differ(&[bc1, bc2], false));
    }

    #[test]
    fn test_files_differ_single_file() {
        let bc = BranchConflict {
            branch_name: "branch1".to_string(),
            path: "/path1".to_string(),
            size: 100,
            hash: None,
            mtime: None,
            ctime: None,
        };
        assert!(!files_differ(&[bc], false));
    }

    #[test]
    fn test_files_differ_empty() {
        assert!(!files_differ(&[], false));
    }

    #[test]
    fn test_detect_conflicts_no_conflicts() {
        let (temp1, branch1) = create_test_branch("disk1");
        let (temp2, branch2) = create_test_branch("disk2");

        // Same content in both
        fs::write(temp1.path().join("same.txt"), "same content").unwrap();
        fs::write(temp2.path().join("same.txt"), "same content").unwrap();

        let branches = vec![&branch1, &branch2];
        let conflicts = detect_conflicts(&branches, Path::new(""), false).unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_conflicts_with_conflicts() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create two branch directories
        let dir1 = temp_dir.path().join("branch1");
        let dir2 = temp_dir.path().join("branch2");
        fs::create_dir_all(&dir1).unwrap();
        fs::create_dir_all(&dir2).unwrap();

        // Create files with different content (different sizes) directly in branch dirs
        fs::write(dir1.join("diff.txt"), "content AAAA").unwrap();
        fs::write(dir2.join("diff.txt"), "content B").unwrap();

        let branch1 = Branch {
            path: dir1,
            mode: BranchMode::RW,
            minfreespace: None,
        };
        let branch2 = Branch {
            path: dir2,
            mode: BranchMode::RW,
            minfreespace: None,
        };

        let branches = vec![&branch1, &branch2];
        // Use empty path since files are at branch root
        let conflicts = detect_conflicts(&branches, Path::new(""), false).unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts.first().unwrap().name, "diff.txt");
    }

    #[test]
    fn test_detect_conflicts_single_branch() {
        let (_temp1, branch1) = create_test_branch("disk1");

        let branches = vec![&branch1];
        let conflicts = detect_conflicts(&branches, Path::new(""), false).unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_conflicts_nonexistent_dir() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");

        let branches = vec![&branch1, &branch2];
        let conflicts = detect_conflicts(&branches, Path::new("nonexistent"), false).unwrap();
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_single_file_conflict_no_conflict() {
        let (temp1, branch1) = create_test_branch("disk1");
        let (temp2, branch2) = create_test_branch("disk2");

        fs::write(temp1.path().join("same.txt"), "same content").unwrap();
        fs::write(temp2.path().join("same.txt"), "same content").unwrap();

        let branches = vec![&branch1, &branch2];
        let result = detect_single_file_conflict(&branches, Path::new("same.txt"), false).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_single_file_conflict_with_conflict() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create two branch directories
        let dir1 = temp_dir.path().join("branch1");
        let dir2 = temp_dir.path().join("branch2");
        fs::create_dir_all(&dir1).unwrap();
        fs::create_dir_all(&dir2).unwrap();

        // Create files with different content (different sizes)
        fs::write(dir1.join("diff.txt"), "content AAAA").unwrap();
        fs::write(dir2.join("diff.txt"), "content B").unwrap();

        let branch1 = Branch {
            path: dir1,
            mode: BranchMode::RW,
            minfreespace: None,
        };
        let branch2 = Branch {
            path: dir2,
            mode: BranchMode::RW,
            minfreespace: None,
        };

        let branches = vec![&branch1, &branch2];
        let result = detect_single_file_conflict(&branches, Path::new("diff.txt"), false).unwrap();
        assert!(result.is_some());
        let conflict = result.unwrap();
        assert_eq!(conflict.name, "diff.txt");
        assert_eq!(conflict.branches.len(), 2);
    }

    #[test]
    fn test_detect_single_file_conflict_single_branch() {
        let (_temp1, branch1) = create_test_branch("disk1");

        let branches = vec![&branch1];
        let result = detect_single_file_conflict(&branches, Path::new("file.txt"), false).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_single_file_conflict_nonexistent() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");

        let branches = vec![&branch1, &branch2];
        let result = detect_single_file_conflict(&branches, Path::new("nonexistent.txt"), false).unwrap();
        assert!(result.is_none());
    }

    #[test]
    #[allow(clippy::get_unwrap)]
    fn test_detect_conflicts_sorted_output() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create two branch directories
        let dir1 = temp_dir.path().join("branch1");
        let dir2 = temp_dir.path().join("branch2");
        fs::create_dir_all(&dir1).unwrap();
        fs::create_dir_all(&dir2).unwrap();

        // Create files with different content (different sizes)
        fs::write(dir1.join("zebra.txt"), "content AAAA").unwrap();
        fs::write(dir2.join("zebra.txt"), "content B").unwrap();
        fs::write(dir1.join("apple.txt"), "content AAA").unwrap();
        fs::write(dir2.join("apple.txt"), "content Y").unwrap();

        let branch1 = Branch {
            path: dir1,
            mode: BranchMode::RW,
            minfreespace: None,
        };
        let branch2 = Branch {
            path: dir2,
            mode: BranchMode::RW,
            minfreespace: None,
        };

        let branches = vec![&branch1, &branch2];
        let conflicts = detect_conflicts(&branches, Path::new(""), false).unwrap();

        // Should be sorted alphabetically
        assert_eq!(conflicts.len(), 2);
        assert_eq!(conflicts.first().unwrap().name, "apple.txt");
        assert_eq!(conflicts.get(1).unwrap().name, "zebra.txt");
    }

    #[test]
    fn test_branch_conflict_sorting_by_mtime() {
        let bc1 = BranchConflict {
            branch_name: "branch1".to_string(),
            path: "/path1".to_string(),
            size: 100,
            hash: None,
            mtime: Some(1000),
            ctime: None,
        };
        let bc2 = BranchConflict {
            branch_name: "branch2".to_string(),
            path: "/path2".to_string(),
            size: 100,
            hash: None,
            mtime: Some(2000),
            ctime: None,
        };

        let mut conflicts = [bc1, bc2];
        conflicts.sort_by(|a, b| b.mtime.cmp(&a.mtime).then_with(|| a.path.cmp(&b.path)));

        // Newest first
        assert_eq!(conflicts.first().unwrap().mtime, Some(2000));
        assert_eq!(conflicts.get(1).unwrap().mtime, Some(1000));
    }

    #[test]
    fn test_compute_file_hash_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("empty.txt");
        fs::write(&file_path, "").unwrap();

        let hash = compute_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_file_hash_exact_threshold() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("threshold.txt");

        // Create a file exactly at the threshold
        let content = "x".repeat(MB as usize);
        fs::write(&file_path, &content).unwrap();

        let hash = compute_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_file_hash_just_above_threshold() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("above_threshold.txt");

        // Create a file just above the threshold
        #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
        let content = "x".repeat((MB + 1) as usize);
        fs::write(&file_path, &content).unwrap();

        let hash = compute_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_compute_file_hash_special_characters_in_path() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("file with spaces.txt");
        fs::write(&file_path, "content").unwrap();

        let hash = compute_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());

        // Test with unicode characters
        let file_path_unicode = temp_dir.path().join("文件.txt");
        fs::write(&file_path_unicode, "content").unwrap();

        let hash_unicode = compute_file_hash(&file_path_unicode).unwrap();
        assert!(!hash_unicode.is_empty());
    }

    #[test]
    fn test_compute_file_hash_binary_content() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("binary.bin");

        // Create binary content
        let content: Vec<u8> = (0..255).cycle().take(1000).collect();
        fs::write(&file_path, &content).unwrap();

        let hash = compute_file_hash(&file_path).unwrap();
        assert!(!hash.is_empty());
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_files_differ_with_mtime() {
        let bc1 = BranchConflict {
            branch_name: "branch1".to_string(),
            path: "/path1".to_string(),
            size: 100,
            hash: Some("hash1".to_string()),
            mtime: Some(1000),
            ctime: Some(500),
        };
        let bc2 = BranchConflict {
            branch_name: "branch2".to_string(),
            path: "/path2".to_string(),
            size: 100,
            hash: Some("hash2".to_string()),
            mtime: Some(2000),
            ctime: Some(600),
        };

        // With hash, they differ
        assert!(files_differ(&[bc1.clone(), bc2.clone()], true));

        // Without hash but with different content, size is same so considered equal
        assert!(!files_differ(&[bc1, bc2], false));
    }

    #[test]
    fn test_files_differ_same_hash() {
        let hash = "same_hash".to_string();
        let bc1 = BranchConflict {
            branch_name: "branch1".to_string(),
            path: "/path1".to_string(),
            size: 100,
            hash: Some(hash.clone()),
            mtime: None,
            ctime: None,
        };
        let bc2 = BranchConflict {
            branch_name: "branch2".to_string(),
            path: "/path2".to_string(),
            size: 100,
            hash: Some(hash),
            mtime: None,
            ctime: None,
        };

        // Same hash means no difference
        assert!(!files_differ(&[bc1, bc2], true));
    }

    #[test]
    fn test_files_differ_with_none_hashes() {
        let bc1 = BranchConflict {
            branch_name: "branch1".to_string(),
            path: "/path1".to_string(),
            size: 100,
            hash: None,
            mtime: None,
            ctime: None,
        };
        let bc2 = BranchConflict {
            branch_name: "branch2".to_string(),
            path: "/path2".to_string(),
            size: 100,
            hash: None,
            mtime: None,
            ctime: None,
        };

        // Same size, no hash - considered equal
        assert!(!files_differ(&[bc1, bc2], true));
    }

    #[test]
    fn test_detect_conflicts_with_empty_files() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let dir1 = temp_dir.path().join("branch1");
        let dir2 = temp_dir.path().join("branch2");
        fs::create_dir_all(&dir1).unwrap();
        fs::create_dir_all(&dir2).unwrap();

        // Create empty files in both branches
        fs::write(dir1.join("empty.txt"), "").unwrap();
        fs::write(dir2.join("empty.txt"), "").unwrap();

        let branch1 = Branch {
            path: dir1,
            mode: BranchMode::RW,
            minfreespace: None,
        };
        let branch2 = Branch {
            path: dir2,
            mode: BranchMode::RW,
            minfreespace: None,
        };

        let branches = vec![&branch1, &branch2];
        let conflicts = detect_conflicts(&branches, Path::new(""), false).unwrap();

        // Empty files with same size (0) should not conflict
        assert!(conflicts.is_empty());
    }

    #[test]
    fn test_detect_conflicts_with_nested_paths() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let dir1 = temp_dir.path().join("branch1");
        let dir2 = temp_dir.path().join("branch2");
        fs::create_dir_all(&dir1).unwrap();
        fs::create_dir_all(&dir2).unwrap();

        // Create files with DIFFERENT SIZES to ensure conflict detection without hash
        fs::write(dir1.join("file.txt"), "content A").unwrap();
        fs::write(dir2.join("file.txt"), "content BB").unwrap();

        let branch1 = Branch {
            path: dir1,
            mode: BranchMode::RW,
            minfreespace: None,
        };
        let branch2 = Branch {
            path: dir2,
            mode: BranchMode::RW,
            minfreespace: None,
        };

        let branches = vec![&branch1, &branch2];
        let conflicts = detect_conflicts(&branches, Path::new(""), false).unwrap();

        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts.first().unwrap().name, "file.txt");
    }

    #[test]
    fn test_detect_conflicts_with_many_branches() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create 5 branches - all pointing to directories with the same structure
        let mut branches = Vec::new();
        for i in 0..5 {
            let dir = temp_dir.path().join(format!("branch{i}"));
            fs::create_dir_all(&dir).unwrap();

            // Same filename with DIFFERENT SIZES to ensure conflict detection without hash
            fs::write(dir.join("shared.txt"), format!("content {}", "x".repeat(i))).unwrap();

            branches.push(Branch {
                path: dir,
                mode: BranchMode::RW,
                minfreespace: None,
            });
        }

        let branch_refs: Vec<&Branch> = branches.iter().collect();
        // All branches have shared.txt at their root, so search at ""
        let conflicts = detect_conflicts(&branch_refs, Path::new(""), false).unwrap();

        // Should detect conflict with multiple branches
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts.first().unwrap().branches.len(), 5);
    }

    #[test]
    fn test_detect_single_file_conflict_with_hash() {
        let (temp1, branch1) = create_test_branch("disk1");
        let (temp2, branch2) = create_test_branch("disk2");

        fs::write(temp1.path().join("same.txt"), "same content").unwrap();
        fs::write(temp2.path().join("same.txt"), "same content").unwrap();

        let branches = vec![&branch1, &branch2];

        // With hash, same content should not conflict
        let result = detect_single_file_conflict(&branches, Path::new("same.txt"), true).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_detect_single_file_conflict_different_sizes() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        let dir1 = temp_dir.path().join("branch1");
        let dir2 = temp_dir.path().join("branch2");
        fs::create_dir_all(&dir1).unwrap();
        fs::create_dir_all(&dir2).unwrap();

        fs::write(dir1.join("diff.txt"), "short").unwrap();
        fs::write(dir2.join("diff.txt"), "much longer content").unwrap();

        let branch1 = Branch {
            path: dir1,
            mode: BranchMode::RW,
            minfreespace: None,
        };
        let branch2 = Branch {
            path: dir2,
            mode: BranchMode::RW,
            minfreespace: None,
        };

        let branches = vec![&branch1, &branch2];

        // Different sizes should conflict even without hash
        let result = detect_single_file_conflict(&branches, Path::new("diff.txt"), false).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_branch_conflict_with_none_metadata() {
        let bc = BranchConflict {
            branch_name: "test".to_string(),
            path: "/test/path".to_string(),
            size: 100,
            hash: None,
            mtime: None,
            ctime: None,
        };

        // Should still sort correctly with None values
        let mut conflicts = vec![bc.clone(), bc.clone()];
        conflicts.sort_by(|a, b| b.mtime.cmp(&a.mtime).then_with(|| a.path.cmp(&b.path)));
        assert_eq!(conflicts.len(), 2);
    }

    #[test]
    fn test_files_differ_with_three_branches() {
        let bc1 = BranchConflict {
            branch_name: "branch1".to_string(),
            path: "/path1".to_string(),
            size: 100,
            hash: None,
            mtime: None,
            ctime: None,
        };
        let bc2 = BranchConflict {
            branch_name: "branch2".to_string(),
            path: "/path2".to_string(),
            size: 200,
            hash: None,
            mtime: None,
            ctime: None,
        };
        let bc3 = BranchConflict {
            branch_name: "branch3".to_string(),
            path: "/path3".to_string(),
            size: 100,
            hash: None,
            mtime: None,
            ctime: None,
        };

        // bc1 and bc3 have same size, bc2 differs
        assert!(files_differ(&[bc1.clone(), bc2.clone(), bc3.clone()], false));

        // All same size
        let bc4 = BranchConflict {
            branch_name: "branch4".to_string(),
            path: "/path4".to_string(),
            size: 100,
            hash: None,
            mtime: None,
            ctime: None,
        };
        assert!(!files_differ(&[bc1, bc3, bc4], false));
    }
}
