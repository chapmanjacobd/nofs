//! Per-operation caching for branch metadata
//!
//! Provides thread-safe caching of space information and existence checks
//! to eliminate redundant statvfs and path.exists() calls within a single
//! command execution.

use crate::branch::Branch;
use crate::error::Result;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

/// Cached space information for a branch
#[derive(Clone, Copy, Debug)]
pub struct SpaceInfo {
    /// Available space in bytes
    pub available: u64,
    /// Total space in bytes
    pub total: u64,
}

/// Per-operation cache for branch metadata
///
/// Thread-safe cache that eliminates redundant statvfs and exists() calls
/// within a single command execution (e.g., cp with 100 files).
///
/// # Example
///
/// ```
/// use nofs::cache::OperationCache;
///
/// let cache = OperationCache::new();
/// // Use cache throughout a single command execution
/// ```
#[derive(Default)]
pub struct OperationCache {
    /// Cached space values per branch path
    space_cache: RwLock<HashMap<PathBuf, SpaceInfo>>,
    /// Cached existence checks: (branch_path, relative_path) -> exists
    exists_cache: RwLock<HashMap<(PathBuf, PathBuf), bool>>,
}

impl OperationCache {
    /// Create a new empty operation cache
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new empty operation cache (alias for new())
    #[must_use]
    pub fn create() -> Self {
        Self::new()
    }

    /// Get cached space info, or None if not cached
    #[must_use]
    pub fn get_space<P: AsRef<Path>>(&self, branch_path: P) -> Option<SpaceInfo> {
        let lock = self.space_cache.read().unwrap();
        lock.get(branch_path.as_ref()).copied()
    }

    /// Cache space info for a branch
    pub fn set_space<P: Into<PathBuf>>(&self, branch_path: P, info: SpaceInfo) {
        let mut lock = self.space_cache.write().unwrap();
        lock.insert(branch_path.into(), info);
    }

    /// Get cached existence check, or None if not cached
    #[must_use]
    pub fn get_exists<B: AsRef<Path>, R: AsRef<Path>>(
        &self,
        branch_path: B,
        relative_path: R,
    ) -> Option<bool> {
        let lock = self.exists_cache.read().unwrap();
        let key = (
            branch_path.as_ref().to_path_buf(),
            relative_path.as_ref().to_path_buf(),
        );
        lock.get(&key).copied()
    }

    /// Cache existence check result
    pub fn set_exists<B: Into<PathBuf>, R: Into<PathBuf>>(
        &self,
        branch_path: B,
        relative_path: R,
        exists: bool,
    ) {
        let mut lock = self.exists_cache.write().unwrap();
        let key = (branch_path.into(), relative_path.into());
        lock.insert(key, exists);
    }

    /// Clear all cached data (optional, for explicit invalidation)
    pub fn clear(&self) {
        let mut space_lock = self.space_cache.write().unwrap();
        space_lock.clear();
        let mut exists_lock = self.exists_cache.write().unwrap();
        exists_lock.clear();
    }

    /// Get number of cached space entries (for debugging/testing)
    #[must_use]
    pub fn space_cache_len(&self) -> usize {
        let lock = self.space_cache.read().unwrap();
        lock.len()
    }

    /// Get number of cached existence entries (for debugging/testing)
    #[must_use]
    pub fn exists_cache_len(&self) -> usize {
        let lock = self.exists_cache.read().unwrap();
        lock.len()
    }
}

/// Helper trait for cache-aware branch operations
pub trait CachedBranch {
    /// Get available space, using cache if available
    fn available_space_cached(&self, cache: &OperationCache) -> Result<u64>;

    /// Get total space, using cache if available
    fn total_space_cached(&self, cache: &OperationCache) -> Result<u64>;

    /// Check path existence with caching
    fn path_exists_cached(&self, relative_path: &Path, cache: &OperationCache) -> bool;
}

impl CachedBranch for Branch {
    /// Get available space, using cache if available
    fn available_space_cached(&self, cache: &OperationCache) -> Result<u64> {
        if let Some(cached) = cache.get_space(&self.path) {
            return Ok(cached.available);
        }

        let available = self.available_space()?;

        // Update cache with full space info if we can get total too
        if let Ok(total) = self.total_space() {
            cache.set_space(self.path.clone(), SpaceInfo { available, total });
        }

        Ok(available)
    }

