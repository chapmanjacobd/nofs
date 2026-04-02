//! Shared utility functions for nofs

use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::{NofsError, Result};

/// Result of parsing a path that may contain a share/context prefix
///
/// This struct captures the raw parsed components. The caller should use
/// `matches_share()` to check if the potential share name matches an actual share.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedPath<'a> {
    /// The part before the first colon (potential share name or Windows drive letter)
    pub prefix: &'a str,
    /// The part after the first colon
    pub path_after_colon: &'a str,
    /// True if there was no colon (definitely a native path)
    pub has_no_colon: bool,
    /// True if prefix contains path separators (UNC path like \\server\share)
    pub is_unc: bool,
}

impl ParsedPath<'_> {
    /// Check if the prefix matches a share name
    #[must_use]
    pub fn matches_share(&self, share_name: &str) -> bool {
        self.prefix == share_name
    }

    /// Check if this looks like a Windows drive letter (e.g., "C:" in "C:\path")
    ///
    /// Returns true if prefix is a single ASCII letter and path starts with separator.
    /// Note: This is ambiguous - a share could be named "d". Use `matches_share()`
    /// to check against actual share names.
    #[must_use]
    pub fn looks_like_windows_drive(&self) -> bool {
        self.prefix.len() == 1
            && self.prefix.chars().next().is_some_and(|c| c.is_ascii_alphabetic())
            && self.path_after_colon.starts_with(['\\', '/'])
    }
}

/// Parse a path that may have a share/context prefix
///
/// Handles the following formats:
/// - `share:path` - share with relative path
/// - `share:/path` or `share:\path` - share with absolute path
/// - `C:\path` or `D:/path` - Windows drive letter (check with `looks_like_windows_drive()`)
/// - `\\server\share\path` - UNC path
/// - `/unix/path` - Unix absolute path
/// - `relative/path` - Relative path
///
/// The caller should use `matches_share()` to check if the prefix matches an actual share name.
/// This correctly handles the ambiguous case of single-letter prefixes like `d:/path` which
/// could be either a Windows drive or a share named "d".
///
/// # Examples
///
/// ```
/// use nofs::utils::parse_path_with_context;
///
/// // Share path - check with matches_share()
/// let parsed = parse_path_with_context("media:/photos/vacation.jpg");
/// assert!(!parsed.has_no_colon);
/// assert!(parsed.matches_share("media"));
///
/// // Windows drive letter - looks_like_windows_drive() returns true
/// let parsed = parse_path_with_context("C:\\Users\\file.txt");
/// assert!(parsed.looks_like_windows_drive());
/// assert!(parsed.matches_share("C")); // Prefix is "C"
/// assert!(!parsed.matches_share("media")); // Not "media"
///
/// // Ambiguous case - single letter share named "d"
/// let parsed = parse_path_with_context("d:/pool/path");
/// assert!(parsed.looks_like_windows_drive()); // looks like Windows drive
/// assert!(parsed.matches_share("d")); // matches share "d"
/// ```
#[must_use]
pub fn parse_path_with_context(input: &str) -> ParsedPath<'_> {
    // Find the first colon
    let Some(colon_idx) = input.find(':') else {
        // No colon, return as native path
        return ParsedPath {
            prefix: input,
            path_after_colon: "",
            has_no_colon: true,
            is_unc: false,
        };
    };

    let prefix = &input[..colon_idx];
    let path_after_colon = &input[colon_idx + 1..];

    // Check if prefix contains path separators (UNC path like \\server\share)
    let is_unc = prefix.contains('/') || prefix.contains('\\');

    ParsedPath {
        prefix,
        path_after_colon,
        has_no_colon: false,
        is_unc,
    }
}

/// SI unit constants for size
pub const KB: u64 = 1000;
pub const MB: u64 = KB * 1000;
pub const GB: u64 = MB * 1000;
pub const TB: u64 = GB * 1000;
pub const PB: u64 = TB * 1000;

/// Format size in human-readable format (SI units)
#[must_use]
pub fn format_size(size: u64) -> String {
    #[allow(clippy::float_arithmetic, clippy::cast_precision_loss, clippy::as_conversions)]
    if size >= PB {
        format!("{:.1}PB", size as f64 / PB as f64)
    } else if size >= TB {
        format!("{:.1}TB", size as f64 / TB as f64)
    } else if size >= GB {
        format!("{:.1}GB", size as f64 / GB as f64)
    } else if size >= MB {
        format!("{:.1}MB", size as f64 / MB as f64)
    } else if size >= KB {
        format!("{:.1}KB", size as f64 / KB as f64)
    } else {
        format!("{size}B")
    }
}

