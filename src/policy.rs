//! Policy engine for nofs
//!
//! Implements branch selection algorithms for different operations.

use crate::branch::Branch;
use crate::error::{NofsError, Result};
use rand::Rng;
use std::path::Path;
use std::str::FromStr;

/// Available policies for branch selection
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
            _ => Err(NofsError::Policy(format!("Unknown policy: {}", s))),
        }
    }
}

impl Policy {
    /// Parse policy from string
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
pub struct CreatePolicy<'a> {
    branches: &'a [Branch],
    minfreespace: u64,
}

impl<'a> CreatePolicy<'a> {
    pub fn new(branches: &'a [Branch], minfreespace: u64) -> Self {
        CreatePolicy {
            branches,
            minfreespace,
        }
    }

    /// Select a branch for file creation
    pub fn select(&self, policy: Policy, relative_path: Option<&Path>) -> Result<&'a Branch> {
        // Filter branches by create eligibility and minfreespace
        let eligible: Vec<&Branch> = self
            .branches
            .iter()
            .filter(|b| {
                if !b.can_create() {
                    return false;
                }

                // Check minfreespace
                let branch_minfree = b
                    .minfreespace
                    .as_ref()
                    .map(|s| parse_size(s).unwrap_or(self.minfreespace))
                    .unwrap_or(self.minfreespace);

                match b.available_space() {
                    Ok(available) => available >= branch_minfree,
                    Err(_) => false,
                }
            })
            .collect();

        if eligible.is_empty() {
            return Err(NofsError::NoSuitableBranch);
        }

        match policy {
            Policy::Pfrd => self.select_pfrd(&eligible),
            Policy::Mfs => self.select_mfs(&eligible),
            Policy::Ff => Ok(eligible[0]),
            Policy::Rand => self.select_rand(&eligible),
            Policy::Lfs => self.select_lfs(&eligible),
            Policy::Lus => self.select_lus(&eligible),
            Policy::Lup => self.select_lup(&eligible),
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
                        return self.select_fallback(policy, &eligible);
                    }

                    match policy {
                        Policy::EpMfs => self.select_mfs(&with_path),
                        Policy::EpFf => Ok(with_path[0]),
                        Policy::EpRand => self.select_rand(&with_path),
                        Policy::EpAll => Ok(eligible[0]), // Return first for create
                        _ => Ok(eligible[0]),
                    }
                } else {
                    self.select_fallback(policy, &eligible)
                }
            }
            Policy::All => Ok(eligible[0]),
        }
    }

    fn select_fallback(&self, policy: Policy, eligible: &[&'a Branch]) -> Result<&'a Branch> {
        match policy {
            Policy::EpMfs => self.select_mfs(eligible),
            Policy::EpFf | Policy::EpAll => Ok(eligible[0]),
            Policy::EpRand => self.select_rand(eligible),
            _ => Ok(eligible[0]),
        }
    }

    fn select_pfrd(&self, branches: &[&'a Branch]) -> Result<&'a Branch> {
        // Calculate total available space
        let total: u64 = branches
            .iter()
            .filter_map(|b| b.available_space().ok())
            .sum();

        if total == 0 {
            return Ok(branches[0]);
        }

        // Select based on weighted random
        let mut rng = rand::thread_rng();
        let pick = rng.gen_range(0..total);

        let mut cumulative = 0u64;
        for branch in branches {
            if let Ok(available) = branch.available_space() {
                cumulative += available;
                if pick < cumulative {
                    return Ok(branch);
                }
            }
        }

        Ok(branches[branches.len() - 1])
    }

    fn select_mfs(&self, branches: &[&'a Branch]) -> Result<&'a Branch> {
        branches
            .iter()
            .max_by_key(|b| b.available_space().unwrap_or(0))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }

    fn select_lfs(&self, branches: &[&'a Branch]) -> Result<&'a Branch> {
        branches
            .iter()
            .min_by_key(|b| b.available_space().unwrap_or(u64::MAX))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }

    fn select_lus(&self, branches: &[&'a Branch]) -> Result<&'a Branch> {
        branches
            .iter()
            .min_by_key(|b| b.used_space().unwrap_or(0))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }

    fn select_lup(&self, branches: &[&'a Branch]) -> Result<&'a Branch> {
        branches
            .iter()
            .min_by_key(|b| b.used_percentage().map(|p| p as i64).unwrap_or(i64::MAX))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }

    fn select_rand(&self, branches: &[&'a Branch]) -> Result<&'a Branch> {
        let mut rng = rand::thread_rng();
        let idx = rng.gen_range(0..branches.len());
        Ok(branches[idx])
    }
}

/// Search policy executor
pub struct SearchPolicy<'a> {
    branches: &'a [Branch],
}

impl<'a> SearchPolicy<'a> {
    pub fn new(branches: &'a [Branch]) -> Self {
        SearchPolicy { branches }
    }

    /// Select a branch for search operations (getattr, open, etc.)
    pub fn select(&self, policy: Policy, relative_path: &Path) -> Result<&'a Branch> {
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
            _ => {
                // For other policies, find all matching and apply policy
                let matching: Vec<&Branch> = self
                    .branches
                    .iter()
                    .filter(|b| b.path.join(relative_path).exists())
                    .collect();

                if matching.is_empty() {
                    return Err(NofsError::PathNotFound(relative_path.display().to_string()));
                }

                match policy {
                    Policy::Mfs => self.select_mfs(&matching),
                    Policy::Lfs => self.select_lfs(&matching),
                    _ => Ok(matching[0]),
                }
            }
        }
    }

    /// Find all branches containing a path
    pub fn find_all(&self, relative_path: &Path) -> Vec<&'a Branch> {
        self.branches
            .iter()
            .filter(|b| b.path.join(relative_path).exists())
            .collect()
    }

    fn select_mfs(&self, branches: &[&'a Branch]) -> Result<&'a Branch> {
        branches
            .iter()
            .max_by_key(|b| b.available_space().unwrap_or(0))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }

    fn select_lfs(&self, branches: &[&'a Branch]) -> Result<&'a Branch> {
        branches
            .iter()
            .min_by_key(|b| b.available_space().unwrap_or(u64::MAX))
            .copied()
            .ok_or(NofsError::NoSuitableBranch)
    }
}

/// Parse human-readable size string to bytes
pub fn parse_size(s: &str) -> Result<u64> {
    let s = s.trim();

    // Try to parse as plain number first
    if let Ok(bytes) = s.parse::<u64>() {
        return Ok(bytes);
    }

    // Parse with suffix
    let num_str: String = s
        .chars()
        .take_while(|c| c.is_numeric() || *c == '.')
        .collect();
    let suffix = s
        .chars()
        .skip(num_str.len())
        .collect::<String>()
        .trim()
        .to_uppercase();

    let num: f64 = num_str
        .parse()
        .map_err(|_| NofsError::Parse(format!("Invalid size number: {}", s)))?;

    let multiplier = match suffix.as_str() {
        "" | "B" => 1u64,
        "K" | "KB" | "KIB" => 1024,
        "M" | "MB" | "MIB" => 1024 * 1024,
        "G" | "GB" | "GIB" => 1024 * 1024 * 1024,
        "T" | "TB" | "TIB" => 1024 * 1024 * 1024 * 1024,
        _ => return Err(NofsError::Parse(format!("Invalid size suffix: {}", s))),
    };

    Ok((num * multiplier as f64) as u64)
}
