//! Policy engine for nofs
//!
//! Implements branch selection algorithms for different operations.

use crate::branch::Branch;
use crate::cache::OperationCache;
use crate::error::{NofsError, Result};
use rand::RngExt;
use std::path::Path;
use std::str::FromStr;

/// Available policies for branch selection
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    /// Percentage free random distribution
    Pfrd,
    /// Most free space
    Mfs,
    /// First found (first in list)
    Ff,
    /// Random
    Rand,
    /// Least free space
    Lfs,
    /// Least used space
    Lus,
    /// Least used percentage
    Lup,
    /// Existing path - most free space
    EpMfs,
    /// Existing path - first found
    EpFf,
    /// Existing path - random
    EpRand,
    /// Existing path - all
    EpAll,
    /// All branches
    All,
}

impl FromStr for Policy {
    type Err = NofsError;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "pfrd" => Ok(Policy::Pfrd),
            "mfs" => Ok(Policy::Mfs),
            "ff" => Ok(Policy::Ff),
            "rand" => Ok(Policy::Rand),
            "lfs" => Ok(Policy::Lfs),
            "lus" => Ok(Policy::Lus),
            "lup" => Ok(Policy::Lup),
            "epmfs" => Ok(Policy::EpMfs),
            "epff" => Ok(Policy::EpFf),
            "eprand" => Ok(Policy::EpRand),
            "epall" => Ok(Policy::EpAll),
            "all" => Ok(Policy::All),
            _ => Err(NofsError::Policy(format!("Unknown policy: {s}"))),
        }
    }
}

impl Policy {
    /// Parse policy from string
    ///
    /// # Errors
    ///
    /// Returns an error if the policy string is not recognized.
    pub fn parse(s: &str) -> Result<Self> {
        <Self as FromStr>::from_str(s)
    }
}

impl std::fmt::Display for Policy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Policy::Pfrd => write!(f, "pfrd"),
            Policy::Mfs => write!(f, "mfs"),
            Policy::Ff => write!(f, "ff"),
            Policy::Rand => write!(f, "rand"),
            Policy::Lfs => write!(f, "lfs"),
            Policy::Lus => write!(f, "lus"),
            Policy::Lup => write!(f, "lup"),
            Policy::EpMfs => write!(f, "epmfs"),
            Policy::EpFf => write!(f, "epff"),
            Policy::EpRand => write!(f, "eprand"),
            Policy::EpAll => write!(f, "epall"),
            Policy::All => write!(f, "all"),
        }
    }
}

/// Policy executor for create operations
pub struct CreatePolicy<'ctx> {
    /// Branches to select from
    branches: &'ctx [Branch],
    /// Minimum free space threshold
    minfreespace: u64,
    /// Optional cache for branch metadata
    cache: Option<&'ctx OperationCache>,
}

impl<'ctx> CreatePolicy<'ctx> {
    #[must_use]
    pub const fn new(branches: &'ctx [Branch], minfreespace: u64) -> Self {
        CreatePolicy {
            branches,
            minfreespace,
            cache: None,
        }
    }

    /// Create a new `CreatePolicy` with cache support
    #[must_use]
    pub const fn with_cache(branches: &'ctx [Branch], minfreespace: u64, cache: &'ctx OperationCache) -> Self {
        CreatePolicy {
            branches,
            minfreespace,
            cache: Some(cache),
        }
    }

