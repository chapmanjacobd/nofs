//! Pool management for nofs
//!
//! A pool is a share of multiple branches.

use crate::branch::Branch;
use crate::cache::OperationCache;
use crate::config::Config;
use crate::error::{NofsError, Result};
use crate::policy::Policy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents a share pool of branches
#[non_exhaustive]
pub struct Pool {
    /// Name of the pool/context
    pub name: String,

    /// Branches in the pool
    pub branches: Vec<Branch>,

    /// Create policy
    pub create_policy: Policy,

    /// Search policy
    pub search_policy: Policy,

    /// Action policy
    pub action_policy: Policy,

    /// Minimum free space threshold
    pub minfreespace: u64,
}

/// Pool manager - holds multiple named pools
pub struct PoolManager {
    /// Map of pool names to pool instances
    pools: HashMap<String, Pool>,
}

impl PoolManager {
    /// Create pool manager from a configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if the config file cannot be read or parsed.
    pub fn from_config<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let config = Config::from_file(config_path)?;
        Self::from_config_inner(&config)
    }

    /// Try to load from default config locations
    ///
    /// # Errors
    ///
    /// Returns an error if no default config is found.
    pub fn from_default_config() -> Result<Self> {
        let config_path = crate::config::find_default_config()
            .ok_or_else(|| NofsError::Config("No configuration file found. Use --config or --paths.".to_string()))?;

        Self::from_config(&config_path)
    }

    /// Create pool manager from ad-hoc paths string (uses "default" context)
    ///
    /// # Errors
    ///
    /// Returns an error if branches cannot be parsed or if no branches are provided.
    pub fn from_paths(paths_str: &str, policy: &str, minfreespace: &str) -> Result<Self> {
        let branches_result: Result<Vec<Branch>> = paths_str.split(',').map(|s| Branch::parse(s.trim())).collect();

        let branches = branches_result?;

        if branches.is_empty() {
            return Err(NofsError::Config("No branches provided".to_string()));
        }

        let pool = Pool {
            name: "default".to_string(),
            branches,
            create_policy: Policy::parse(policy)?,
            search_policy: Policy::Ff,
            action_policy: Policy::EpAll,
            minfreespace: crate::policy::parse_size(minfreespace)?,
        };

        let mut pools = HashMap::new();
        pools.insert("default".to_string(), pool);

        Ok(PoolManager { pools })
    }

    /// Create pool manager from config
    fn from_config_inner(config: &Config) -> Result<Self> {
        let mut pools = HashMap::new();

        for (name, share_config) in &config.share {
            let branches = share_config.get_branches()?;

            if branches.is_empty() {
                return Err(NofsError::Config(format!("No branches defined in share '{name}'")));
            }

            let pool = Pool {
                name: name.clone(),
                branches,
                create_policy: share_config.create_policy()?,
                search_policy: share_config.search_policy()?,
                action_policy: share_config.action_policy()?,
                minfreespace: share_config.minfreespace_bytes()?,
            };

            pools.insert(name.clone(), pool);
        }

        if pools.is_empty() {
            return Err(NofsError::Config("No shares defined in config".to_string()));
        }

        Ok(PoolManager { pools })
    }

    /// Get a pool by name
    ///
    /// # Errors
    ///
    /// Returns an error if the pool is not found.
    pub fn get_pool(&self, name: &str) -> Result<&Pool> {
        self.pools
            .get(name)
            .ok_or_else(|| NofsError::Config(format!("Share '{name}' not found")))
    }

    /// Get the first/default pool
    ///
    /// # Errors
    ///
    /// Returns an error if no pools are available.
    pub fn default_pool(&self) -> Result<&Pool> {
        self.pools
            .get("default")
            .or_else(|| self.pools.iter().next().map(|(_, pool)| pool))
            .ok_or_else(|| NofsError::Config("No pools available".to_string()))
    }

    /// Get all pool names
    #[must_use]
    pub fn pool_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.pools.keys().map(std::string::String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// Parse a context:path reference and return the appropriate pool
    ///
    /// # Errors
    ///
    /// Returns an error if the context is not found.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn resolve_context_path<'ctx>(&'ctx self, input: &'ctx str) -> Result<(&'ctx Pool, &'ctx str)> {
        if let Some(colon_idx) = input.find(':') {
            let context = &input[..colon_idx];
            // Strip leading `/` to make path relative to share root
            let path = input[colon_idx + 1..].trim_start_matches('/');
            let pool = self.get_pool(context)?;
            Ok((pool, path))
        } else {
            // No context specified, use default pool
            // Strip leading `/` to make path relative to share root
            let pool = self.default_pool()?;
            let path = input.trim_start_matches('/');
            Ok((pool, path))
        }
    }
}

