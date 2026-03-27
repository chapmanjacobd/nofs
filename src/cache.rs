//! Per-operation caching for branch metadata
//!
//! Provides thread-safe caching of space information and existence checks
//! to eliminate redundant statvfs and `path.exists()` calls within a single
//! command execution.

use crate::branch::Branch;
use crate::error::Result;
use dashmap::DashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Cached space information for a branch
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct SpaceInfo {
    /// Available space in bytes
    pub available: u64,
    /// Total space in bytes
    pub total: u64,
}

/// Per-operation cache for branch metadata
///
/// Thread-safe cache using `DashMap` for fine-grained per-key locking.
/// This eliminates redundant statvfs and `exists()` calls within a single
/// command execution (e.g., cp with 100 files).
///
/// # Thread Safety
///
/// Unlike `RwLock<HashMap>`, `DashMap` provides per-key locking, meaning
/// concurrent access to different keys does not block. Additionally,
/// the `get_or_insert_*` methods provide atomic get-or-compute semantics,
/// preventing TOCTOU race conditions where multiple threads might
/// compute the same value simultaneously.
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
    space_cache: DashMap<PathBuf, SpaceInfo>,
    /// Cached existence checks: (`branch_path`, `relative_path`) -> exists
    exists_cache: DashMap<(PathBuf, PathBuf), bool>,
}

impl OperationCache {
    /// Create a new empty operation cache
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new empty operation cache (alias for `new()`)
    #[must_use]
    pub fn create() -> Self {
        Self::new()
    }

    /// Get cached space info, or None if not cached
    #[must_use]
    pub fn get_space<P: AsRef<Path>>(&self, branch_path: P) -> Option<SpaceInfo> {
        self.space_cache.get(branch_path.as_ref()).map(|r| *r)
    }

    /// Cache space info for a branch
    ///
    /// If the key already exists, the value is updated.
    pub fn set_space<P: Into<PathBuf>>(&self, branch_path: P, info: SpaceInfo) {
        self.space_cache.insert(branch_path.into(), info);
    }

    /// Get cached existence check, or None if not cached
    #[must_use]
    pub fn get_exists<B: AsRef<Path>, R: AsRef<Path>>(
        &self,
        branch_path: B,
        relative_path: R,
    ) -> Option<bool> {
        let key = (
            branch_path.as_ref().to_path_buf(),
            relative_path.as_ref().to_path_buf(),
        );
        self.exists_cache.get(&key).map(|r| *r)
    }

    /// Cache existence check result
    ///
    /// If the key already exists, the value is updated.
    pub fn set_exists<B: Into<PathBuf>, R: Into<PathBuf>>(
        &self,
        branch_path: B,
        relative_path: R,
        exists: bool,
    ) {
        let key = (branch_path.into(), relative_path.into());
        self.exists_cache.insert(key, exists);
    }

    /// Atomically get or compute space info
    ///
    /// The `compute` closure is called at most once per key, even under
    /// concurrent access from multiple threads. This prevents TOCTOU
    /// race conditions where multiple threads might compute the same
    /// value simultaneously.
    ///
    /// # Example
    ///
    /// ```
    /// use nofs::cache::OperationCache;
    ///
    /// let cache = OperationCache::new();
    /// let info = cache.get_or_insert_space("/branch1", || {
    ///     // Expensive computation here
    ///     SpaceInfo { available: 1000, total: 2000 }
    /// });
    /// ```
    pub fn get_or_insert_space<P, F>(&self, branch_path: P, compute: F) -> SpaceInfo
    where
        P: AsRef<Path> + Into<PathBuf> + Clone,
        F: FnOnce() -> SpaceInfo,
    {
        *self
            .space_cache
            .entry(branch_path.into())
            .or_insert_with(compute)
    }

    /// Atomically get or compute existence check
    ///
    /// The `compute` closure is called at most once per key, even under
    /// concurrent access from multiple threads.
    pub fn get_or_insert_exists<B, R, F>(
        &self,
        branch_path: &B,
        relative_path: &R,
        compute: F,
    ) -> bool
    where
        B: AsRef<Path> + ?Sized,
        R: AsRef<Path> + ?Sized,
        F: FnOnce() -> bool,
    {
        let branch_key = branch_path.as_ref().to_path_buf();
        let rel_key = relative_path.as_ref().to_path_buf();
        let key = (branch_key, rel_key);

        *self.exists_cache.entry(key).or_insert_with(compute)
    }

