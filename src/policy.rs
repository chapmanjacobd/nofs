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

    /// Fallback policy selection when original policy cannot be applied.
    ///
    /// This is called when a path-preserving policy (`Ep*`) cannot find any branches
    /// where the path exists. In this case, we fall back to non-path-preserving behavior:
    /// - `EpMfs` falls back to `Mfs` (select branch with most free space)
    /// - All other policies fall back to "first eligible branch" (same as `Ff`)
    #[allow(clippy::unnecessary_wraps)]
    fn select_fallback(policy: Policy, eligible: &[(&'ctx Branch, u64)]) -> Result<&'ctx Branch> {
        match policy {
            // EpMfs → Mfs: select branch with most free space
            Policy::EpMfs => eligible
                .iter()
                .max_by_key(|(_, space)| *space)
                .map(|(b, _)| *b)
                .ok_or(NofsError::NoSuitableBranch),

            // All other policies → Ff: select first eligible branch
            Policy::EpFf
            | Policy::EpAll
            | Policy::EpRand
            | Policy::Pfrd
            | Policy::Mfs
            | Policy::Ff
            | Policy::Rand
            | Policy::Lfs
            | Policy::Lus
            | Policy::Lup
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
            // For search operations, space-based and random policies fall back to "first found"
            // since the file already exists and we just need to locate it.
            // - Pfrd/Lus/Lup: Space metrics don't matter for existing files
            // - Rand: Could randomly select, but first-found is more deterministic for reads
            // - Ep*: Path-preserving policies don't apply to search (path already known)
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
        .map_err(|e| NofsError::Parse(format!("Invalid size number {num_str} in {s}: {e}")))?;

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
    use crate::branch::BranchMode;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_branch(name: &str) -> (TempDir, Branch) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let branch_path = temp_dir.path().join(name);
        fs::create_dir_all(&branch_path).unwrap();
        let branch = Branch {
            path: branch_path,
            mode: BranchMode::RW,
            minfreespace: None,
        };
        (temp_dir, branch)
    }

    #[test]
    fn test_policy_from_str() {
        assert_eq!(Policy::from_str("pfrd").unwrap(), Policy::Pfrd);
        assert_eq!(Policy::from_str("PFRD").unwrap(), Policy::Pfrd);
        assert_eq!(Policy::from_str("mfs").unwrap(), Policy::Mfs);
        assert_eq!(Policy::from_str("ff").unwrap(), Policy::Ff);
        assert_eq!(Policy::from_str("rand").unwrap(), Policy::Rand);
        assert_eq!(Policy::from_str("lfs").unwrap(), Policy::Lfs);
        assert_eq!(Policy::from_str("lus").unwrap(), Policy::Lus);
        assert_eq!(Policy::from_str("lup").unwrap(), Policy::Lup);
        assert_eq!(Policy::from_str("epmfs").unwrap(), Policy::EpMfs);
        assert_eq!(Policy::from_str("epff").unwrap(), Policy::EpFf);
        assert_eq!(Policy::from_str("eprand").unwrap(), Policy::EpRand);
        assert_eq!(Policy::from_str("epall").unwrap(), Policy::EpAll);
        assert_eq!(Policy::from_str("all").unwrap(), Policy::All);
        assert!(Policy::from_str("invalid").is_err());
    }

    #[test]
    fn test_policy_display() {
        assert_eq!(Policy::Pfrd.to_string(), "pfrd");
        assert_eq!(Policy::Mfs.to_string(), "mfs");
        assert_eq!(Policy::Ff.to_string(), "ff");
        assert_eq!(Policy::Rand.to_string(), "rand");
        assert_eq!(Policy::Lfs.to_string(), "lfs");
        assert_eq!(Policy::Lus.to_string(), "lus");
        assert_eq!(Policy::Lup.to_string(), "lup");
        assert_eq!(Policy::EpMfs.to_string(), "epmfs");
        assert_eq!(Policy::EpFf.to_string(), "epff");
        assert_eq!(Policy::EpRand.to_string(), "eprand");
        assert_eq!(Policy::EpAll.to_string(), "epall");
        assert_eq!(Policy::All.to_string(), "all");
    }

    #[test]
    fn test_policy_parse() {
        assert_eq!(Policy::parse("mfs").unwrap(), Policy::Mfs);
        assert!(Policy::parse("invalid").is_err());
    }

    #[test]
    fn test_create_policy_new() {
        let (_temp, branch) = create_test_branch("test");
        let branches = vec![branch];
        let policy = CreatePolicy::new(&branches, 1024);
        assert_eq!(policy.minfreespace, 1024);
        assert!(policy.cache.is_none());
    }

    #[test]
    fn test_create_policy_with_cache() {
        let (_temp, branch) = create_test_branch("test");
        let branches = vec![branch];
        let cache = OperationCache::new();
        let policy = CreatePolicy::with_cache(&branches, 1024, &cache);
        assert!(policy.cache.is_some());
    }

    #[test]
    fn test_create_policy_select_mfs() {
        let (temp1, branch1) = create_test_branch("disk1");
        let (temp2, branch2) = create_test_branch("disk2");

        // Create some files to differentiate the branches
        fs::write(temp1.path().join("file.txt"), "small").unwrap();
        fs::write(temp2.path().join("file.txt"), "larger content file").unwrap();

        let branches = vec![branch1, branch2];
        let policy = CreatePolicy::new(&branches, 0);

        let selected = policy.select(Policy::Mfs, None).unwrap();
        assert!(selected.can_create());
    }

    #[test]
    fn test_create_policy_select_ff() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");

        let branches = vec![branch1, branch2];
        let policy = CreatePolicy::new(&branches, 0);
        
        let selected = policy.select(Policy::Ff, None).unwrap();
        assert_eq!(selected.path, branches.first().unwrap().path);
    }

    #[test]
    fn test_create_policy_select_lfs() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");

        let branches = vec![branch1, branch2];
        let policy = CreatePolicy::new(&branches, 0);

        let selected = policy.select(Policy::Lfs, None).unwrap();
        assert!(selected.can_create());
    }

    #[test]
    fn test_create_policy_select_rand() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");

        let branches = vec![branch1, branch2];
        let policy = CreatePolicy::new(&branches, 0);

        // Run multiple times to ensure randomness doesn't crash
        for _ in 0..5 {
            let selected = policy.select(Policy::Rand, None).unwrap();
            assert!(selected.can_create());
        }
    }

    #[test]
    fn test_create_policy_select_pfrd() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");

        let branches = vec![branch1, branch2];
        let policy = CreatePolicy::new(&branches, 0);

        let selected = policy.select(Policy::Pfrd, None).unwrap();
        assert!(selected.can_create());
    }

    #[test]
    fn test_create_policy_select_lus() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");

        let branches = vec![branch1, branch2];
        let policy = CreatePolicy::new(&branches, 0);

        let selected = policy.select(Policy::Lus, None).unwrap();
        assert!(selected.can_create());
    }

    #[test]
    fn test_create_policy_select_lup() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");

        let branches = vec![branch1, branch2];
        let policy = CreatePolicy::new(&branches, 0);

        let selected = policy.select(Policy::Lup, None).unwrap();
        assert!(selected.can_create());
    }

    #[test]
    fn test_create_policy_no_suitable_branch() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let mut branch_ro = branch1;
        branch_ro.mode = BranchMode::RO;

        let branches = vec![branch_ro];
        let policy = CreatePolicy::new(&branches, 0);
        
        let result = policy.select(Policy::Mfs, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_policy_with_minfreespace() {
        let (_temp, branch) = create_test_branch("disk");
        let branches = vec![branch];
        let policy = CreatePolicy::new(&branches, u64::MAX);

        let result = policy.select(Policy::Mfs, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_create_policy_epmfs_with_existing_path() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");
        
        // Create file only in disk1's branch path
        let file_path = branch1.path.join("existing.txt");
        fs::write(&file_path, "content").unwrap();

        let branches = vec![branch1, branch2];
        let policy = CreatePolicy::new(&branches, 0);
        
        let selected = policy.select(Policy::EpMfs, Some(Path::new("existing.txt"))).unwrap();
        assert!(selected.can_create());
    }

    #[test]
    fn test_create_policy_epmfs_fallback() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");

        let branches = vec![branch1, branch2];
        let policy = CreatePolicy::new(&branches, 0);

        // Path doesn't exist anywhere, should fallback to Mfs
        let selected = policy
            .select(Policy::EpMfs, Some(Path::new("nonexistent.txt")))
            .unwrap();
        assert!(selected.can_create());
    }

    #[test]
    fn test_create_policy_epff_with_existing_path() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");
        
        fs::write(branch1.path.join("existing.txt"), "content").unwrap();

        let branches = vec![branch1, branch2];
        let policy = CreatePolicy::new(&branches, 0);
        
        let selected = policy.select(Policy::EpFf, Some(Path::new("existing.txt"))).unwrap();
        assert_eq!(selected.path, branches.first().unwrap().path);
    }

    #[test]
    fn test_create_policy_eprand_with_existing_path() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");
        
        fs::write(branch1.path.join("existing.txt"), "content").unwrap();

        let branches = vec![branch1, branch2];
        let policy = CreatePolicy::new(&branches, 0);
        
        let selected = policy.select(Policy::EpRand, Some(Path::new("existing.txt"))).unwrap();
        assert_eq!(selected.path, branches.first().unwrap().path);
    }

    #[test]
    fn test_search_policy_new() {
        let (_temp, branch) = create_test_branch("test");
        let branches = vec![branch];
        let policy = SearchPolicy::new(&branches);
        assert!(policy.cache.is_none());
    }

    #[test]
    fn test_search_policy_with_cache() {
        let (_temp, branch) = create_test_branch("test");
        let branches = vec![branch];
        let cache = OperationCache::new();
        let policy = SearchPolicy::with_cache(&branches, &cache);
        assert!(policy.cache.is_some());
    }

    #[test]
    fn test_search_policy_select_ff() {
        let (_temp, branch) = create_test_branch("disk");
        let file_path = branch.path.join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let branches = vec![branch];
        let policy = SearchPolicy::new(&branches);
        
        let selected = policy.select(Policy::Ff, Path::new("file.txt")).unwrap();
        assert_eq!(selected.path, branches.first().unwrap().path);
    }

    #[test]
    fn test_search_policy_select_ff_not_found() {
        let (_temp, branch) = create_test_branch("disk");

        let branches = vec![branch];
        let policy = SearchPolicy::new(&branches);

        let result = policy.select(Policy::Ff, Path::new("nonexistent.txt"));
        assert!(result.is_err());
    }

    #[test]
    fn test_search_policy_select_all() {
        let (_temp, branch) = create_test_branch("disk");

        let branches = vec![branch];
        let policy = SearchPolicy::new(&branches);
        
        let selected = policy.select(Policy::All, Path::new("any.txt")).unwrap();
        assert_eq!(selected.path, branches.first().unwrap().path);
    }

    #[test]
    fn test_search_policy_select_epall() {
        let (_temp, branch) = create_test_branch("disk");

        let branches = vec![branch];
        let policy = SearchPolicy::new(&branches);
        
        let selected = policy.select(Policy::EpAll, Path::new("any.txt")).unwrap();
        assert_eq!(selected.path, branches.first().unwrap().path);
    }

    #[test]
    fn test_search_policy_select_mfs() {
        let (_temp, branch) = create_test_branch("disk");
        let file_path = branch.path.join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let branches = vec![branch];
        let policy = SearchPolicy::new(&branches);
        
        let selected = policy.select(Policy::Mfs, Path::new("file.txt")).unwrap();
        assert_eq!(selected.path, branches.first().unwrap().path);
    }

    #[test]
    fn test_search_policy_select_lfs() {
        let (_temp, branch) = create_test_branch("disk");
        let file_path = branch.path.join("file.txt");
        fs::write(&file_path, "content").unwrap();

        let branches = vec![branch];
        let policy = SearchPolicy::new(&branches);
        
        let selected = policy.select(Policy::Lfs, Path::new("file.txt")).unwrap();
        assert_eq!(selected.path, branches.first().unwrap().path);
    }

    #[test]
    fn test_search_policy_select_fallback() {
        let (_temp, branch) = create_test_branch("disk");

        let branches = vec![branch];
        let policy = SearchPolicy::new(&branches);

        // Lus/Lup/Rand should fall back to first found for search
        let selected = policy.select(Policy::Lus, Path::new("nonexistent.txt"));
        assert!(selected.is_err());
    }

    #[test]
    fn test_search_policy_find_all() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");
        
        // Create file in both branch paths
        fs::write(branch1.path.join("shared.txt"), "content1").unwrap();
        fs::write(branch2.path.join("shared.txt"), "content2").unwrap();

        let branches = vec![branch1, branch2];
        let policy = SearchPolicy::new(&branches);
        
        let found = policy.find_all(Path::new("shared.txt"));
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn test_search_policy_find_all_none() {
        let (_temp1, branch1) = create_test_branch("disk1");
        let (_temp2, branch2) = create_test_branch("disk2");

        let branches = vec![branch1, branch2];
        let policy = SearchPolicy::new(&branches);

        let found = policy.find_all(Path::new("nonexistent.txt"));
        assert!(found.is_empty());
    }

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
