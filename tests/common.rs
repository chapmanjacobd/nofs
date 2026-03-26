//! Common test utilities

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Test context for managing temporary test directories
pub struct TestContext {
    pub root: PathBuf,
    pub config_path: PathBuf,
}

impl TestContext {
    /// Create a new test context with temporary directories
    pub fn new(test_name: &str) -> Self {
        let root =
            std::env::temp_dir().join(format!("nofs_test_{}_{}", test_name, std::process::id()));

        // Clean up if exists
        if root.exists() {
            let _ = fs::remove_dir_all(&root);
        }

        fs::create_dir_all(&root).expect("Failed to create test root");

        let config_path = root.join("config.toml");

        TestContext { root, config_path }
    }

    /// Create a branch directory structure
    pub fn create_branch(&self, name: &str, files: &[&str]) -> PathBuf {
        let branch_path = self.root.join(name);
        fs::create_dir_all(&branch_path).expect("Failed to create branch");

        for file in files {
            let file_path = branch_path.join(file);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::write(&file_path, format!("content of {}", file)).expect("Failed to create file");
        }

        branch_path
    }

    /// Write a config file
    pub fn write_config(&self, content: &str) {
        fs::write(&self.config_path, content).expect("Failed to write config");
    }

    /// Run nofs command
    pub fn run_nofs(&self, args: &[&str]) -> CommandOutput {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_nofs"));
        for arg in args {
            cmd.arg(arg);
        }

        let output = cmd.output().expect("Failed to run nofs");

        CommandOutput {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            status: output.status,
        }
    }

    /// Get path within test root
    pub fn path(&self, path: &str) -> PathBuf {
        self.root.join(path)
    }
}

impl Drop for TestContext {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// Command output helper
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: std::process::ExitStatus,
}

impl CommandOutput {
    pub fn success(&self) -> bool {
        self.status.success()
    }

    pub fn stdout_contains(&self, text: &str) -> bool {
        self.stdout.contains(text)
    }

    pub fn stderr_contains(&self, text: &str) -> bool {
        self.stderr.contains(text)
    }
}

/// Create a temporary test file and return its path
pub fn temp_file(path: &Path, content: &str) -> PathBuf {
    let file_path = path.join("testfile.txt");
    fs::write(&file_path, content).expect("Failed to write temp file");
    file_path
}
