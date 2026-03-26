//! Configuration parsing for nofs
//!
//! Supports TOML configuration files with named share contexts.

use crate::branch::Branch;
use crate::error::{NofsError, Result};
use crate::policy::Policy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Share context configuration
#[non_exhaustive]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ShareConfig {
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
#[non_exhaustive]
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    /// Share contexts
    #[serde(default)]
    pub share: HashMap<String, ShareConfig>,
}

impl Config {
    /// Load configuration from a TOML file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_ref = path.as_ref();
        let content = fs::read_to_string(path_ref).map_err(|e| {
            NofsError::Config(format!(
                "Failed to read config file {}: {}",
                path_ref.display(),
                e
            ))
        })?;

        let config: Config = toml::from_str(&content)
            .map_err(|e| NofsError::Config(format!("Failed to parse config file: {e}")))?;

        Ok(config)
    }

    /// Get a share context by name
    ///
    /// # Errors
    ///
    /// Returns an error if the share context is not found.
    pub fn get_share(&self, name: &str) -> Result<&ShareConfig> {
        self.share
            .get(name)
            .ok_or_else(|| NofsError::Config(format!("Share context '{name}' not found")))
    }

    /// Get the first share context (for single-context configs)
    ///
    /// # Errors
    ///
    /// Returns an error if no share contexts are defined.
    pub fn first_share(&self) -> Result<(&str, &ShareConfig)> {
        self.share
            .iter()
            .next()
            .map(|(k, v)| (k.as_str(), v))
            .ok_or_else(|| NofsError::Config("No share contexts defined in config".to_string()))
    }
}

impl ShareConfig {
    /// Convert to Branch structs
    ///
    /// # Errors
    ///
    /// Returns an error if any branch path cannot be parsed.
    pub fn get_branches(&self) -> Result<Vec<Branch>> {
        let mut branches = Vec::new();

        // Add RW paths (default)
        for path_str in &self.paths {
            branches.push(Branch::parse(path_str)?);
        }

        // Add RO paths
        for path_str in &self.ro_paths {
            let branch_str = format!("{path_str}=RO");
            branches.push(Branch::parse(&branch_str)?);
        }

        // Add NC paths
        for path_str in &self.nc_paths {
            let branch_str = format!("{path_str}=NC");
            branches.push(Branch::parse(&branch_str)?);
        }

        Ok(branches)
    }

    /// Get create policy
    ///
    /// # Errors
    ///
    /// Returns an error if the policy string cannot be parsed.
    pub fn create_policy(&self) -> Result<Policy> {
        Policy::parse(&self.create_policy)
    }

    /// Get search policy
    ///
    /// # Errors
    ///
    /// Returns an error if the policy string cannot be parsed.
    pub fn search_policy(&self) -> Result<Policy> {
        Policy::parse(&self.search_policy)
    }

    /// Get action policy
    ///
    /// # Errors
    ///
    /// Returns an error if the policy string cannot be parsed.
    pub fn action_policy(&self) -> Result<Policy> {
        Policy::parse(&self.action_policy)
    }

    /// Get minfreespace in bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the minfreespace string cannot be parsed.
    pub fn minfreespace_bytes(&self) -> Result<u64> {
        crate::policy::parse_size(&self.minfreespace)
    }
}

/// Try to find default config locations
#[must_use]
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

    locations.into_iter().flatten().find(|loc| loc.exists())
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
