//! Configuration parsing for nofs
//!
//! Supports TOML configuration files with named union contexts.

use crate::branch::Branch;
use crate::error::{NofsError, Result};
use crate::policy::Policy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Union context configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UnionConfig {
    /// Branch paths with RW mode (default)
    #[serde(default)]
    pub paths: Vec<String>,

    /// Branch paths with RO (read-only) mode
    #[serde(default)]
    pub ro_paths: Vec<String>,

    /// Branch paths with NC (no-create) mode
    #[serde(default)]
    pub nc_paths: Vec<String>,

    /// Policy for create operations
    #[serde(default = "default_create_policy")]
    pub create_policy: String,

    /// Policy for search operations
    #[serde(default = "default_search_policy")]
    pub search_policy: String,

    /// Policy for action operations
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

/// Main configuration structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Union contexts
    #[serde(default)]
    pub union: HashMap<String, UnionConfig>,
}

impl Config {
    /// Load configuration from a TOML file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = fs::read_to_string(path).map_err(|e| {
            NofsError::Config(format!(
                "Failed to read config file {}: {}",
                path.display(),
                e
            ))
        })?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| NofsError::Config(format!("Failed to parse config file: {}", e)))?;

        Ok(config)
    }

    /// Get a union context by name
    pub fn get_union(&self, name: &str) -> Result<&UnionConfig> {
        self.union
            .get(name)
            .ok_or_else(|| NofsError::Config(format!("Union context '{}' not found", name)))
    }

    /// Get the first union context (for single-context configs)
    pub fn first_union(&self) -> Result<(&str, &UnionConfig)> {
        self.union
            .iter()
            .next()
            .map(|(k, v)| (k.as_str(), v))
            .ok_or_else(|| NofsError::Config("No union contexts defined in config".to_string()))
    }
}

impl UnionConfig {
    /// Convert to Branch structs
    pub fn get_branches(&self) -> Result<Vec<Branch>> {
        let mut branches = Vec::new();

        // Add RW paths (default)
        for path_str in &self.paths {
            branches.push(Branch::from_str(path_str)?);
        }

        // Add RO paths
        for path_str in &self.ro_paths {
            let branch_str = format!("{}=RO", path_str);
            branches.push(Branch::from_str(&branch_str)?);
        }

        // Add NC paths
        for path_str in &self.nc_paths {
            let branch_str = format!("{}=NC", path_str);
            branches.push(Branch::from_str(&branch_str)?);
        }

        Ok(branches)
    }

    /// Get create policy
    pub fn create_policy(&self) -> Result<Policy> {
        Policy::from_str(&self.create_policy)
    }

    /// Get search policy
    pub fn search_policy(&self) -> Result<Policy> {
        Policy::from_str(&self.search_policy)
    }

    /// Get action policy
    pub fn action_policy(&self) -> Result<Policy> {
        Policy::from_str(&self.action_policy)
    }

    /// Get minfreespace in bytes
    pub fn minfreespace_bytes(&self) -> Result<u64> {
        crate::policy::parse_size(&self.minfreespace)
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
        dirs_home().map(|mut p| {
            p.push(".config/nofs/config.toml");
            p
        }),
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