/// Configuration for file hashing
#[non_exhaustive]
pub struct HashConfig {
    /// Files at or below this size are hashed entirely
    pub small_file_threshold: u64,
    /// Size of each chunk to sample in large files
    pub sample_chunk_size: u64,
    /// Number of samples to take from large files (0 = use 3-point sampling: beginning, middle, end)
    pub num_samples: u64,
}

impl Default for HashConfig {
    fn default() -> Self {
        // Default: conflict detection settings (3-point sampling)
        HashConfig {
            small_file_threshold: MB,
            sample_chunk_size: 8 * KB,
            num_samples: 0, // Use 3-point sampling
        }
    }
}

impl HashConfig {
    /// Configuration optimized for conflict detection
    /// Uses 3-point sampling (beginning, middle, end) for large files
    #[must_use]
    pub const fn conflict_detection() -> Self {
        HashConfig {
            small_file_threshold: MB,
            sample_chunk_size: 8 * KB,
            num_samples: 0,
        }
    }

    /// Configuration optimized for copy/move conflict resolution
    /// Uses multi-sample hashing for higher accuracy
    #[must_use]
    pub const fn copy_resolution() -> Self {
        HashConfig {
            small_file_threshold: 640 * KB,
            sample_chunk_size: 64 * KB,
            num_samples: 10,
        }
    }
}

/// Compute a hash of a file's content with configurable sampling strategy
///
/// For small files (below threshold), the entire content is hashed.
/// For larger files, sampling is used:
/// - If `num_samples` is 0: 3-point sampling (beginning, middle, end)
/// - If `num_samples` > 0: evenly distributed samples across the file
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn compute_file_hash_with_config(path: &Path, config: &HashConfig) -> Result<String> {
    let mut file =
        File::open(path).map_err(|e| NofsError::Conflict(format!("Failed to open file {}: {}", path.display(), e)))?;

    let metadata = file
        .metadata()
        .map_err(|e| NofsError::Conflict(format!("Failed to get metadata for {}: {}", path.display(), e)))?;

    let file_size = metadata.len();

    // For small files, hash the entire content
    if file_size <= config.small_file_threshold {
        let content = fs::read(path)
            .map_err(|e| NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e)))?;
        let mut hasher = DefaultHasher::new();
        hasher.write(&content);
        return Ok(format!("{:x}", hasher.finish()));
    }

    // For larger files, use sampling
    let mut hasher = DefaultHasher::new();
    let chunk_size = usize::try_from(config.sample_chunk_size).unwrap_or(usize::MAX);
    let mut buf = vec![0_u8; chunk_size];

    if config.num_samples == 0 {
        // 3-point sampling: beginning, middle, end
        // Sample beginning
        let bytes_read = file
            .read(&mut buf)
            .map_err(|e| NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e)))?;
        if let Some(buf_slice) = buf.get(..bytes_read) {
            buf_slice.hash(&mut hasher);
        }

        // Sample middle
        let middle_pos = file_size.checked_div(2).unwrap_or(0);
        file.seek(SeekFrom::Start(middle_pos))
            .map_err(|e| NofsError::Conflict(format!("Failed to seek in file {}: {}", path.display(), e)))?;
        let bytes_read_middle = file
            .read(&mut buf)
            .map_err(|e| NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e)))?;
        if let Some(buf_slice) = buf.get(..bytes_read_middle) {
            buf_slice.hash(&mut hasher);
        }

        // Sample end
        let end_pos = file_size.saturating_sub(config.sample_chunk_size);
        file.seek(SeekFrom::Start(end_pos))
            .map_err(|e| NofsError::Conflict(format!("Failed to seek in file {}: {}", path.display(), e)))?;
        let bytes_read_end = file
            .read(&mut buf)
            .map_err(|e| NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e)))?;
        if let Some(buf_slice) = buf.get(..bytes_read_end) {
            buf_slice.hash(&mut hasher);
        }
    } else {
        // Multi-sample: evenly distributed samples
        for i in 0..config.num_samples {
            let pos = file_size.saturating_mul(i).checked_div(config.num_samples).unwrap_or(0);
            file.seek(SeekFrom::Start(pos))
                .map_err(|e| NofsError::Conflict(format!("Failed to seek in file {}: {}", path.display(), e)))?;
            let bytes_read = file
                .read(&mut buf)
                .map_err(|e| NofsError::Conflict(format!("Failed to read file {}: {}", path.display(), e)))?;
            if let Some(buf_slice) = buf.get(..bytes_read) {
                buf_slice.hash(&mut hasher);
            }
        }
    }

    Ok(format!("{:x}", hasher.finish()))
}

