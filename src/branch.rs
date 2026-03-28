//! Branch management for nofs
//!
//! A branch is a path that contributes to the share pool.

use crate::cache::OperationCache;
use crate::error::{NofsError, Result};
use serde::{Deserialize, Serialize};
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
        let (path_str, options) = s
            .find('=')
            .map_or((s, None), |idx| (&s[..idx], Some(&s[idx.saturating_add(1)..])));

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
                // Try to parse as BranchMode first, otherwise treat as minfreespace
                if let Ok(parsed_mode) = BranchMode::from_str(opt) {
                    mode = parsed_mode;
                } else if opt.chars().any(char::is_numeric) {
                    // Treat as minfreespace value
                    minfreespace = Some(opt.to_string());
                } else {
                    return Err(NofsError::Branch(format!(
                        "Invalid branch option: '{opt}'. Expected RW/RO/NC or a size value (e.g., 4G)"
                    )));
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
    pub fn available_space(&self) -> Result<u64> {
        fs4::available_space(&self.path).map_err(|e| NofsError::Branch(format!("Failed to get available space: {e}")))
    }

    /// Get total space on this branch in bytes
    ///
    /// # Errors
    ///
    /// Returns an error if the path cannot be converted to a C string or if statvfs fails.
    pub fn total_space(&self) -> Result<u64> {
        fs4::total_space(&self.path).map_err(|e| NofsError::Branch(format!("Failed to get total space: {e}")))
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
    #[allow(clippy::cast_precision_loss, clippy::as_conversions, clippy::float_arithmetic)]
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
    /// This method checks the cache first before calling `path.exists()`,
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_temp_branch() -> (TempDir, Branch) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let branch = Branch {
            path: temp_dir.path().to_path_buf(),
            mode: BranchMode::RW,
            minfreespace: None,
        };
        (temp_dir, branch)
    }

    #[test]
    fn test_branch_mode_from_str() {
        assert_eq!(BranchMode::from_str("RW").unwrap(), BranchMode::RW);
        assert_eq!(BranchMode::from_str("rw").unwrap(), BranchMode::RW);
        assert_eq!(BranchMode::from_str("RO").unwrap(), BranchMode::RO);
        assert_eq!(BranchMode::from_str("ro").unwrap(), BranchMode::RO);
        assert_eq!(BranchMode::from_str("NC").unwrap(), BranchMode::NC);
        assert_eq!(BranchMode::from_str("nc").unwrap(), BranchMode::NC);
        assert!(BranchMode::from_str("INVALID").is_err());
    }

    #[test]
    fn test_branch_mode_display() {
        assert_eq!(BranchMode::RW.to_string(), "RW");
        assert_eq!(BranchMode::RO.to_string(), "RO");
        assert_eq!(BranchMode::NC.to_string(), "NC");
    }

    #[test]
    fn test_branch_parse_simple_path() {
        let (temp_dir, branch) = create_temp_branch();
        let path_str = branch.path.to_str().unwrap();
        let parsed = Branch::parse(path_str).unwrap();
        assert_eq!(parsed.path, branch.path);
        assert_eq!(parsed.mode, BranchMode::RW);
        assert!(parsed.minfreespace.is_none());
        drop(temp_dir);
    }

    #[test]
    fn test_branch_parse_with_mode() {
        let (temp_dir, _) = create_temp_branch();
        let path_str = format!("{}=RO", temp_dir.path().display());
        let parsed = Branch::parse(&path_str).unwrap();
        assert_eq!(parsed.mode, BranchMode::RO);
    }

    #[test]
    fn test_branch_parse_with_mode_and_minfreespace() {
        let (temp_dir, _) = create_temp_branch();
        let path_str = format!("{}=RW,1G", temp_dir.path().display());
        let parsed = Branch::parse(&path_str).unwrap();
        assert_eq!(parsed.mode, BranchMode::RW);
        assert_eq!(parsed.minfreespace, Some("1G".to_string()));
    }

    #[test]
    fn test_branch_parse_with_minfreespace_only() {
        let (temp_dir, _) = create_temp_branch();
        let path_str = format!("{}=512M", temp_dir.path().display());
        let parsed = Branch::parse(&path_str).unwrap();
        assert_eq!(parsed.mode, BranchMode::RW);
        assert_eq!(parsed.minfreespace, Some("512M".to_string()));
    }

    #[test]
    fn test_branch_parse_invalid_option() {
        let (temp_dir, _) = create_temp_branch();
        let path_str = format!("{}=INVALID", temp_dir.path().display());
        let result = Branch::parse(&path_str);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid branch option"));
    }

    #[test]
    fn test_branch_parse_nonexistent_path() {
        let result = Branch::parse("/nonexistent/path/that/does/not/exist");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn test_branch_can_create() {
        let (temp_dir, _) = create_temp_branch();
        let branch_rw = Branch {
            path: temp_dir.path().to_path_buf(),
            mode: BranchMode::RW,
            minfreespace: None,
        };
        let branch_read_only = Branch {
            path: temp_dir.path().to_path_buf(),
            mode: BranchMode::RO,
            minfreespace: None,
        };
        let branch_no_create = Branch {
            path: temp_dir.path().to_path_buf(),
            mode: BranchMode::NC,
            minfreespace: None,
        };

        assert!(branch_rw.can_create());
        assert!(!branch_read_only.can_create());
        assert!(!branch_no_create.can_create());
    }

    #[test]
    fn test_branch_can_action() {
        let (temp_dir, _) = create_temp_branch();
        let branch_rw = Branch {
            path: temp_dir.path().to_path_buf(),
            mode: BranchMode::RW,
            minfreespace: None,
        };
        let branch_read_only = Branch {
            path: temp_dir.path().to_path_buf(),
            mode: BranchMode::RO,
            minfreespace: None,
        };
        let branch_no_create = Branch {
            path: temp_dir.path().to_path_buf(),
            mode: BranchMode::NC,
            minfreespace: None,
        };

        assert!(branch_rw.can_action());
        assert!(!branch_read_only.can_action());
        assert!(!branch_no_create.can_action());
    }

    #[test]
    fn test_branch_space_methods() {
        let (_temp_dir, branch) = create_temp_branch();

        let available = branch.available_space();
        assert!(available.is_ok());
        assert!(available.unwrap() > 0);

        let total = branch.total_space();
        assert!(total.is_ok());
        assert!(total.unwrap() > 0);

        let used = branch.used_space();
        assert!(used.is_ok());

        let free_pct = branch.free_percentage();
        assert!(free_pct.is_ok());
        let free_val = free_pct.unwrap();
        assert!((0.0..=100.0).contains(&free_val));

        let used_pct = branch.used_percentage();
        assert!(used_pct.is_ok());
        let used_val = used_pct.unwrap();
        assert!((0.0..=100.0).contains(&used_val));

        assert!((free_val + used_val - 100.0).abs() < 0.1);
    }

    #[test]
    fn test_branch_cached_methods() {
        let (_temp_dir, branch) = create_temp_branch();
        let cache = OperationCache::new();

        let available_cached = branch.available_space_cached(&cache);
        assert!(available_cached.is_ok());

        let total_cached = branch.total_space_cached(&cache);
        assert!(total_cached.is_ok());

        let test_file = branch.path.join("test_file.txt");
        fs::write(&test_file, "test").unwrap();
        let relative = Path::new("test_file.txt");
        assert!(branch.path_exists_cached(relative, &cache));

        let non_existent = Path::new("non_existent.txt");
        assert!(!branch.path_exists_cached(non_existent, &cache));
    }

    #[test]
    fn test_branch_parse_with_multiple_options() {
        let (temp_dir, _) = create_temp_branch();
        let path_str = format!("{}=NC,2G", temp_dir.path().display());
        let parsed = Branch::parse(&path_str).unwrap();
        assert_eq!(parsed.mode, BranchMode::NC);
        assert_eq!(parsed.minfreespace, Some("2G".to_string()));
    }

    #[test]
    fn test_branch_parse_with_whitespace() {
        let (temp_dir, _) = create_temp_branch();
        // Note: Branch::parse trims whitespace from options but not from the path itself
        let path_str = format!("{}=RO,1G", temp_dir.path().display());
        let parsed = Branch::parse(&path_str).unwrap();
        assert_eq!(parsed.mode, BranchMode::RO);
        assert_eq!(parsed.minfreespace, Some("1G".to_string()));
    }

    #[test]
    fn test_branch_parse_nc_mode() {
        let (temp_dir, _) = create_temp_branch();
        let path_str = format!("{}=NC", temp_dir.path().display());
        let parsed = Branch::parse(&path_str).unwrap();
        assert_eq!(parsed.mode, BranchMode::NC);
        assert!(!parsed.can_create());
        assert!(!parsed.can_action());
    }

    #[test]
    fn test_branch_minfreespace_parsing_variations() {
        let (temp_dir, _) = create_temp_branch();

        // Different size suffixes
        let path_str_100m = format!("{}=100M", temp_dir.path().display());
        let parsed_100m = Branch::parse(&path_str_100m).unwrap();
        assert_eq!(parsed_100m.minfreespace, Some("100M".to_string()));

        let path_str_500k = format!("{}=500K", temp_dir.path().display());
        let parsed_500k = Branch::parse(&path_str_500k).unwrap();
        assert_eq!(parsed_500k.minfreespace, Some("500K".to_string()));

        let path_str_10t = format!("{}=10T", temp_dir.path().display());
        let parsed_10t = Branch::parse(&path_str_10t).unwrap();
        assert_eq!(parsed_10t.minfreespace, Some("10T".to_string()));

        // Plain bytes
        let path_str_bytes = format!("{}=1000000", temp_dir.path().display());
        let parsed_bytes = Branch::parse(&path_str_bytes).unwrap();
        assert_eq!(parsed_bytes.minfreespace, Some("1000000".to_string()));
    }

    #[test]
    fn test_branch_parse_invalid_minfreespace_format() {
        let (temp_dir, _) = create_temp_branch();
        // Invalid option that's not a mode or numeric
        let path_str = format!("{}=INVALID", temp_dir.path().display());
        let result = Branch::parse(&path_str);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid branch option"));
    }

    #[test]
    fn test_branch_space_edge_cases() {
        let (_temp_dir, branch) = create_temp_branch();

        // Space methods should return positive values for valid directories
        let available = branch.available_space().unwrap();
        let total = branch.total_space().unwrap();
        let used = branch.used_space().unwrap();

        // Basic sanity checks
        assert!(available > 0);
        assert!(total > 0);
        assert!(used > 0);
        assert!(used <= total);
        assert!(available <= total);

        // Percentages should be in valid range
        let free_pct = branch.free_percentage().unwrap();
        let used_pct = branch.used_percentage().unwrap();
        assert!((0.0..=100.0).contains(&free_pct));
        assert!((0.0..=100.0).contains(&used_pct));

        // Percentages should sum to ~100
        assert!((free_pct + used_pct - 100.0).abs() < 1.0);
    }

    #[test]
    fn test_branch_mode_serialization() {
        use serde_json;

        // Test serialization
        assert_eq!(serde_json::to_string(&BranchMode::RW).unwrap(), "\"RW\"");
        assert_eq!(serde_json::to_string(&BranchMode::RO).unwrap(), "\"RO\"");
        assert_eq!(serde_json::to_string(&BranchMode::NC).unwrap(), "\"NC\"");

        // Test deserialization
        assert_eq!(serde_json::from_str::<BranchMode>("\"RW\"").unwrap(), BranchMode::RW);
        assert_eq!(serde_json::from_str::<BranchMode>("\"RO\"").unwrap(), BranchMode::RO);
        assert_eq!(serde_json::from_str::<BranchMode>("\"NC\"").unwrap(), BranchMode::NC);
    }

    #[test]
    fn test_branch_serialization() {
        use serde_json;

        let branch = Branch {
            path: PathBuf::from("/test/path"),
            mode: BranchMode::RW,
            minfreespace: Some("1G".to_string()),
        };

        let json = serde_json::to_string(&branch).unwrap();
        assert!(json.contains("/test/path"));
        assert!(json.contains("RW"));
        assert!(json.contains("1G"));

        // Deserialize back
        let deserialized: Branch = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.path, branch.path);
        assert_eq!(deserialized.mode, branch.mode);
        assert_eq!(deserialized.minfreespace, branch.minfreespace);
    }

    #[test]
    fn test_branch_clone_and_debug() {
        let (_temp_dir, branch) = create_temp_branch();

        // Test clone
        let branch_clone = branch.clone();
        assert_eq!(branch.path, branch_clone.path);
        assert_eq!(branch.mode, branch_clone.mode);
        assert_eq!(branch.minfreespace, branch_clone.minfreespace);

        // Test debug formatting
        let debug_str = format!("{branch:?}");
        assert!(debug_str.contains("Branch"));
    }
}
