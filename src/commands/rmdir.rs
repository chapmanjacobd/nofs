//! rmdir command - Remove empty directories

use crate::error::{NofsError, Result};
use crate::pool::Pool;
use std::path::Path;

/// Execute the rmdir command
///
/// # Errors
///
/// Returns an error if the directory cannot be removed.
pub fn execute(pool: &Pool, path: &str, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Find all branches containing this directory
    let branches = pool.find_all_branches(pool_path);

    if branches.is_empty() {
        return Err(NofsError::Command(format!(
            "cannot remove '{path}': No such file or directory"
        )));
    }

    for branch in branches {
        let full_path = branch.path.join(pool_path);
        if !full_path.is_dir() {
            return Err(NofsError::Command(format!(
                "cannot remove '{}': Not a directory",
                full_path.display()
            )));
        }

        // Check if directory is empty
        let is_empty = std::fs::read_dir(&full_path)?.next().is_none();
        if !is_empty {
            return Err(NofsError::Command(format!(
                "cannot remove '{}': Directory not empty",
                full_path.display()
            )));
        }

        if verbose {
            eprintln!("removing directory: {}", full_path.display());
        }
        std::fs::remove_dir(&full_path)?;
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
        let test_dir = std::env::temp_dir().join(format!("nofs_test_rmdir_{name}"));
        let _ = fs::remove_dir_all(&test_dir);
        fs::create_dir_all(&test_dir).unwrap();

        let branch_path = test_dir.join("branch1");
        fs::create_dir_all(&branch_path).unwrap();

        let config_content = format!(
            r#"
[share.test]
paths = ["{}"]
"#,
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
    fn test_rmdir_empty() {
        let (pool_mgr, test_dir) = setup_test_pool("empty");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create an empty directory
        let dir_path = test_dir.join("branch1").join("emptydir");
        fs::create_dir_all(&dir_path).unwrap();
        assert!(dir_path.exists());

        // Remove the empty directory
        execute(pool, "emptydir", false).unwrap();
        assert!(!dir_path.exists());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rmdir_nonempty() {
        let (pool_mgr, test_dir) = setup_test_pool("nonempty");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create a directory with a file
        let dir_path = test_dir.join("branch1").join("nonemptydir");
        fs::create_dir_all(&dir_path).unwrap();
        fs::write(dir_path.join("file.txt"), "content").unwrap();
        assert!(dir_path.exists());

        // Try to remove non-empty directory - should fail
        let result = execute(pool, "nonemptydir", false);
        assert!(result.is_err(), "Should fail when directory is not empty");

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rmdir_not_a_directory() {
        let (pool_mgr, test_dir) = setup_test_pool("not_dir");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create a file
        let file_path = test_dir.join("branch1").join("file.txt");
        fs::write(&file_path, "content").unwrap();
        assert!(file_path.exists());

        // Try to remove file with rmdir - should fail
        let result = execute(pool, "file.txt", false);
        assert!(result.is_err(), "Should fail when path is not a directory");

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rmdir_nonexistent() {
        let (pool_mgr, test_dir) = setup_test_pool("nonexistent");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Try to remove nonexistent directory
        let result = execute(pool, "nonexistent", false);
        assert!(result.is_err(), "Should fail when directory doesn't exist");

        cleanup_test_dir(&test_dir);
    }
}
