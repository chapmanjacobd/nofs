//! rm command - Remove files and directories

use crate::error::{NofsError, Result};
use crate::pool::Pool;
use std::path::Path;

/// Execute the rm command
///
/// # Errors
///
/// Returns an error if the file cannot be removed.
pub fn execute(pool: &Pool, path: &str, recursive: bool, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Find all branches containing this file
    let branches = pool.find_all_branches(pool_path);

    if branches.is_empty() {
        return Err(NofsError::Command(format!(
            "cannot remove '{path}': No such file or directory"
        )));
    }

    for branch in branches {
        let full_path = branch.path.join(pool_path);
        if recursive {
            if verbose {
                eprintln!("removing: {}", full_path.display());
            }
            std::fs::remove_dir_all(&full_path)?;
        } else {
            if full_path.is_dir() {
                if verbose {
                    eprintln!("removing directory: {}", full_path.display());
                }
                std::fs::remove_dir(&full_path)?;
            } else {
                if verbose {
                    eprintln!("removing: {}", full_path.display());
                }
                std::fs::remove_file(&full_path)?;
            }
        }
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
        let test_dir = std::env::temp_dir().join(format!("nofs_test_rm_{name}"));
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
    fn test_rm_file() {
        let (pool_mgr, test_dir) = setup_test_pool("file");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create a test file
        let file_path = test_dir.join("branch1").join("test.txt");
        fs::write(&file_path, "content").unwrap();
        assert!(file_path.exists());

        // Remove the file
        execute(pool, "test.txt", false, false).unwrap();
        assert!(!file_path.exists());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rm_directory() {
        let (pool_mgr, test_dir) = setup_test_pool("dir");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create a test directory
        let dir_path = test_dir.join("branch1").join("testdir");
        fs::create_dir_all(&dir_path).unwrap();
        assert!(dir_path.exists());

        // Remove the directory
        execute(pool, "testdir", false, false).unwrap();
        assert!(!dir_path.exists());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rm_recursive() {
        let (pool_mgr, test_dir) = setup_test_pool("recursive");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create a nested directory structure
        let dir_path = test_dir.join("branch1").join("testdir");
        let nested_path = dir_path.join("nested");
        fs::create_dir_all(&nested_path).unwrap();
        fs::write(nested_path.join("file.txt"), "content").unwrap();
        assert!(dir_path.exists());

        // Remove recursively
        execute(pool, "testdir", true, false).unwrap();
        assert!(!dir_path.exists());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_rm_nonexistent() {
        let (pool_mgr, test_dir) = setup_test_pool("nonexistent");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Try to remove a nonexistent file - should return error
        let result = execute(pool, "nonexistent.txt", false, false);
        assert!(result.is_err(), "Should fail when file doesn't exist");

        cleanup_test_dir(&test_dir);
    }
}
