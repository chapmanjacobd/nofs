//! Policy engine for nofs
//!
//! Implements branch selection algorithms for different operations.

use crate::branch::Branch;
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
    branches: &'ctx [Branch],
    minfreespace: u64,
}

impl<'ctx> CreatePolicy<'ctx> {
    #[must_use]
    pub fn new(branches: &'ctx [Branch], minfreespace: u64) -> Self {
        CreatePolicy {
            branches,
            minfreespace,
        }
    }

    /// Select a branch for file creation
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable branch is found.
    #[allow(clippy::too_many_lines)]
    pub fn select(&self, policy: Policy, relative_path: Option<&Path>) -> Result<&'ctx Branch> {
        // Filter branches by create eligibility and minfreespace
        let eligible: Vec<&Branch> = self
            .branches
            .iter()
            .filter(|b| {
                if !b.can_create() {
                    return false;
                }

                // Check minfreespace
                let branch_minfree = b.minfreespace.as_ref().map_or(self.minfreespace, |s| {
                    parse_size(s).unwrap_or(self.minfreespace)
                });

                b.available_space()
                    .is_ok_and(|available| available >= branch_minfree)
            })
            .collect();

        if eligible.is_empty() {
            return Err(NofsError::NoSuitableBranch);
        }

        match policy {
            Policy::Pfrd => Self::select_pfrd(&eligible),
            Policy::Mfs => Self::select_mfs(&eligible),
            Policy::Ff | Policy::All => {
                eligible.first().copied().ok_or(NofsError::NoSuitableBranch)
            }
            Policy::Rand => Ok(Self::select_rand(&eligible)),
            Policy::Lfs => Self::select_lfs(&eligible),
            Policy::Lus => Self::select_lus(&eligible),
            Policy::Lup => Self::select_lup(&eligible),
            Policy::EpMfs | Policy::EpFf | Policy::EpRand | Policy::EpAll => {
                // For existing path policies, check if path exists
                if let Some(rel_path) = relative_path {
                    let with_path: Vec<&Branch> = eligible
                        .iter()
                        .copied()
                        .filter(|b| b.path.join(rel_path).exists())
                        .collect();

                    if with_path.is_empty() {
                        // Fall back to non-path-preserving variant
                        return Self::select_fallback(policy, &eligible);
                    }

                    match policy {
                        Policy::EpMfs => Self::select_mfs(&with_path),
                        Policy::EpFf => with_path
                            .first()
                            .copied()
                            .ok_or(NofsError::NoSuitableBranch),
                        Policy::EpRand => Ok(Self::select_rand(&with_path)),
                        Policy::EpAll
                        | Policy::Pfrd
                        | Policy::Mfs
                        | Policy::Ff
                        | Policy::Rand
                        | Policy::Lfs
                        | Policy::Lus
                        | Policy::Lup
                        | _ => eligible.first().copied().ok_or(NofsError::NoSuitableBranch),
                    }
                } else {
                    Self::select_fallback(policy, &eligible)
                }
            }
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    fn select_fallback(policy: Policy, eligible: &[&'ctx Branch]) -> Result<&'ctx Branch> {
        match policy {
            Policy::EpMfs => Self::select_mfs(eligible),
            Policy::EpFf
            | Policy::EpAll
            | Policy::Pfrd
            | Policy::Mfs
            | Policy::Ff
            | Policy::Rand
            | Policy::Lfs
            | Policy::Lus
            | Policy::Lup
            | _ => eligible.first().copied().ok_or(NofsError::NoSuitableBranch),
        }
    }

    #[allow(clippy::arithmetic_side_effects)]
    fn select_pfrd(branches: &[&'ctx Branch]) -> Result<&'ctx Branch> {
        // Calculate total available space
        let total: u64 = branches
            .iter()
            .filter_map(|b| b.available_space().ok())
            .sum();

        if total == 0 {
            return branches.first().copied().ok_or(NofsError::NoSuitableBranch);
        }

        // Select based on weighted random
        let mut rng = rand::rng();
        let pick = rng.random_range(0..total);

        let mut cumulative = 0_u64;
        for branch in branches {
            if let Ok(available) = branch.available_space() {
                cumulative += available;
                if pick < cumulative {
                    return Ok(branch);
                }
            }
        }

        branches.last().copied().ok_or(NofsError::NoSuitableBranch)
    }

    fn select_mfs(branches: &[&'ctx Branch]) -> Result<&'ctx Branch> {
        branches
            .iter()
            .max_by_key(|b| b.available_space().unwrap_or(0))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }

    fn select_lfs(branches: &[&'ctx Branch]) -> Result<&'ctx Branch> {
        branches
            .iter()
            .min_by_key(|b| b.available_space().unwrap_or(u64::MAX))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }

    fn select_lus(branches: &[&'ctx Branch]) -> Result<&'ctx Branch> {
        branches
            .iter()
            .min_by_key(|b| b.used_space().unwrap_or(0))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }

    #[allow(clippy::cast_possible_truncation, clippy::as_conversions)]
    fn select_lup(branches: &[&'ctx Branch]) -> Result<&'ctx Branch> {
        branches
            .iter()
            .min_by_key(|b| b.used_percentage().map(|p| p as i64).unwrap_or(i64::MAX))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }

    #[allow(clippy::indexing_slicing)]
    fn select_rand(branches: &[&'ctx Branch]) -> &'ctx Branch {
        let mut rng = rand::rng();
        let idx = rng.random_range(0..branches.len());
        branches[idx]
    }
}

/// Search policy executor
pub struct SearchPolicy<'ctx> {
    branches: &'ctx [Branch],
}

impl<'ctx> SearchPolicy<'ctx> {
    #[must_use]
    pub fn new(branches: &'ctx [Branch]) -> Self {
        SearchPolicy { branches }
    }

    /// Select a branch for search operations (getattr, open, etc.)
    ///
    /// # Errors
    ///
    /// Returns an error if no suitable branch is found for the operation.
    #[allow(clippy::too_many_lines)]
    pub fn select(&self, policy: Policy, relative_path: &Path) -> Result<&'ctx Branch> {
        match policy {
            Policy::Ff => {
                // First found - return first branch where path exists
                for branch in self.branches {
                    if branch.path.join(relative_path).exists() {
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
                let matching: Vec<&Branch> = self
                    .branches
                    .iter()
                    .filter(|b| b.path.join(relative_path).exists())
                    .collect();

                if matching.is_empty() {
                    return Err(NofsError::PathNotFound(relative_path.display().to_string()));
                }
                Self::select_mfs(&matching)
            }
            Policy::Lfs => {
                let matching: Vec<&Branch> = self
                    .branches
                    .iter()
                    .filter(|b| b.path.join(relative_path).exists())
                    .collect();

                if matching.is_empty() {
                    return Err(NofsError::PathNotFound(relative_path.display().to_string()));
                }
                Self::select_lfs(&matching)
            }
            Policy::Pfrd
            | Policy::Rand
            | Policy::Lus
            | Policy::Lup
            | Policy::EpMfs
            | Policy::EpFf
            | Policy::EpRand => {
                let matching: Vec<&Branch> = self
                    .branches
                    .iter()
                    .filter(|b| b.path.join(relative_path).exists())
                    .collect();

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
            .filter(|b| b.path.join(relative_path).exists())
            .collect()
    }

    fn select_mfs(branches: &[&'ctx Branch]) -> Result<&'ctx Branch> {
        branches
            .iter()
            .max_by_key(|b| b.available_space().unwrap_or(0))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }

    fn select_lfs(branches: &[&'ctx Branch]) -> Result<&'ctx Branch> {
        branches
            .iter()
            .min_by_key(|b| b.available_space().unwrap_or(u64::MAX))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }
}

/// Parse human-readable size string to bytes
///
/// # Errors
///
/// Returns an error if the size string cannot be parsed.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::as_conversions
)]
pub fn parse_size(s: &str) -> Result<u64> {
    let trimmed = s.trim();

    // Try to parse as plain number first
    if let Ok(bytes) = trimmed.parse::<u64>() {
        return Ok(bytes);
    }

    // Parse with suffix
    let num_str: String = trimmed
        .chars()
        .take_while(|c| c.is_numeric() || *c == '.')
        .collect();
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
        "K" | "KB" | "KIB" => 1024,
        "M" | "MB" | "MIB" => 1024 * 1024,
        "G" | "GB" | "GIB" => 1024 * 1024 * 1024,
        "T" | "TB" | "TIB" => 1024 * 1024 * 1024 * 1024,
        _ => return Err(NofsError::Parse(format!("Invalid size suffix: {s}"))),
    };

    #[allow(
        clippy::cast_precision_loss,
        clippy::as_conversions,
        clippy::float_arithmetic
    )]
    Ok((num * multiplier as f64) as u64)
}
