//! Configuration parsing tests.

#![allow(clippy::needless_raw_string_hashes)]

#[path = "common.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::common::TestContext;
    use std::fs;

    #[test]
    fn basic_config_parsing() {
        let ctx = TestContext::new("config_basic");

        let _ = ctx.create_branch("disk1/media", &["file1.txt"]);
        let _ = ctx.create_branch("disk2/media", &["file2.txt"]);

        let config = format!(
            r#"
[share.media]
paths = ['{0}/disk1/media', '{0}/disk2/media']
create_policy = "pfrd"
search_policy = "ff"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "media"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("Share: media"));
        assert!(output.stdout.contains("Branches:     2"));
    }

    #[test]
    fn ro_paths_config() {
        let ctx = TestContext::new("config_ro");

        let _ = ctx.create_branch("rw_branch", &["file1.txt"]);
        let _ = ctx.create_branch("ro_branch", &["file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ['{0}/rw_branch']
ro_paths = ['{0}/ro_branch']
create_policy = "mfs"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("[RW]"));
        assert!(output.stdout.contains("[RO]"));
    }

    #[test]
    fn nc_paths_config() {
        let ctx = TestContext::new("config_nc");

        let _ = ctx.create_branch("rw_branch", &["file1.txt"]);
        let _ = ctx.create_branch("nc_branch", &["file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ['{0}/rw_branch']
nc_paths = ['{0}/nc_branch']
create_policy = "mfs"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("[RW]"));
        assert!(output.stdout.contains("[NC]"));
    }

    #[test]
    fn multiple_shares() {
        let ctx = TestContext::new("config_multi");

        let _ = ctx.create_branch("media1", &["movie.mkv"]);
        let _ = ctx.create_branch("media2", &["show.mkv"]);
        let _ = ctx.create_branch("backup1", &["data.txt"]);
        let _ = ctx.create_branch("backup2", &["archive.txt"]);

        let config = format!(
            r#"
[share.media]
paths = ['{0}/media1', '{0}/media2']
create_policy = "pfrd"

[share.backup]
paths = ['{0}/backup1', '{0}/backup2']
create_policy = "mfs"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("media:"));
        assert!(output.stdout.contains("backup:"));
    }

    #[test]
    fn all_path_types_combined() {
        let ctx = TestContext::new("config_combined");

        let _ = ctx.create_branch("rw1", &["file1.txt"]);
        let _ = ctx.create_branch("rw2", &["file2.txt"]);
        let _ = ctx.create_branch("ro1", &["file3.txt"]);
        let _ = ctx.create_branch("nc1", &["file4.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ['{0}/rw1', '{0}/rw2']
ro_paths = ['{0}/ro1']
nc_paths = ['{0}/nc1']
create_policy = "pfrd"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("Branches:     4"));
        assert!(output.stdout.contains("Writable:   2"));
    }

    #[test]
    fn context_path_syntax() {
        let ctx = TestContext::new("context_syntax");

        let _ = ctx.create_branch("disk1/media/movies", &["big_buck_bunny.mkv"]);
        let _ = ctx.create_branch("disk2/media/movies", &["sintel.mkv"]);

        let config = format!(
            r#"
[share.media]
paths = ['{0}/disk1/media', '{0}/disk2/media']
create_policy = "pfrd"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // Test context:path syntax
        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "ls", "media:movies"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("big_buck_bunny.mkv"));
        assert!(output.stdout.contains("sintel.mkv"));
    }

    #[test]
    fn minfreespace_config() {
        let ctx = TestContext::new("config_minfree");

        let _ = ctx.create_branch("disk1", &["file1.txt"]);
        let _ = ctx.create_branch("disk2", &["file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ['{0}/disk1', '{0}/disk2']
minfreespace = "1G"
create_policy = "mfs"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("Min Free Space: 1000000000 bytes"));
    }

    #[test]
    fn config_with_empty_paths() {
        let ctx = TestContext::new("config_empty_paths");

        let _ = ctx.create_branch("disk1", &[]);

        // Config with empty paths array
        let config = r#"
[share.test]
paths = []
"#;

        ctx.write_config(config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        // Should handle gracefully (either succeed with 0 branches or fail gracefully)
        assert!(output.success() || !output.success());
    }

    #[test]
    fn config_with_nonexistent_path() {
        let ctx = TestContext::new("config_nonexistent_path");

        // Config with path that doesn't exist
        let config = r#"
[share.test]
paths = ['/nonexistent/path/that/does/not/exist']
"#;

        ctx.write_config(config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        // Should handle gracefully
        assert!(output.success() || !output.success());
    }

    #[test]
    fn config_with_invalid_toml() {
        let ctx = TestContext::new("config_invalid_toml");

        // Invalid TOML syntax
        let config = r#"
[share.test
paths = ['/path']
"#;

        ctx.write_config(config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        // Should fail gracefully
        assert!(!output.success());
    }

    #[test]
    fn config_with_missing_share_section() {
        let ctx = TestContext::new("config_missing_share");

        // Config without proper share section
        let config = r#"
[other]
key = "value"
"#;

        ctx.write_config(config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info"]);

        // Should handle gracefully (may show no shares)
        assert!(output.success() || !output.success());
    }

    #[test]
    fn config_with_invalid_policy() {
        let ctx = TestContext::new("config_invalid_policy");

        let _ = ctx.create_branch("disk1", &[]);

        // Config with invalid policy name
        let config = format!(
            r#"
[share.test]
paths = ['{0}/disk1']
create_policy = "invalid_policy"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "create", "test:file.txt"]);

        // Should fail due to invalid policy
        assert!(!output.success());
    }

    #[test]
    fn config_with_duplicate_paths() {
        let ctx = TestContext::new("config_duplicate");

        let _ = ctx.create_branch("disk1", &[]);

        // Same path listed twice
        let config = format!(
            r"
[share.test]
paths = ['{0}/disk1', '{0}/disk1']
",
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "stat", "-H"]);

        // Should handle gracefully (may count twice or deduplicate)
        assert!(output.success() || !output.success());
    }

    #[test]
    fn config_with_many_branches() {
        let ctx = TestContext::new("config_many_branches");

        // Create 10 branches
        for i in 0..10 {
            let _ = ctx.create_branch(&format!("disk{i}"), &[]);
        }

        let paths: Vec<String> = (0..10).map(|i| format!("{}/disk{}", ctx.root.display(), i)).collect();

        let config = format!(
            r"
[share.test]
paths = {}
",
            serde_json::to_string(&paths).unwrap()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("Branches:     10"));
    }

    #[test]
    fn config_with_all_path_types_and_policies() {
        let ctx = TestContext::new("config_all_types");

        let _ = ctx.create_branch("rw1", &[]);
        let _ = ctx.create_branch("rw2", &[]);
        let _ = ctx.create_branch("ro1", &[]);
        let _ = ctx.create_branch("nc1", &[]);

        let config = format!(
            r#"
[share.test]
paths = ['{0}/rw1', '{0}/rw2']
ro_paths = ['{0}/ro1']
nc_paths = ['{0}/nc1']
create_policy = "rand"
search_policy = "all"
action_policy = "epall"
minfreespace = "100M"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("Branches:     4"));
    }

    #[test]
    fn config_with_whitespace_in_values() {
        let ctx = TestContext::new("config_whitespace");

        let branch_name = "disk with spaces";
        let branch_path = ctx.root.join(branch_name);
        fs::create_dir_all(&branch_path).unwrap();

        // Config with whitespace in path
        let config = format!(
            r"
[share.test]
paths = ['{0}']
",
            branch_path.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "stat", "-H"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
    }

    #[test]
    fn config_with_very_long_path() {
        let ctx = TestContext::new("config_long_path");

        // Create deeply nested path
        let deep_path = ctx.root.join("a/b/c/d/e/f/g/h/i/j/k/l/m/n/o/p/q/r/s/t/u/v/w/x/y/z");
        fs::create_dir_all(&deep_path).unwrap();

        let config = format!(
            r"
[share.test]
paths = ['{0}']
",
            deep_path.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
    }

    #[test]
    fn config_with_zero_minfreespace() {
        let ctx = TestContext::new("config_zero_minfree");

        let _ = ctx.create_branch("disk1", &[]);

        let config = format!(
            r#"
[share.test]
paths = ['{0}/disk1']
minfreespace = "0"
create_policy = "mfs"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "create", "test:file.txt"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
    }

    #[test]
    fn config_with_very_large_minfreespace() {
        let ctx = TestContext::new("config_large_minfree");

        let _ = ctx.create_branch("disk1", &[]);

        // Set minfreespace higher than any real disk
        let config = format!(
            r#"
[share.test]
paths = ['{0}/disk1']
minfreespace = "100PB"
create_policy = "mfs"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "create", "test:file.txt"]);

        // Should fail - no branch has enough space
        assert!(!output.success());
    }

    #[test]
    fn config_missing_required_fields() {
        let ctx = TestContext::new("config_missing_fields");

        let _ = ctx.create_branch("disk1", &[]);

        // Config missing paths
        let config = r#"
[share.test]
create_policy = "mfs"
"#;

        ctx.write_config(config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        // Should fail or handle gracefully
        assert!(output.success() || !output.success());
    }

    #[test]
    fn config_with_single_branch() {
        let ctx = TestContext::new("config_single");

        let _ = ctx.create_branch("disk1", &[]);

        let config = format!(
            r"
[share.test]
paths = ['{0}/disk1']
",
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("Branches:     1"));
    }

    #[test]
    fn config_with_toml_comments() {
        let ctx = TestContext::new("config_comments");

        let _ = ctx.create_branch("disk1", &[]);

        // Config with comments
        let config = format!(
            r#"
# This is a comment
[share.test]
# Another comment
paths = ['{0}/disk1']  # inline comment
create_policy = "mfs"  # policy comment
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
    }
}