impl Pool {
    /// Get total available space across all RW branches
    #[must_use]
    pub fn total_available_space(&self) -> u64 {
        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for branch_ref in &self.branches {
                if branch_ref.can_create() {
                    let branch = branch_ref.clone();
                    handles.push(s.spawn(move || branch.available_space().ok()));
                }
            }
            let mut total: u64 = 0;
            for handle in handles {
                if let Some(space) = handle.join().ok().flatten() {
                    total = total.saturating_add(space);
                }
            }
            total
        })
    }

    /// Get total space across all branches
    #[must_use]
    pub fn total_space(&self) -> u64 {
        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for branch_ref in &self.branches {
                let branch = branch_ref.clone();
                handles.push(s.spawn(move || branch.total_space().ok()));
            }
            let mut total: u64 = 0;
            for handle in handles {
                if let Some(space) = handle.join().ok().flatten() {
                    total = total.saturating_add(space);
                }
            }
            total
        })
    }

    /// Get total used space across all branches
    #[must_use]
    #[allow(clippy::arithmetic_side_effects)]
    pub fn total_used_space(&self) -> u64 {
        self.total_space() - self.total_available_space()
    }

    /// Get number of branches
    #[must_use]
    pub const fn branch_count(&self) -> usize {
        self.branches.len()
    }

    /// Get number of writable branches
    #[must_use]
    pub fn writable_branch_count(&self) -> usize {
        self.branches.iter().filter(|b| b.can_create()).count()
    }

    /// Find a branch by path
    #[must_use]
    pub fn find_branch(&self, path: &Path) -> Option<&Branch> {
        self.branches.iter().find(|b| b.path == path)
    }

    /// Resolve a pool path to actual branch paths
    /// Returns all branches where the path exists
    #[must_use]
    pub fn resolve_path(&self, pool_path: &Path) -> Vec<PathBuf> {
        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for branch_ref in &self.branches {
                let branch = branch_ref.clone();
                handles.push(s.spawn(move || {
                    let full_path = branch.path.join(pool_path);
                    full_path.exists().then_some(full_path)
                }));
            }

            let mut results = Vec::new();
            for handle in handles {
                if let Some(path) = handle.join().ok().flatten() {
                    results.push(path);
                }
            }
            results
        })
    }

    /// Find the first branch where a path exists
    #[must_use]
    pub fn resolve_path_first(&self, pool_path: &Path) -> Option<PathBuf> {
        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for branch_ref in &self.branches {
                let branch = branch_ref.clone();
                handles.push(s.spawn(move || {
                    let full_path = branch.path.join(pool_path);
                    full_path.exists().then_some(full_path)
                }));
            }

            for handle in handles {
                if let Some(path) = handle.join().ok().flatten() {
                    return Some(path);
                }
            }
            None
        })
    }

    /// Get the best branch for creating a file at the given path
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable branch is found.
    pub fn select_create_branch(&self, relative_path: &Path) -> Result<&Branch> {
        use crate::policy::CreatePolicy;

        let policy = CreatePolicy::new(&self.branches, self.minfreespace);
        policy.select(self.create_policy, Some(relative_path))
    }

    /// Get all branches containing a path
    #[must_use]
    pub fn find_all_branches(&self, relative_path: &Path) -> Vec<&Branch> {
        use crate::policy::SearchPolicy;

        let search = SearchPolicy::new(&self.branches);
        search.find_all(relative_path)
    }

    /// Check if a path exists in the pool
    #[must_use]
    pub fn exists(&self, pool_path: &Path) -> bool {
        self.branches.iter().any(|b| b.path.join(pool_path).exists())
    }

    // Note: The following cached methods are duplicated from the non-cached versions above.
    // This duplication is intentional - the cached and non-cached versions have different
    // signatures (cached versions take &OperationCache) and call different branch methods.
    // Consolidating them would require trait objects or complex generics, which would add
    // runtime overhead that defeats the purpose of caching.

    /// Get total available space across all RW branches (cached)
    #[must_use]
    pub fn total_available_space_cached(&self, cache: &OperationCache) -> u64 {
        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for branch_ref in &self.branches {
                if branch_ref.can_create() {
                    let branch = branch_ref.clone();
                    handles.push(s.spawn(move || branch.available_space_cached(cache).ok()));
                }
            }
            let mut total: u64 = 0;
            for handle in handles {
                if let Some(space) = handle.join().ok().flatten() {
                    total = total.saturating_add(space);
                }
            }
            total
        })
    }

    /// Get total space across all branches (cached)
    #[must_use]
    pub fn total_space_cached(&self, cache: &OperationCache) -> u64 {
        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for branch_ref in &self.branches {
                let branch = branch_ref.clone();
                handles.push(s.spawn(move || branch.total_space_cached(cache).ok()));
            }
            let mut total: u64 = 0;
            for handle in handles {
                if let Some(space) = handle.join().ok().flatten() {
                    total = total.saturating_add(space);
                }
            }
            total
        })
    }

    /// Get total used space across all branches (cached)
    #[must_use]
    #[allow(clippy::arithmetic_side_effects)]
    pub fn total_used_space_cached(&self, cache: &OperationCache) -> u64 {
        self.total_space_cached(cache)
            .saturating_sub(self.total_available_space_cached(cache))
    }

    /// Resolve a pool path to actual branch paths (cached)
    /// Returns all branches where the path exists
    #[must_use]
    pub fn resolve_path_cached(&self, pool_path: &Path, cache: &OperationCache) -> Vec<PathBuf> {
        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for branch_ref in &self.branches {
                let branch = branch_ref.clone();
                handles.push(s.spawn(move || {
                    branch
                        .path_exists_cached(pool_path, cache)
                        .then(|| branch.path.join(pool_path))
                }));
            }

            let mut results = Vec::new();
            for handle in handles {
                if let Some(path) = handle.join().ok().flatten() {
                    results.push(path);
                }
            }
            results
        })
    }

    /// Find the first branch where a path exists (cached)
    #[must_use]
    pub fn resolve_path_first_cached(&self, pool_path: &Path, cache: &OperationCache) -> Option<PathBuf> {
        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for branch_ref in &self.branches {
                let branch = branch_ref.clone();
                handles.push(s.spawn(move || {
                    branch
                        .path_exists_cached(pool_path, cache)
                        .then(|| branch.path.join(pool_path))
                }));
            }

            for handle in handles {
                if let Some(path) = handle.join().ok().flatten() {
                    return Some(path);
                }
            }
            None
        })
    }

    /// Get the best branch for creating a file at the given path (cached)
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable branch is found.
    pub fn select_create_branch_cached<'a>(
        &'a self,
        relative_path: &Path,
        cache: &'a OperationCache,
    ) -> Result<&'a Branch> {
        use crate::policy::CreatePolicy;

        let policy = CreatePolicy::with_cache(&self.branches, self.minfreespace, cache);
        policy.select(self.create_policy, Some(relative_path))
    }

    /// Get all branches containing a path (cached)
    #[must_use]
    pub fn find_all_branches_cached<'a>(&'a self, relative_path: &Path, cache: &'a OperationCache) -> Vec<&'a Branch> {
        use crate::policy::SearchPolicy;

        let search = SearchPolicy::with_cache(&self.branches, cache);
        search.find_all(relative_path)
    }

    /// Check if a path exists in the pool (cached)
    #[must_use]
    pub fn exists_cached(&self, pool_path: &Path, cache: &OperationCache) -> bool {
        self.branches.iter().any(|b| b.path_exists_cached(pool_path, cache))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::branch::BranchMode;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_pool() -> (TempDir, Pool) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let branch_path = temp_dir.path().join("branch1");
        fs::create_dir_all(&branch_path).unwrap();

        let branch = Branch {
            path: branch_path,
            mode: BranchMode::RW,
            minfreespace: None,
        };

        let pool = Pool {
            name: "test_pool".to_string(),
            branches: vec![branch],
            create_policy: Policy::Mfs,
            search_policy: Policy::Ff,
            action_policy: Policy::EpAll,
            minfreespace: 0,
        };

        (temp_dir, pool)
    }

    #[test]
    fn test_pool_total_available_space() {
        let (_temp, pool) = create_test_pool();
        let space = pool.total_available_space();
        assert!(space > 0);
    }

    #[test]
    fn test_pool_total_space() {
        let (_temp, pool) = create_test_pool();
        let space = pool.total_space();
        assert!(space > 0);
    }

    #[test]
    fn test_pool_total_used_space() {
        let (_temp, pool) = create_test_pool();
        let used = pool.total_used_space();
        // Note: used space can be 0 on empty filesystems
        let _ = used;
    }

    #[test]
    fn test_pool_branch_count() {
        let (_temp, pool) = create_test_pool();
        assert_eq!(pool.branch_count(), 1);
    }

    #[test]
    fn test_pool_writable_branch_count() {
        let (_temp, pool) = create_test_pool();
        assert_eq!(pool.writable_branch_count(), 1);
    }

    #[test]
    fn test_pool_writable_branch_count_with_ro() {
        let (temp, mut pool) = create_test_pool();
        let branch_path = temp.path().join("branch2");
        fs::create_dir_all(&branch_path).unwrap();

        pool.branches.push(Branch {
            path: branch_path,
            mode: BranchMode::RO,
            minfreespace: None,
        });

        assert_eq!(pool.writable_branch_count(), 1);
        assert_eq!(pool.branch_count(), 2);
    }

    #[test]
    fn test_pool_find_branch() {
        let (temp, pool) = create_test_pool();
        let found = pool.find_branch(&temp.path().join("branch1"));
        assert!(found.is_some());

        let not_found = pool.find_branch(&PathBuf::from("/nonexistent"));
        assert!(not_found.is_none());
    }

    #[test]
    fn test_pool_resolve_path() {
        let (temp, pool) = create_test_pool();
        let file_path = temp.path().join("branch1").join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let resolved = pool.resolve_path(Path::new("test.txt"));
        assert_eq!(resolved.len(), 1);
        assert!(resolved.first().unwrap().exists());
    }

    #[test]
    fn test_pool_resolve_path_first() {
        let (temp, pool) = create_test_pool();
        let file_path = temp.path().join("branch1").join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let resolved = pool.resolve_path_first(Path::new("test.txt"));
        assert!(resolved.is_some());
        assert!(resolved.unwrap().exists());
    }

    #[test]
    fn test_pool_resolve_path_not_found() {
        let (_temp, pool) = create_test_pool();
        let resolved = pool.resolve_path(Path::new("nonexistent.txt"));
        assert!(resolved.is_empty());

        let resolved_first = pool.resolve_path_first(Path::new("nonexistent.txt"));
        assert!(resolved_first.is_none());
    }

    #[test]
    fn test_pool_select_create_branch() {
        let (_temp, pool) = create_test_pool();
        let branch = pool.select_create_branch(Path::new("newfile.txt"));
        assert!(branch.is_ok());
    }

    #[test]
    fn test_pool_find_all_branches() {
        let (temp, pool) = create_test_pool();
        let file_path = temp.path().join("branch1").join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let branches = pool.find_all_branches(Path::new("test.txt"));
        assert_eq!(branches.len(), 1);
    }

    #[test]
    fn test_pool_exists() {
        let (temp, pool) = create_test_pool();
        let file_path = temp.path().join("branch1").join("test.txt");
        fs::write(&file_path, "content").unwrap();

        assert!(pool.exists(Path::new("test.txt")));
        assert!(!pool.exists(Path::new("nonexistent.txt")));
    }

    #[test]
    fn test_pool_cached_methods() {
        let (temp, pool) = create_test_pool();
        let cache = OperationCache::new();

        let file_path = temp.path().join("branch1").join("test.txt");
        fs::write(&file_path, "content").unwrap();

        let available = pool.total_available_space_cached(&cache);
        assert!(available > 0);

        let total = pool.total_space_cached(&cache);
        assert!(total > 0);

        let used = pool.total_used_space_cached(&cache);
        // Note: used space can be 0 on empty filesystems
        let _ = used;

        let resolved = pool.resolve_path_cached(Path::new("test.txt"), &cache);
        assert_eq!(resolved.len(), 1);

        let resolved_first = pool.resolve_path_first_cached(Path::new("test.txt"), &cache);
        assert!(resolved_first.is_some());

        let exists = pool.exists_cached(Path::new("test.txt"), &cache);
        assert!(exists);

        let branches = pool.find_all_branches_cached(Path::new("test.txt"), &cache);
        assert_eq!(branches.len(), 1);
    }

    #[test]
    fn test_pool_manager_from_paths() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let branch_path = temp_dir.path().join("branch1");
        fs::create_dir_all(&branch_path).unwrap();

        let paths_str = branch_path.to_str().unwrap();
        let manager = PoolManager::from_paths(paths_str, "mfs", "0").unwrap();

        assert_eq!(manager.pool_names().len(), 1);
        let pool = manager.default_pool().unwrap();
        assert_eq!(pool.name, "default");
    }

    #[test]
    fn test_pool_manager_get_pool() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let branch_path = temp_dir.path().join("branch1");
        fs::create_dir_all(&branch_path).unwrap();

        let paths_str = branch_path.to_str().unwrap();
        let manager = PoolManager::from_paths(paths_str, "mfs", "0").unwrap();

        let pool = manager.get_pool("default").unwrap();
        assert_eq!(pool.name, "default");

        let not_found = manager.get_pool("nonexistent");
        assert!(not_found.is_err());
    }

    #[test]
    fn test_pool_manager_resolve_context_path() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let branch_path = temp_dir.path().join("branch1");
        fs::create_dir_all(&branch_path).unwrap();

        let paths_str = branch_path.to_str().unwrap();
        let manager = PoolManager::from_paths(paths_str, "mfs", "0").unwrap();

        let (pool, path) = manager.resolve_context_path("default:some/path.txt").unwrap();
        assert_eq!(pool.name, "default");
        assert_eq!(path, "some/path.txt");

        let (pool2, path2) = manager.resolve_context_path("/some/path.txt").unwrap();
        assert_eq!(pool2.name, "default");
        assert_eq!(path2, "some/path.txt");
    }

    #[test]
    fn test_pool_manager_resolve_context_path_not_found() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let branch_path = temp_dir.path().join("branch1");
        fs::create_dir_all(&branch_path).unwrap();

        let paths_str = branch_path.to_str().unwrap();
        let manager = PoolManager::from_paths(paths_str, "mfs", "0").unwrap();

        let result = manager.resolve_context_path("nonexistent:path.txt");
        assert!(result.is_err());
    }
}
