//! Pool management for nofs
//!
//! A pool is a union of multiple branches.

use crate::branch::Branch;
use crate::config::Config;
use crate::error::{NofsError, Result};
use crate::policy::Policy;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Represents a union pool of branches
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
    pools: HashMap<String, Pool>,
}

impl PoolManager {
    /// Create pool manager from a configuration file
    pub fn from_config<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let config = Config::from_file(config_path)?;
        Self::from_config_inner(&config)
    }

    /// Try to load from default config locations
    pub fn from_default_config() -> Result<Self> {
        let config_path = crate::config::find_default_config().ok_or_else(|| {
            NofsError::Config("No configuration file found. Use --config or --paths.".to_string())
        })?;

        Self::from_config(&config_path)
    }

    /// Create pool manager from ad-hoc paths string (uses "default" context)
    pub fn from_paths(paths_str: &str, policy: &str, minfreespace: &str) -> Result<Self> {
        let branches: Result<Vec<Branch>> = paths_str
            .split(',')
            .map(|s| Branch::from_str(s.trim()))
            .collect();

        let branches = branches?;

        if branches.is_empty() {
            return Err(NofsError::Config("No branches provided".to_string()));
        }

        let pool = Pool {
            name: "default".to_string(),
            branches,
            create_policy: Policy::from_str(policy)?,
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

        for (name, union_config) in &config.union {
            let branches = union_config.get_branches()?;

            if branches.is_empty() {
                return Err(NofsError::Config(format!(
                    "No branches defined in union '{}'",
                    name
                )));
            }

            let pool = Pool {
                name: name.clone(),
                branches,
                create_policy: union_config.create_policy()?,
                search_policy: union_config.search_policy()?,
                action_policy: union_config.action_policy()?,
                minfreespace: union_config.minfreespace_bytes()?,
            };

            pools.insert(name.clone(), pool);
        }

        if pools.is_empty() {
            return Err(NofsError::Config(
                "No union contexts defined in config".to_string(),
            ));
        }

        Ok(PoolManager { pools })
    }

    /// Get a pool by name
    pub fn get_pool(&self, name: &str) -> Result<&Pool> {
        self.pools
            .get(name)
            .ok_or_else(|| NofsError::Config(format!("Union context '{}' not found", name)))
    }

    /// Get the first/default pool
    pub fn default_pool(&self) -> Result<&Pool> {
        if let Some(pool) = self.pools.get("default") {
            Ok(pool)
        } else if let Some((_, pool)) = self.pools.iter().next() {
            Ok(pool)
        } else {
            Err(NofsError::Config("No pools available".to_string()))
        }
    }

    /// Get all pool names
    pub fn pool_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.pools.keys().map(|s| s.as_str()).collect();
        names.sort();
        names
    }

    /// Parse a context:path reference and return the appropriate pool
    pub fn resolve_context_path<'a>(&'a self, input: &'a str) -> Result<(&'a Pool, &'a str)> {
        if let Some(colon_idx) = input.find(':') {
            let context = &input[..colon_idx];
            let path = &input[colon_idx + 1..];
            let pool = self.get_pool(context)?;
            Ok((pool, path))
        } else {
            // No context specified, use default pool
            let pool = self.default_pool()?;
            Ok((pool, input))
        }
    }
}

impl Pool {
    /// Get total available space across all RW branches
    pub fn total_available_space(&self) -> u64 {
        self.branches
            .iter()
            .filter(|b| b.can_create())
            .filter_map(|b| b.available_space().ok())
            .sum()
    }

    /// Get total space across all branches
    pub fn total_space(&self) -> u64 {
        self.branches
            .iter()
            .filter_map(|b| b.total_space().ok())
            .sum()
    }

    /// Get total used space across all branches
    pub fn total_used_space(&self) -> u64 {
        self.total_space() - self.total_available_space()
    }

    /// Get number of branches
    pub fn branch_count(&self) -> usize {
        self.branches.len()
    }

    /// Get number of writable branches
    pub fn writable_branch_count(&self) -> usize {
        self.branches.iter().filter(|b| b.can_create()).count()
    }

    /// Find a branch by path
    pub fn find_branch(&self, path: &Path) -> Option<&Branch> {
        self.branches.iter().find(|b| b.path == path)
    }

    /// Resolve a pool path to actual branch paths
    /// Returns all branches where the path exists
    pub fn resolve_path(&self, pool_path: &Path) -> Vec<PathBuf> {
        self.branches
            .iter()
            .filter(|b| b.path.join(pool_path).exists())
            .map(|b| b.path.join(pool_path))
            .collect()
    }

    /// Find the first branch where a path exists
    pub fn resolve_path_first(&self, pool_path: &Path) -> Option<PathBuf> {
        self.branches
            .iter()
            .find(|b| b.path.join(pool_path).exists())
            .map(|b| b.path.join(pool_path))
    }

    /// Get the best branch for creating a file at the given path
    pub fn select_create_branch(&self, relative_path: &Path) -> Result<&Branch> {
        use crate::policy::CreatePolicy;

        let policy = CreatePolicy::new(&self.branches, self.minfreespace);
        policy.select(self.create_policy, Some(relative_path))
    }

    /// Get all branches containing a path
    pub fn find_all_branches(&self, relative_path: &Path) -> Vec<&Branch> {
        use crate::policy::SearchPolicy;

        let search = SearchPolicy::new(&self.branches);
        search.find_all(relative_path)
    }

    /// Check if a path exists in the pool
    pub fn exists(&self, pool_path: &Path) -> bool {
        self.branches
            .iter()
            .any(|b| b.path.join(pool_path).exists())
    }
}
