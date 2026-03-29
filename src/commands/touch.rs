//! touch command - Create or update files

use crate::cache::OperationCache;
use crate::error::Result;
use crate::pool::Pool;
use std::path::Path;

/// Execute the touch command
///
/// # Errors
///
/// Returns an error if the file cannot be created or updated.
pub fn execute(pool: &Pool, path: &str, verbose: bool) -> Result<()> {
    let pool_path = Path::new(path);

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    // Check if file exists in any branch (cached)
    let branches = pool.find_all_branches_cached(pool_path, &cache);

    if branches.is_empty() {
        // File doesn't exist - create on best branch (cached)
        let parent = pool_path.parent().unwrap_or_else(|| Path::new(""));
        let branch = pool.select_create_branch_cached(parent, &cache)?;

        // Create the full path on the selected branch
        let full_path = branch.path.join(pool_path);

        // Ensure parent directory exists
        if let Some(parent_dir) = full_path.parent() {
            std::fs::create_dir_all(parent_dir)?;
        }

        if verbose {
            eprintln!("creating file: {}", full_path.display());
        }
        std::fs::File::create(&full_path)?;
    } else {
        // File exists - update timestamps on all copies
        for branch in branches {
            let full_path = branch.path.join(pool_path);
            if verbose {
                eprintln!("updating timestamp: {}", full_path.display());
            }
            let now = filetime::FileTime::now();
            filetime::set_file_times(&full_path, now, now)?;
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
    use std::time::Duration;

    fn setup_test_pool(name: &str) -> (PoolManager, PathBuf) {
        let test_dir = std::env::temp_dir().join(format!("nofs_test_touch_{name}"));
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
    fn test_touch_create() {
        let (pool_mgr, test_dir) = setup_test_pool("create");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create a new file
        execute(pool, "newfile.txt", false).unwrap();

        let file_path = test_dir.join("branch1").join("newfile.txt");
        assert!(file_path.exists());
        assert!(file_path.is_file());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_touch_update_timestamp() {
        let (pool_mgr, test_dir) = setup_test_pool("update");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create a file first
        let file_path = test_dir.join("branch1").join("existing.txt");
        fs::write(&file_path, "content").unwrap();

        // Get original modification time
        let original_meta = fs::metadata(&file_path).unwrap();
        let original_mtime = original_meta.modified().unwrap();

        // Sleep to ensure time difference
        std::thread::sleep(Duration::from_secs(2));

        // Touch the file to update timestamp
        execute(pool, "existing.txt", false).unwrap();

        // Check that modification time was updated
        let new_meta = fs::metadata(&file_path).unwrap();
        let new_mtime = new_meta.modified().unwrap();

        assert!(new_mtime > original_mtime, "Modification time should be updated");

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_touch_create_with_parent_dirs() {
        let (pool_mgr, test_dir) = setup_test_pool("parent_dirs");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create a file in a nested directory (parent dirs should be created)
        execute(pool, "parent/child/file.txt", false).unwrap();

        let file_path = test_dir.join("branch1").join("parent/child/file.txt");
        assert!(file_path.exists());
        assert!(file_path.is_file());

        cleanup_test_dir(&test_dir);
    }

    #[test]
    fn test_touch_verbose() {
        let (pool_mgr, test_dir) = setup_test_pool("verbose");
        let pool = pool_mgr.get_pool("test").unwrap();

        // Create a file with verbose output
        execute(pool, "verbose.txt", true).unwrap();

        let file_path = test_dir.join("branch1").join("verbose.txt");
        assert!(file_path.exists());

        cleanup_test_dir(&test_dir);
    }
}
