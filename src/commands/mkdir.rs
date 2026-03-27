//! mkdir command - Create directories

use crate::error::Result;
use crate::pool::Pool;
use std::path::Path;

/// Execute the mkdir command
///
/// # Errors
///
/// Returns an error if the directory cannot be created.
pub fn execute(pool: &Pool, path: &str, parents: bool, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Get the best branch for creating this path
    let parent = pool_path.parent().unwrap_or(Path::new(""));
    let branch = pool.select_create_branch(parent)?;

    // Create the full path on the selected branch
    let full_path = branch.path.join(pool_path);

    if parents {
        if verbose {
            eprintln!("creating directory (with parents): {}", full_path.display());
        }
        std::fs::create_dir_all(&full_path)?;
    } else {
        if verbose {
            eprintln!("creating directory: {}", full_path.display());
        }
        std::fs::create_dir(&full_path)?;
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
        let test_dir = std::env::temp_dir().join(format!("nofs_test_mkdir_{name}"));
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
    fn test_mkdir_simple() {
        let (pool_mgr, test_dir) = setup_test_pool("simple");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create a directory
        execute(pool, "newdir", false, false).unwrap();

        let dir_path = test_dir.join("branch1").join("newdir");
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_mkdir_with_parents() {
        let (pool_mgr, test_dir) = setup_test_pool("parents");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create nested directories with parents flag
        execute(pool, "parent/child/grandchild", true, false).unwrap();

        let dir_path = test_dir.join("branch1").join("parent/child/grandchild");
        assert!(dir_path.exists());
        assert!(dir_path.is_dir());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_mkdir_without_parents_fails() {
        let (pool_mgr, test_dir) = setup_test_pool("no_parents");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Try to create nested directory without parents flag - should fail
        let result = execute(pool, "parent/child", false, false);
        assert!(
            result.is_err(),
            "Should fail when parent directory doesn't exist"
        );

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_mkdir_verbose() {
        let (pool_mgr, test_dir) = setup_test_pool("verbose");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create a directory with verbose output
        execute(pool, "verbose_dir", false, true).unwrap();

        let dir_path = test_dir.join("branch1").join("verbose_dir");
        assert!(dir_path.exists());

        cleanup_test_dir(&test_dir);
    }
}