/// Compute a hash of a file's content using conflict detection settings
///
/// Uses 3-point sampling (beginning, middle, end) for large files.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn compute_file_hash(path: &Path) -> Result<String> {
    compute_file_hash_with_config(path, &HashConfig::conflict_detection())
}

/// Compute a hash of a file's content using copy/move resolution settings
///
/// Uses multi-sample hashing (10 samples) for higher accuracy.
///
/// # Errors
///
/// Returns an error if the file cannot be read.
pub fn sample_hash(path: &Path) -> Result<String> {
    compute_file_hash_with_config(path, &HashConfig::copy_resolution())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_path_with_context_share_path() {
        let parsed = parse_path_with_context("media:/photos/vacation.jpg");
        assert!(!parsed.has_no_colon);
        assert!(!parsed.is_unc);
        assert!(parsed.matches_share("media"));
        assert_eq!(parsed.path_after_colon, "/photos/vacation.jpg");
    }

    #[test]
    fn test_parse_path_with_context_share_relative() {
        let parsed = parse_path_with_context("docs:readme.txt");
        assert!(!parsed.has_no_colon);
        assert!(!parsed.is_unc);
        assert!(parsed.matches_share("docs"));
        assert_eq!(parsed.path_after_colon, "readme.txt");
    }

    #[test]
    fn test_parse_path_with_context_windows_drive_backslash() {
        let parsed = parse_path_with_context("C:\\Users\\file.txt");
        assert!(!parsed.has_no_colon);
        assert!(!parsed.is_unc);
        assert!(parsed.looks_like_windows_drive());
        assert_eq!(parsed.prefix, "C");
        assert_eq!(parsed.path_after_colon, "\\Users\\file.txt");
    }

    #[test]
    fn test_parse_path_with_context_windows_drive_forward_slash() {
        let parsed = parse_path_with_context("D:/data/file.txt");
        assert!(!parsed.has_no_colon);
        assert!(!parsed.is_unc);
        assert!(parsed.looks_like_windows_drive());
        assert_eq!(parsed.prefix, "D");
        assert_eq!(parsed.path_after_colon, "/data/file.txt");
    }

    #[test]
    fn test_parse_path_with_context_unc_path() {
        let parsed = parse_path_with_context("\\\\server\\share\\file.txt");
        // UNC path has no colon, so has_no_colon is true
        // is_unc is only set when there IS a colon but prefix contains separators
        assert!(parsed.has_no_colon);
        assert!(!parsed.is_unc);
        assert!(!parsed.looks_like_windows_drive());
    }

    #[test]
    fn test_parse_path_with_context_unix_absolute() {
        let parsed = parse_path_with_context("/home/user/file.txt");
        assert!(parsed.has_no_colon);
        assert!(!parsed.is_unc);
        assert!(!parsed.looks_like_windows_drive());
    }

    #[test]
    fn test_parse_path_with_context_relative_path() {
        let parsed = parse_path_with_context("some/relative/path.txt");
        assert!(parsed.has_no_colon);
        assert!(!parsed.is_unc);
        assert!(!parsed.looks_like_windows_drive());
    }

    #[test]
    fn test_parse_path_with_context_no_colon() {
        let parsed = parse_path_with_context("just_a_file.txt");
        assert!(parsed.has_no_colon);
        assert!(!parsed.is_unc);
        assert!(!parsed.looks_like_windows_drive());
    }

    #[test]
    fn test_parse_path_with_context_single_letter_share() {
        // Single letter share name without separator - treated as share
        let parsed = parse_path_with_context("x:some_file.txt");
        assert!(!parsed.has_no_colon);
        assert!(!parsed.looks_like_windows_drive());
        assert!(parsed.matches_share("x"));
    }

    #[test]
    fn test_parse_path_with_context_single_letter_ambiguous() {
        // Single letter share name with separator - AMBIGUOUS
        // Could be Windows drive OR share named "d"
        // The caller should use matches_share() to resolve
        let parsed = parse_path_with_context("d:/some/path");
        assert!(!parsed.has_no_colon);
        assert!(parsed.looks_like_windows_drive()); // looks like Windows drive
        assert!(parsed.matches_share("d")); // but also matches share "d"
        assert_eq!(parsed.prefix, "d");
        assert_eq!(parsed.path_after_colon, "/some/path");
    }

    #[test]
    fn test_parse_path_with_context_single_letter_not_share() {
        // Single letter that doesn't match any share - Windows drive
        // Note: matches_share just checks if prefix equals the given string,
        // it doesn't know about "real" shares. The caller must check against actual share names.
        let parsed = parse_path_with_context("C:\\path");
        assert!(parsed.looks_like_windows_drive());
        assert!(parsed.matches_share("C")); // prefix is "C"
        assert!(!parsed.matches_share("D")); // prefix is not "D"
    }
}
