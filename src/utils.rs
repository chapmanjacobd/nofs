//! Shared utility functions for nofs

use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::{NofsError, Result};

/// SI unit constants for size
pub const KB: u64 = 1000;
pub const MB: u64 = KB * 1000;
pub const GB: u64 = MB * 1000;
pub const TB: u64 = GB * 1000;
pub const PB: u64 = TB * 1000;

/// Format size in human-readable format (SI units)
#[allow(clippy::cast_precision_loss, clippy::as_conversions, clippy::float_arithmetic)]
#[must_use]
pub fn format_size(size: u64) -> String {
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
#[allow(clippy::integer_division, clippy::cast_possible_truncation, clippy::as_conversions)]
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
    let chunk_size = config.sample_chunk_size as usize;
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
        let middle_pos = file_size / 2;
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
        #[allow(clippy::arithmetic_side_effects)]
        for i in 0..config.num_samples {
            let pos = file_size.saturating_mul(i) / config.num_samples;
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
