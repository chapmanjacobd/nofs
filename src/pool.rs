//! Pool management for nofs
//! 
//! A pool is a union of multiple branches.

use std::path::{Path, PathBuf};
use crate::branch::Branch;
use crate::config::{Config, PoolConfig};
use crate::error::{NofsError, Result};
use crate::policy::{parse_size, Policy};

/// Represents a union pool of branches
pub struct Pool {
    /// Name of the pool
    pub name: Option<String>,
    
    /// Mount point path
    pub mountpoint: Option<PathBuf>,
    
    /// Branches in the pool
    pub branches: Vec<Branch>,
    
    /// Default create policy
    pub create_policy: Policy,
    
    /// Default search policy
    pub search_policy: Policy,
    
    /// Default action policy
    pub action_policy: Policy,
    
    /// Minimum free space threshold
    pub minfreespace: u64,
}

impl Pool {
    /// Create a pool from a configuration file
    pub fn from_config<P: AsRef<Path>>(config_path: P) -> Result<Self> {
        let config = Config::from_file(config_path)?;
        let pool_config = config.first_pool()?;
        Self::from_pool_config(pool_config)
    }

    /// Try to load from default config locations
    pub fn from_default_config() -> Result<Self> {
        let config_path = crate::config::find_default_config()
            .ok_or_else(|| NofsError::Config(
                "No configuration file found. Use --config or --paths.".to_string()
            ))?;
        
        Self::from_config(&config_path)
    }

    /// Create a pool from ad-hoc paths string
    /// Format: "/path1,/path2" or "/path1=RW,/path2=RO"
    pub fn from_paths(paths_str: &str, policy: &str, minfreespace: &str) -> Result<Self> {
        let branches: Result<Vec<Branch>> = paths_str
            .split(',')
            .map(|s| Branch::from_str(s.trim()))
            .collect();
        
        let branches = branches?;
        
        if branches.is_empty() {
            return Err(NofsError::Config("No branches provided".to_string()));
        }

        Ok(Pool {
            name: None,
            mountpoint: None,
            branches,
            create_policy: Policy::from_str(policy)?,
            search_policy: Policy::Ff,
            action_policy: Policy::EpAll,
            minfreespace: parse_size(minfreespace)?,
        })
    }

    /// Create a pool from a pool configuration
    fn from_pool_config(config: &PoolConfig) -> Result<Self> {
        let branches = config.get_branches()?;
        
        if branches.is_empty() {
            return Err(NofsError::Config("No branches defined in pool".to_string()));
        }

        Ok(Pool {
            name: config.name.clone(),
            mountpoint: config.mountpoint.as_ref().map(PathBuf::from),
            create_policy: Policy::from_str(&config.create_policy)?,
            search_policy: Policy::from_str(&config.search_policy)?,
            action_policy: Policy::from_str(&config.action_policy)?,
            minfreespace: parse_size(&config.minfreespace)?,
            branches,
        })
    }

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
