//! Configuration parsing for nofs
//! 
//! Supports TOML configuration files with pool and branch definitions.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::fs;
use crate::branch::Branch;
use crate::error::{NofsError, Result};

/// Pool configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PoolConfig {
    /// Pool name (for identification)
    pub name: Option<String>,
    
    /// Mount point path (where the pool would be mounted in FUSE scenario)
    pub mountpoint: Option<String>,
    
    /// Branches in this pool
    pub branches: Vec<BranchConfig>,
    
    /// Default policy for create operations
    #[serde(default = "default_create_policy")]
    pub create_policy: String,
    
    /// Default policy for search operations
    #[serde(default = "default_search_policy")]
    pub search_policy: String,
    
    /// Default policy for action operations
    #[serde(default = "default_action_policy")]
    pub action_policy: String,
    
    /// Minimum free space threshold
    #[serde(default = "default_minfreespace")]
    pub minfreespace: String,
}

fn default_create_policy() -> String {
    "pfrd".to_string()
}

fn default_search_policy() -> String {
    "ff".to_string()
}

fn default_action_policy() -> String {
    "epall".to_string()
}

fn default_minfreespace() -> String {
    "4G".to_string()
}

/// Branch configuration within a pool
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BranchConfig {
    /// Path to the branch
    pub path: String,
    
    /// Branch mode (RW, RO, NC)
    #[serde(default)]
    pub mode: Option<String>,
    
    /// Optional per-branch minimum free space
    pub minfreespace: Option<String>,
}

impl BranchConfig {
    /// Convert to Branch struct
    pub fn to_branch(&self) -> Result<Branch> {
        let mut branch_str = self.path.clone();
        
        if let Some(mode) = &self.mode {
            branch_str.push('=');
            branch_str.push_str(mode);
        }
        
        if let Some(minfree) = &self.minfreespace {
            if !branch_str.contains('=') {
                branch_str.push('=');
            } else {
                branch_str.push(',');
            }
            branch_str.push_str(minfree);
        }
        
        Branch::from_str(&branch_str)
    }
}

/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// List of pools
    pub pools: Vec<PoolConfig>,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path)
            .map_err(|e| NofsError::Config(format!(
                "Failed to read config file {}: {}",
                path.display(),
                e
            )))?;
        
        let config: Config = toml::from_str(&content)
            .map_err(|e| NofsError::Config(format!(
                "Failed to parse config file: {}",
                e
            )))?;
        
        Ok(config)
    }

    /// Get the first pool (for single-pool configs)
    pub fn first_pool(&self) -> Result<&PoolConfig> {
        self.pools.first()
            .ok_or_else(|| NofsError::Config("No pools defined in config".to_string()))
    }

    /// Get a pool by name
    pub fn get_pool(&self, name: &str) -> Result<&PoolConfig> {
        self.pools.iter()
            .find(|p| p.name.as_deref() == Some(name))
            .ok_or_else(|| NofsError::Config(format!("Pool '{}' not found", name)))
    }
}

impl PoolConfig {
    /// Convert all branch configs to Branch structs
    pub fn get_branches(&self) -> Result<Vec<Branch>> {
        self.branches.iter()
            .map(|b| b.to_branch())
            .collect()
    }
}

/// Try to find default config locations
pub fn find_default_config() -> Option<PathBuf> {
    // Check common locations
    let locations = [
        // Current directory
        Some(PathBuf::from("nofs.toml")),
        Some(PathBuf::from(".nofs.toml")),
        // Home directory
        dirs_home().map(|mut p| { p.push(".config/nofs/config.toml"); p }),
        // System wide
        Some(PathBuf::from("/etc/nofs/config.toml")),
    ];

    for loc in locations.into_iter().flatten() {
        if loc.exists() {
            return Some(loc);
        }
    }

    None
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
