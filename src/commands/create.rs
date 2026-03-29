//! create command - Get the best branch path for creating a new file

use crate::cache::OperationCache;
use crate::error::Result;
use crate::output::CreateOutput;
use crate::pool::Pool;
use serde_json;
use std::path::Path;

/// Execute the create command
///
/// # Errors
///
/// Returns an error if no suitable branch is found.
pub fn execute(pool: &Pool, path: &str, verbose: bool, json: bool) -> Result<()> {
    // Check if path ends with '/' - this means the path itself is a directory
    let is_directory = path.ends_with('/');

    // Strip trailing slashes from the path
    let clean_path = path.trim_end_matches('/');
    let pool_path = Path::new(clean_path);

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    // Get parent directory for path preservation policies
    let parent = pool_path.parent().unwrap_or_else(|| Path::new(""));

    // Select the best branch (cached)
    let branch = pool.select_create_branch_cached(parent, &cache)?;

    // Create the parent directory on the selected branch if it doesn't exist
    let full_path = branch.path.join(pool_path);
    if let Some(parent_path) = full_path.parent() {
        std::fs::create_dir_all(parent_path)?;
    }

    // If path ends with '/', also create the final directory
    if is_directory {
        std::fs::create_dir_all(&full_path)?;
    }

    // Return the full path on the selected branch
    let full_path_str = full_path.display().to_string();

    if verbose {
        eprintln!("selected:");
        eprintln!("  {} ({} policy)", full_path.display(), pool.create_policy);
    }

    if json {
        let output = CreateOutput {
            path: path.to_string(),
            selected_branch: full_path_str,
            policy: pool.create_policy.to_string(),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("{full_path_str}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::PoolManager;
    use std::fs;
    use std::path::PathBuf;

    fn setup_test_pool(name: &str) -> (PoolManager, PathBuf) {
        let test_dir = std::env::temp_dir().join(format!("nofs_test_create_{name}"));
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let branch_path = test_dir.join("branch1");
        fs::create_dir_all(&branch_path).unwrap();

        // Use single quotes for TOML to avoid escape sequence issues with Windows paths
        let config_content = format!(
            "
[share.test]
paths = ['{}']
",
            branch_path.display()
        );

        let config_path = test_dir.join("config.toml");
        fs::write(&config_path, config_content).unwrap();

        let pool_mgr = PoolManager::from_config(&config_path).unwrap();
        (pool_mgr, test_dir)
    }

    fn cleanup_test_dir(test_dir: &PathBuf) {
        let _ = fs::remove_dir_all(test_dir);
    }

    #[test]
    fn test_create_single_directory_with_trailing_slash() {
        let (pool_mgr, test_dir) = setup_test_pool("single_dir_slash");
        let pool = pool_mgr.get_pool("test").unwrap();

        // create library/ - should create the library directory
        execute(pool, "library/", false, false).unwrap();

        let dir_path = test_dir.join("branch1").join("library");
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_create_no_directory_without_trailing_slash() {
        let (pool_mgr, test_dir) = setup_test_pool("no_dir_no_slash");
        let pool = pool_mgr.get_pool("test").unwrap();

        // create library - should NOT create any folder (it's a filename)
        execute(pool, "library", false, false).unwrap();

        let dir_path = test_dir.join("branch1").join("library");
        assert!(!dir_path.exists());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_create_nested_directories_with_trailing_slash() {
        let (pool_mgr, test_dir) = setup_test_pool("nested_dir_slash");
        let pool = pool_mgr.get_pool("test").unwrap();

        // create library/other/ - should create library/other (like mkdir -p)
        execute(pool, "library/other/", false, false).unwrap();

        let dir_path = test_dir.join("branch1").join("library/other");
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());

        let parent_path = test_dir.join("branch1").join("library");
        assert!(parent_path.exists());
        assert!(parent_path.is_dir());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_create_nested_path_without_trailing_slash() {
        let (pool_mgr, test_dir) = setup_test_pool("nested_no_slash");
        let pool = pool_mgr.get_pool("test").unwrap();

        // create library/other - should create only library (parent dir for a file)
        execute(pool, "library/other", false, false).unwrap();

        let parent_path = test_dir.join("branch1").join("library");
        assert!(parent_path.exists());
        assert!(parent_path.is_dir());

        let file_path = test_dir.join("branch1").join("library/other");
        assert!(!file_path.exists());

        cleanup_test_dir(&test_dir);
    }
}
