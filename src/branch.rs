//! Branch management for nofs
//!
//! A branch is a path that contributes to the union pool.

use crate::error::{NofsError, Result};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::path::PathBuf;

/// Branch mode determines how a branch can be used
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "UPPERCASE")]
#[derive(Default)]
pub enum BranchMode {
    /// Read/write - full participation in all operations
    #[default]
    RW,
    /// Read-only - excluded from create/action operations
    RO,
    /// No-create - can read and modify, but not create new files
    NC,
}

impl std::fmt::Display for BranchMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BranchMode::RW => write!(f, "RW"),
            BranchMode::RO => write!(f, "RO"),
            BranchMode::NC => write!(f, "NC"),
        }
    }
}

/// Represents a single branch in the pool
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Branch {
    /// Path to the branch
    pub path: PathBuf,

    /// Branch mode (RW, RO, NC)
    #[serde(default)]
    pub mode: BranchMode,

    /// Optional per-branch minimum free space override
    #[serde(default)]
    pub minfreespace: Option<String>,
}

impl Branch {
    /// Create a new branch from a path string
    /// Format: "/path" or "/path=MODE" or "/path=MODE,minfreespace"
    pub fn from_str(s: &str) -> Result<Self> {
        let (path_str, options) = if let Some(idx) = s.find('=') {
            (&s[..idx], Some(&s[idx + 1..]))
        } else {
            (s, None)
        };

        let path = PathBuf::from(path_str);

        // Validate path exists
        if !path.exists() {
            return Err(NofsError::Branch(format!(
                "Branch path does not exist: {}",
                path.display()
            )));
        }

        let mut mode = BranchMode::RW;
        let mut minfreespace = None;

        if let Some(opts) = options {
            for opt in opts.split(',') {
                let opt = opt.trim();
                match opt.to_uppercase().as_str() {
                    "RW" => mode = BranchMode::RW,
                    "RO" => mode = BranchMode::RO,
                    "NC" => mode = BranchMode::NC,
                    _ if opt.chars().any(|c| c.is_numeric()) => {
                        // Treat as minfreespace value
                        minfreespace = Some(opt.to_string());
                    }
                    _ => {
                        return Err(NofsError::Parse(format!("Unknown branch option: {}", opt)));
                    }
                }
            }
        }

        Ok(Branch {
            path,
            mode,
            minfreespace,
        })
    }

    /// Check if branch is eligible for create operations
    pub fn can_create(&self) -> bool {
        matches!(self.mode, BranchMode::RW)
    }

    /// Check if branch is eligible for action operations (chmod, chown, etc.)
    pub fn can_action(&self) -> bool {
        matches!(self.mode, BranchMode::RW)
    }

    /// Get available space on this branch in bytes
    pub fn available_space(&self) -> Result<u64> {
        let path_c = CString::new(self.path.to_string_lossy().as_bytes())
            .map_err(|e| NofsError::Branch(format!("Invalid path: {}", e)))?;

        let mut stat = unsafe { std::mem::zeroed() };

        let result = unsafe { libc::statvfs(path_c.as_ptr(), &mut stat) };

        if result == 0 {
            // f_bavail is free blocks for unprivileged users
            Ok(stat.f_bavail * stat.f_frsize)
        } else {
            Err(NofsError::Branch("Failed to statvfs".to_string()))
        }
    }

    /// Get total space on this branch in bytes
    pub fn total_space(&self) -> Result<u64> {
        let path_c = CString::new(self.path.to_string_lossy().as_bytes())
            .map_err(|e| NofsError::Branch(format!("Invalid path: {}", e)))?;

        let mut stat = unsafe { std::mem::zeroed() };

        let result = unsafe { libc::statvfs(path_c.as_ptr(), &mut stat) };

        if result == 0 {
            Ok(stat.f_blocks * stat.f_frsize)
        } else {
            Err(NofsError::Branch("Failed to statvfs".to_string()))
        }
    }

    /// Get used space on this branch in bytes
    pub fn used_space(&self) -> Result<u64> {
        let total = self.total_space()?;
        let available = self.available_space()?;
        Ok(total.saturating_sub(available))
    }

    /// Get free space percentage (0-100)
    pub fn free_percentage(&self) -> Result<f64> {
        let total = self.total_space()?;
        if total == 0 {
            return Ok(0.0);
        }
        let available = self.available_space()?;
        Ok((available as f64 / total as f64) * 100.0)
    }

    /// Get used space percentage (0-100)
    pub fn used_percentage(&self) -> Result<f64> {
        Ok(100.0 - self.free_percentage()?)
    }
}