    /// Get total space, using cache if available
    fn total_space_cached(&self, cache: &OperationCache) -> Result<u64> {
        if let Some(cached) = cache.get_space(&self.path) {
            return Ok(cached.total);
        }

        let total = self.total_space()?;

        // Update cache with full space info if we can get available too
        if let Ok(available) = self.available_space() {
            cache.set_space(self.path.clone(), SpaceInfo { available, total });
        }

        Ok(total)
    }

    /// Check path existence with caching
    fn path_exists_cached(&self, relative_path: &Path, cache: &OperationCache) -> bool {
        if let Some(cached) = cache.get_exists(&self.path, relative_path) {
            return cached;
        }

        let exists = self.path.join(relative_path).exists();
        cache.set_exists(self.path.clone(), relative_path, exists);
        exists
    }
}

/// Arc-wrapped cache for easy sharing across threads
pub type SharedCache = Arc<OperationCache>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn setup_test_branch() -> (Branch, PathBuf) {
        let test_dir = std::env::temp_dir().join("nofs_test_cache_branch");
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let branch = Branch {
            path: test_dir.clone(),
            mode: crate::branch::BranchMode::RW,
            minfreespace: None,
        };

        (branch, test_dir)
    }

    fn cleanup_test_dir(test_dir: &PathBuf) {
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_cache_basic() {
        let cache = OperationCache::new();
        let test_path = PathBuf::from("/test/path");

        // Initially not cached
        assert!(cache.get_space(&test_path).is_none());

        // Set and retrieve
        let info = SpaceInfo {
            available: 1000,
            total: 2000,
        };
        cache.set_space(test_path.clone(), info);
        let retrieved = cache.get_space(&test_path).unwrap();
        assert_eq!(retrieved.available, 1000);
        assert_eq!(retrieved.total, 2000);
    }

    #[test]
    fn test_cache_exists() {
        let cache = OperationCache::new();
        let branch_path = PathBuf::from("/branch1");
        let rel_path = PathBuf::from("file.txt");

        // Initially not cached
        assert!(cache.get_exists(&branch_path, &rel_path).is_none());

        // Set and retrieve
        cache.set_exists(branch_path.clone(), rel_path.clone(), true);
        assert!(cache.get_exists(&branch_path, &rel_path).unwrap());

        cache.set_exists(branch_path.clone(), rel_path.clone(), false);
        assert!(!cache.get_exists(&branch_path, &rel_path).unwrap());
    }

    #[test]
    fn test_cache_clear() {
        let cache = OperationCache::new();
        let test_path = PathBuf::from("/test/path");

        cache.set_space(
            test_path.clone(),
            SpaceInfo {
                available: 1000,
                total: 2000,
            },
        );

        assert!(cache.get_space(&test_path).is_some());
        cache.clear();
        assert!(cache.get_space(&test_path).is_none());
    }

    #[test]
    fn test_cached_branch_space() {
        let (branch, test_dir) = setup_test_branch();
        let cache = OperationCache::new();

        // First call should populate cache
        let available = branch.available_space_cached(&cache).unwrap();
        assert!(available > 0);
        assert_eq!(cache.space_cache_len(), 1);

        // Second call should use cache
        let available2 = branch.available_space_cached(&cache).unwrap();
        assert_eq!(available, available2);

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_cached_branch_exists() {
        let (branch, test_dir) = setup_test_branch();
        let cache = OperationCache::new();

        // Create a test file
        let test_file = test_dir.join("test.txt");
        fs::write(&test_file, "content").unwrap();

        // First call should populate cache
        assert!(branch.path_exists_cached(Path::new("test.txt"), &cache));
        assert_eq!(cache.exists_cache_len(), 1);

        // Second call should use cache
        assert!(branch.path_exists_cached(Path::new("test.txt"), &cache));

        // Non-existent file
        assert!(!branch.path_exists_cached(Path::new("nonexistent.txt"), &cache));

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_cache_thread_safety() {
        use std::thread;

        let cache = Arc::new(OperationCache::new());
        let test_path = PathBuf::from("/test/path");

        let mut handles = vec![];

        // Spawn multiple threads to write and read
        for i in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let path_clone = test_path.clone();
            let handle = thread::spawn(move || {
                let info = SpaceInfo {
                    available: i * 100,
                    total: i * 200,
                };
                cache_clone.set_space(path_clone.clone(), info);
                let retrieved = cache_clone.get_space(&path_clone);
                retrieved.unwrap().available
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.join();
        }

        // Should have one entry (last write wins)
        assert_eq!(cache.space_cache_len(), 1);
    }
}