    /// Clear all cached data (optional, for explicit invalidation)
    pub fn clear(&self) {
        self.space_cache.clear();
        self.exists_cache.clear();
    }

    /// Get number of cached space entries (for debugging/testing)
    #[must_use]
    pub fn space_cache_len(&self) -> usize {
        self.space_cache.len()
    }

    /// Get number of cached existence entries (for debugging/testing)
    #[must_use]
    pub fn exists_cache_len(&self) -> usize {
        self.exists_cache.len()
    }
}

/// Helper trait for cache-aware branch operations
pub trait CachedBranch {
    /// Get available space, using cache if available
    ///
    /// # Errors
    ///
    /// Returns an error if the space cannot be determined.
    fn available_space_cached(&self, cache: &OperationCache) -> Result<u64>;

    /// Get total space, using cache if available
    ///
    /// # Errors
    ///
    /// Returns an error if the space cannot be determined.
    fn total_space_cached(&self, cache: &OperationCache) -> Result<u64>;

    /// Check path existence with caching
    fn path_exists_cached(&self, relative_path: &Path, cache: &OperationCache) -> bool;
}

impl CachedBranch for Branch {
    /// Get available space, using cache if available
    fn available_space_cached(&self, cache: &OperationCache) -> Result<u64> {
        let info = cache.get_or_insert_space(&self.path, || {
            // This closure runs at most once per branch, even with concurrent access
            let available = self.available_space().unwrap_or(0);
            let total = self.total_space().unwrap_or(0);
            SpaceInfo { available, total }
        });
        Ok(info.available)
    }

    /// Get total space, using cache if available
    fn total_space_cached(&self, cache: &OperationCache) -> Result<u64> {
        let info = cache.get_or_insert_space(&self.path, || {
            let available = self.available_space().unwrap_or(0);
            let total = self.total_space().unwrap_or(0);
            SpaceInfo { available, total }
        });
        Ok(info.total)
    }

    /// Check path existence with caching
    fn path_exists_cached(&self, relative_path: &Path, cache: &OperationCache) -> bool {
        cache.get_or_insert_exists(&self.path, relative_path, || {
            self.path.join(relative_path).exists()
        })
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
        let test_dir =
            std::env::temp_dir().join(format!("nofs_test_cache_branch_{}", std::process::id()));
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

    #[test]
    fn test_cache_get_or_insert_atomicity() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::thread;

        let cache = Arc::new(OperationCache::new());
        let test_path = PathBuf::from("/test/path");
        let compute_count = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];

        // Spawn multiple threads that all try to compute the same value
        for _ in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let path_clone = test_path.clone();
            let count_clone = Arc::clone(&compute_count);

            let handle = thread::spawn(move || {
                cache_clone.get_or_insert_space(path_clone, || {
                    // Increment counter to track how many times compute is called
                    count_clone.fetch_add(1, Ordering::SeqCst);
                    SpaceInfo {
                        available: 1000,
                        total: 2000,
                    }
                })
            });
            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            let _ = handle.join();
        }

        // The compute closure should have been called exactly once
        // (DashMap's entry API ensures atomicity)
        assert_eq!(compute_count.load(Ordering::SeqCst), 1);

        // All threads should see the same value
        assert_eq!(cache.get_space(&test_path).unwrap().available, 1000);
    }

    #[test]
    fn test_cache_get_or_insert_exists_atomicity() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::thread;

        let cache = Arc::new(OperationCache::new());
        let branch_path = PathBuf::from("/branch1");
        let rel_path = PathBuf::from("file.txt");
        let compute_count = Arc::new(AtomicUsize::new(0));

        let mut handles = vec![];

        for _ in 0..10 {
            let cache_clone = Arc::clone(&cache);
            let branch_clone = branch_path.clone();
            let rel_clone = rel_path.clone();
            let count_clone = Arc::clone(&compute_count);

            let handle = thread::spawn(move || {
                cache_clone.get_or_insert_exists(&branch_clone, &rel_clone, || {
                    count_clone.fetch_add(1, Ordering::SeqCst);
                    true
                })
            });
            handles.push(handle);
        }

        for handle in handles {
            let _ = handle.join();
        }

        // The compute closure should have been called exactly once
        assert_eq!(compute_count.load(Ordering::SeqCst), 1);
        assert!(cache.get_exists(&branch_path, &rel_path).unwrap());
    }
}
