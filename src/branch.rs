//! Branch management for nofs
//!
//! A branch is a path that contributes to the share pool.

use crate::cache::OperationCache;
use crate::error::{NofsError, Result};
use serde::{Deserialize, Serialize};
use std::ffi::CString;
use std::path::{Path, PathBuf};
use std::str::FromStr;

/// Branch mode determines how a branch can be used
#[non_exhaustive]
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

impl FromStr for BranchMode {
    type Err = NofsError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "RW" => Ok(BranchMode::RW),
            "RO" => Ok(BranchMode::RO),
            "NC" => Ok(BranchMode::NC),
            _ => Err(NofsError::Parse(format!("Unknown branch mode: {s}"))),
        }
    }
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
#[non_exhaustive]
#[derive(Debug, Clone, Deserialize, Serialize)]
#[allow(clippy::unsafe_derive_deserialize)]
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
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist or if the mode cannot be parsed.
    pub fn parse(s: &str) -> Result<Self> {
        let (path_str, options) = s.find('=').map_or((s, None), |idx| {
            (&s[..idx], Some(&s[idx.saturating_add(1)..]))
        });

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
            for opt_str in opts.split(',') {
                let opt = opt_str.trim();
                if opt.chars().any(char::is_numeric) {
                    // Treat as minfreespace value
                    minfreespace = Some(opt.to_string());
                } else {
                    mode = BranchMode::from_str(opt)?;
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
    #[must_use]
    pub const fn can_create(&self) -> bool {
        matches!(self.mode, BranchMode::RW)
    }

    /// Check if branch is eligible for action operations (chmod, chown, etc.)
    #[must_use]
    pub const fn can_action(&self) -> bool {
        matches!(self.mode, BranchMode::RW)
    }

    /// Get available space on this branch in bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be converted to a C string or if statvfs fails.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn available_space(&self) -> Result<u64> {
        let path_c = CString::new(self.path.to_string_lossy().as_bytes())
            .map_err(|e| NofsError::Branch(format!("Invalid path: {e}")))?;

        // Safety: statvfs is called with a valid C string pointer (path_c is guaranteed
        // to be null-terminated by CString) and a valid statvfs pointer (stat is properly
        // zero-initialized). The result is checked for success (0 return value).
        let mut stat = unsafe { std::mem::zeroed() };
        // Safety: `path_c` is a valid null-terminated C string from CString, and `stat`
        // is a properly initialized statvfs struct. libc::statvfs will write to `stat`
        // only on success (return value 0).
        let result = unsafe { libc::statvfs(path_c.as_ptr(), &raw mut stat) };

        if result == 0 {
            // f_bavail is free blocks for unprivileged users
            Ok(stat.f_bavail * stat.f_frsize)
        } else {
            Err(NofsError::Branch("Failed to statvfs".to_string()))
        }
    }

    /// Get total space on this branch in bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be converted to a C string or if statvfs fails.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn total_space(&self) -> Result<u64> {
        let path_c = CString::new(self.path.to_string_lossy().as_bytes())
            .map_err(|e| NofsError::Branch(format!("Invalid path: {e}")))?;

        // Safety: statvfs is called with a valid C string pointer (path_c is guaranteed
        // to be null-terminated by CString) and a valid statvfs pointer (stat is properly
        // zero-initialized). The result is checked for success (0 return value).
        let mut stat = unsafe { std::mem::zeroed() };
        // Safety: `path_c` is a valid null-terminated C string from CString, and `stat`
        // is a properly initialized statvfs struct. libc::statvfs will write to `stat`
        // only on success (return value 0).
        let result = unsafe { libc::statvfs(path_c.as_ptr(), &raw mut stat) };

        if result == 0 {
            Ok(stat.f_blocks * stat.f_frsize)
        } else {
            Err(NofsError::Branch("Failed to statvfs".to_string()))
        }
    }

    /// Get used space on this branch in bytes
    ///
    /// # Errors
    ///
    /// Returns an error if statvfs fails for total or available space.
    pub fn used_space(&self) -> Result<u64> {
        let total = self.total_space()?;
        let available = self.available_space()?;
        Ok(total.saturating_sub(available))
    }

    /// Get free space percentage (0-100)
    ///
    /// # Errors
    ///
    /// Returns an error if statvfs fails.
    #[allow(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        clippy::float_arithmetic
    )]
    pub fn free_percentage(&self) -> Result<f64> {
        let total = self.total_space()?;
        if total == 0 {
            return Ok(0.0);
        }
        let available = self.available_space()?;
        Ok((available as f64 / total as f64) * 100.0)
    }

    /// Get used space percentage (0-100)
    ///
    /// # Errors
    ///
    /// Returns an error if statvfs fails.
    #[allow(clippy::float_arithmetic)]
    pub fn used_percentage(&self) -> Result<f64> {
        Ok(100.0 - self.free_percentage()?)
    }

    /// Get available space, using cache if available
    ///
    /// This method checks the cache first before calling statvfs,
    /// reducing redundant syscalls during batch operations.
    ///
    /// # Errors
    ///
    /// Returns an error if statvfs fails and the value is not cached.
    pub fn available_space_cached(&self, cache: &OperationCache) -> Result<u64> {
        if let Some(cached) = cache.get_space(&self.path) {
            return Ok(cached.available);
        }

        let available = self.available_space()?;

        // Update cache with full space info if we can get total too
        if let Ok(total) = self.total_space() {
            use crate::cache::SpaceInfo;
            cache.set_space(self.path.clone(), SpaceInfo { available, total });
        }

        Ok(available)
    }

    /// Get total space, using cache if available
    ///
    /// This method checks the cache first before calling statvfs,
    /// reducing redundant syscalls during batch operations.
    ///
    /// # Errors
    ///
    /// Returns an error if statvfs fails and the value is not cached.
    pub fn total_space_cached(&self, cache: &OperationCache) -> Result<u64> {
        if let Some(cached) = cache.get_space(&self.path) {
            return Ok(cached.total);
        }

        let total = self.total_space()?;

        // Update cache with full space info if we can get available too
        if let Ok(available) = self.available_space() {
            use crate::cache::SpaceInfo;
            cache.set_space(self.path.clone(), SpaceInfo { available, total });
        }

        Ok(total)
    }

    /// Check path existence with caching
    ///
    /// This method checks the cache first before calling path.exists(),
    /// reducing redundant filesystem calls during batch operations.
    #[must_use]
    pub fn path_exists_cached(&self, relative_path: &Path, cache: &OperationCache) -> bool {
        if let Some(cached) = cache.get_exists(&self.path, relative_path) {
            return cached;
        }

        let exists = self.path.join(relative_path).exists();
        cache.set_exists(self.path.clone(), relative_path, exists);
        exists
    }
}