    /// Select a branch for file creation
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable branch is found.
    #[allow(clippy::too_many_lines)]
    pub fn select(&self, policy: Policy, relative_path: Option<&Path>) -> Result<&'ctx Branch> {
        // Single pass: collect eligible branches with pre-fetched space info
        let eligible_with_space: Vec<(&'ctx Branch, u64)> = self
            .branches
            .iter()
            .filter_map(|b| {
                if !b.can_create() {
                    return None;
                }

                // Check minfreespace using cached or direct statvfs
                let available = if let Some(cache) = self.cache {
                    b.available_space_cached(cache).ok()?
                } else {
                    b.available_space().ok()?
                };

                let branch_minfree = b
                    .minfreespace
                    .as_ref()
                    .map_or(self.minfreespace, |s| parse_size(s).unwrap_or(self.minfreespace));

                (available >= branch_minfree).then_some((b, available))
            })
            .collect();

        if eligible_with_space.is_empty() {
            return Err(NofsError::NoSuitableBranch);
        }

        match policy {
            Policy::Pfrd => Self::select_pfrd_with_space(&eligible_with_space),
            Policy::Mfs => eligible_with_space
                .into_iter()
                .max_by_key(|(_, space)| *space)
                .map(|(b, _)| b)
                .ok_or(NofsError::NoSuitableBranch),
            Policy::Ff | Policy::All => eligible_with_space
                .first()
                .map(|(b, _)| *b)
                .ok_or(NofsError::NoSuitableBranch),
            Policy::Rand => Ok(Self::select_rand_with_space(&eligible_with_space)),
            Policy::Lfs => eligible_with_space
                .into_iter()
                .min_by_key(|(_, space)| *space)
                .map(|(b, _)| b)
                .ok_or(NofsError::NoSuitableBranch),
            Policy::Lus => {
                // For Lus and Lup, we need to fetch used space separately
                let eligible: Vec<&Branch> = eligible_with_space.into_iter().map(|(b, _)| b).collect();
                Self::select_lus(&eligible)
            }
            Policy::Lup => {
                let eligible: Vec<&Branch> = eligible_with_space.into_iter().map(|(b, _)| b).collect();
                Self::select_lup(&eligible)
            }
            Policy::EpMfs | Policy::EpFf | Policy::EpRand | Policy::EpAll => {
                // For existing path policies, check if path exists
                if let Some(rel_path) = relative_path {
                    let with_path: Vec<(&'ctx Branch, u64)> = eligible_with_space
                        .iter()
                        .copied()
                        .filter(|(b, _)| {
                            self.cache.map_or_else(
                                || b.path.join(rel_path).exists(),
                                |cache| b.path_exists_cached(rel_path, cache),
                            )
                        })
                        .collect();

                    if with_path.is_empty() {
                        // Fall back to non-path-preserving variant
                        return Self::select_fallback(policy, &eligible_with_space);
                    }

                    match policy {
                        Policy::EpMfs => with_path
                            .into_iter()
                            .max_by_key(|(_, space)| *space)
                            .map(|(b, _)| b)
                            .ok_or(NofsError::NoSuitableBranch),
                        Policy::EpFf => with_path.first().map(|(b, _)| *b).ok_or(NofsError::NoSuitableBranch),
                        Policy::EpRand => Ok(Self::select_rand_with_space(&with_path)),
                        Policy::Pfrd
                        | Policy::Mfs
                        | Policy::Ff
                        | Policy::Rand
                        | Policy::Lfs
                        | Policy::Lus
                        | Policy::Lup
                        | Policy::EpAll
                        | Policy::All => eligible_with_space
                            .first()
                            .map(|(b, _)| *b)
                            .ok_or(NofsError::NoSuitableBranch),
                    }
                } else {
                    Self::select_fallback(policy, &eligible_with_space)
                }
            }
        }
    }

    /// Fallback policy selection when original policy cannot be applied
    #[allow(clippy::unnecessary_wraps)]
    fn select_fallback(policy: Policy, eligible: &[(&'ctx Branch, u64)]) -> Result<&'ctx Branch> {
        match policy {
            Policy::EpMfs => eligible
                .iter()
                .max_by_key(|(_, space)| *space)
                .map(|(b, _)| *b)
                .ok_or(NofsError::NoSuitableBranch),
            Policy::EpFf
            | Policy::EpAll
            | Policy::Pfrd
            | Policy::Mfs
            | Policy::Ff
            | Policy::Rand
            | Policy::Lfs
            | Policy::Lus
            | Policy::Lup
            | Policy::EpRand
            | Policy::All => eligible.first().map(|(b, _)| *b).ok_or(NofsError::NoSuitableBranch),
        }
    }

    /// Select branch based on percentage free random distribution
    /// Uses pre-fetched space values to avoid redundant statvfs calls
    #[allow(clippy::arithmetic_side_effects)]
    fn select_pfrd_with_space(eligible: &[(&'ctx Branch, u64)]) -> Result<&'ctx Branch> {
        let total: u64 = eligible.iter().map(|(_, s)| s).sum();

        if total == 0 {
            return eligible.first().map(|(b, _)| *b).ok_or(NofsError::NoSuitableBranch);
        }

        // Select based on weighted random
        let mut rng = rand::rng();
        let pick = rng.random_range(0..total);

        let mut cumulative = 0_u64;
        for (branch, available) in eligible {
            cumulative += available;
            if pick < cumulative {
                return Ok(branch);
            }
        }

        eligible.last().map(|(b, _)| *b).ok_or(NofsError::NoSuitableBranch)
    }

    /// Select a random branch from eligible branches with pre-fetched space
    #[allow(clippy::indexing_slicing)]
    fn select_rand_with_space(eligible: &[(&'ctx Branch, u64)]) -> &'ctx Branch {
        let mut rng = rand::rng();
        let idx = rng.random_range(0..eligible.len());
        eligible[idx].0
    }

    /// Select branch with least used space
    fn select_lus(branches: &[&'ctx Branch]) -> Result<&'ctx Branch> {
        branches
            .iter()
            .min_by_key(|b| b.used_space().unwrap_or(0))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }

    /// Select branch with least used percentage
    #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
    fn select_lup(branches: &[&'ctx Branch]) -> Result<&'ctx Branch> {
        branches
            .iter()
            .min_by_key(|b| b.used_percentage().map(|p| p as i64).unwrap_or(i64::MAX))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }
}

/// Search policy executor
pub struct SearchPolicy<'ctx> {
    /// Branches to select from
    branches: &'ctx [Branch],
    /// Optional cache for branch metadata
    cache: Option<&'ctx OperationCache>,
}

impl<'ctx> SearchPolicy<'ctx> {
    #[must_use]
    pub const fn new(branches: &'ctx [Branch]) -> Self {
        SearchPolicy { branches, cache: None }
    }

    /// Create a new `SearchPolicy` with cache support
    #[must_use]
    pub const fn with_cache(branches: &'ctx [Branch], cache: &'ctx OperationCache) -> Self {
        SearchPolicy {
            branches,
            cache: Some(cache),
        }
    }

    /// Select a branch for search operations (getattr, open, etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable branch is found for the operation.
    #[allow(clippy::too_many_lines)]
    pub fn select(&self, policy: Policy, relative_path: &Path) -> Result<&'ctx Branch> {
        // Helper to check existence with or without cache
        let exists = |b: &Branch| -> bool {
            self.cache.map_or_else(
                || b.path.join(relative_path).exists(),
                |cache| b.path_exists_cached(relative_path, cache),
            )
        };

        match policy {
            Policy::Ff => {
                // First found - return first branch where path exists
                for branch in self.branches {
                    if exists(branch) {
                        return Ok(branch);
                    }
                }
                Err(NofsError::PathNotFound(relative_path.display().to_string()))
            }
            Policy::All | Policy::EpAll => {
                // Return first branch (caller should iterate)
                self.branches.first().ok_or(NofsError::NoSuitableBranch)
            }
            Policy::Mfs => {
                // Single pass: collect matching branches with space info
                let matching_with_space: Vec<(&Branch, u64)> = self
                    .branches
                    .iter()
                    .filter_map(|b| {
                        if !exists(b) {
                            return None;
                        }
                        let space = if let Some(cache) = self.cache {
                            b.available_space_cached(cache).ok()?
                        } else {
                            b.available_space().ok()?
                        };
                        Some((b, space))
                    })
                    .collect();

                if matching_with_space.is_empty() {
                    return Err(NofsError::PathNotFound(relative_path.display().to_string()));
                }
                matching_with_space
                    .into_iter()
                    .max_by_key(|(_, space)| *space)
                    .map(|(b, _)| b)
                    .ok_or(NofsError::NoSuitableBranch)
            }
            Policy::Lfs => {
                let matching_with_space: Vec<(&Branch, u64)> = self
                    .branches
                    .iter()
                    .filter_map(|b| {
                        if !exists(b) {
                            return None;
                        }
                        let space = if let Some(cache) = self.cache {
                            b.available_space_cached(cache).ok()?
                        } else {
                            b.available_space().ok()?
                        };
                        Some((b, space))
                    })
                    .collect();

                if matching_with_space.is_empty() {
                    return Err(NofsError::PathNotFound(relative_path.display().to_string()));
                }
                matching_with_space
                    .into_iter()
                    .min_by_key(|(_, space)| *space)
                    .map(|(b, _)| b)
                    .ok_or(NofsError::NoSuitableBranch)
            }
            Policy::Pfrd | Policy::Rand | Policy::Lus | Policy::Lup | Policy::EpMfs | Policy::EpFf | Policy::EpRand => {
                let matching: Vec<&Branch> = self.branches.iter().filter(|b| exists(b)).collect();

                if matching.is_empty() {
                    return Err(NofsError::PathNotFound(relative_path.display().to_string()));
                }
                matching.first().copied().ok_or(NofsError::NoSuitableBranch)
            }
        }
    }

    /// Find all branches containing a path
    #[must_use]
    pub fn find_all(&self, relative_path: &Path) -> Vec<&'ctx Branch> {
        self.branches
            .iter()
            .filter(|b| {
                self.cache.map_or_else(
                    || b.path.join(relative_path).exists(),
                    |cache| b.path_exists_cached(relative_path, cache),
                )
            })
            .collect()
    }
}

use crate::utils::{GB, KB, MB, PB, TB};

/// Parse human-readable size string to bytes
///
/// # Errors
///
/// Returns an error if the size string cannot be parsed.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::as_conversions)]
pub fn parse_size(s: &str) -> Result<u64> {
    let trimmed = s.trim();

    // Try to parse as plain number first
    if let Ok(bytes) = trimmed.parse::<u64>() {
        return Ok(bytes);
    }

    // Parse with suffix
    let num_str: String = trimmed.chars().take_while(|c| c.is_numeric() || *c == '.').collect();
    let suffix = trimmed
        .chars()
        .skip(num_str.len())
        .collect::<String>()
        .trim()
        .to_uppercase();

    let num: f64 = num_str
        .parse()
        .map_err(|_| NofsError::Parse(format!("Invalid size number: {s}")))?;

    let multiplier = match suffix.as_str() {
        "" | "B" => 1_u64,
        "K" | "KB" => KB,
        "M" | "MB" => MB,
        "G" | "GB" => GB,
        "T" | "TB" => TB,
        "P" | "PB" => PB,
        "KIB" => 1024,
        "MIB" => 1024 * 1024,
        "GIB" => 1024 * 1024 * 1024,
        "TIB" => 1024 * 1024 * 1024 * 1024,
        "PIB" => 1024 * 1024 * 1024 * 1024 * 1024,
        _ => return Err(NofsError::Parse(format!("Invalid size suffix: {s}"))),
    };

    #[allow(clippy::cast_precision_loss, clippy::as_conversions, clippy::float_arithmetic)]
    Ok((num * multiplier as f64) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_size() {
        // Plain bytes
        assert_eq!(parse_size("1024").unwrap(), 1024);
        assert_eq!(parse_size("1000").unwrap(), 1000);

        // SI units
        assert_eq!(parse_size("1K").unwrap(), 1000);
        assert_eq!(parse_size("1KB").unwrap(), 1000);
        assert_eq!(parse_size("1M").unwrap(), 1_000_000);
        assert_eq!(parse_size("1MB").unwrap(), 1_000_000);
        assert_eq!(parse_size("1G").unwrap(), 1_000_000_000);
        assert_eq!(parse_size("1GB").unwrap(), 1_000_000_000);
        assert_eq!(parse_size("1T").unwrap(), 1_000_000_000_000);
        assert_eq!(parse_size("1TB").unwrap(), 1_000_000_000_000);
        assert_eq!(parse_size("1P").unwrap(), 1_000_000_000_000_000);
        assert_eq!(parse_size("1PB").unwrap(), 1_000_000_000_000_000);

        // IEC units
        assert_eq!(parse_size("1KIB").unwrap(), 1024);
        assert_eq!(parse_size("1MIB").unwrap(), 1024 * 1024);
        assert_eq!(parse_size("1GIB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("1TIB").unwrap(), 1024 * 1024 * 1024 * 1024);
        assert_eq!(parse_size("1PIB").unwrap(), 1024 * 1024 * 1024 * 1024 * 1024);

        // Floats
        assert_eq!(parse_size("1.5K").unwrap(), 1500);
        assert_eq!(parse_size("1.5M").unwrap(), 1_500_000);

        // Error cases
        assert!(parse_size("1X").is_err());
        assert!(parse_size("abc").is_err());
    }
}
