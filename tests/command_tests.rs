//! Command e2e tests.

#[path = "common.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::common::TestContext;

    #[test]
    fn ls_command() {
        let ctx = TestContext::new("cmd_ls");

        let _ = ctx.create_branch("disk1/dir", &["file1.txt", "file2.txt"]);
        let _ = ctx.create_branch("disk2/dir", &["file3.txt", "file4.txt"]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "ls",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("file1.txt"));
        assert!(output.stdout.contains("file2.txt"));
        assert!(output.stdout.contains("file3.txt"));
        assert!(output.stdout.contains("file4.txt"));
    }

    #[test]
    fn ls_long_format() {
        let ctx = TestContext::new("cmd_ls_long");

        let _ = ctx.create_branch("disk1/dir", &["file1.txt"]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "ls",
            "-l",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Long format should show file size
        assert!(output.stdout.contains('B') || output.stdout.contains('1'));
    }

    #[test]
    fn ls_hidden_files() {
        let ctx = TestContext::new("cmd_ls_hidden");

        let _ = ctx.create_branch("disk1/dir", &[".hidden", "visible.txt"]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // Without -a, hidden files should not appear
        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "ls",
            "test:dir",
        ]);

        assert!(output.success());
        assert!(output.stdout.contains("visible.txt"));
        assert!(!output.stdout.contains(".hidden"));

        // With -a, hidden files should appear
        let output2 = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "ls",
            "-a",
            "test:dir",
        ]);

        assert!(output2.success());
        assert!(output2.stdout.contains(".hidden"));
    }

    #[test]
    fn where_command() {
        let ctx = TestContext::new("cmd_where");

        let _ = ctx.create_branch("disk1/dir", &["unique_file.txt"]);
        let _ = ctx.create_branch("disk2/dir", &["other.txt"]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "where",
            "test:dir/unique_file.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("disk1/dir/unique_file.txt"));
    }

    #[test]
    fn where_all_flag() {
        let ctx = TestContext::new("cmd_where_all");

        // Create same filename in multiple branches
        let _ = ctx.create_branch("disk1/dir", &["shared.txt"]);
        let _ = ctx.create_branch("disk2/dir", &["shared.txt"]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "where",
            "-a",
            "test:dir/shared.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Should show both disk1 and disk2
        assert!(output.stdout.contains("disk1"));
        assert!(output.stdout.contains("disk2"));
    }

    #[test]
    fn find_command() {
        let ctx = TestContext::new("cmd_find");

        let _ = ctx.create_branch("disk1", &["file1.txt", "file2.log", "subdir/file3.txt"]);
        let _ = ctx.create_branch("disk2", &["file4.txt", "file5.log"]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "find",
            "test:.",
            "--name",
            "*.log",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("file2.log"));
        assert!(output.stdout.contains("file5.log"));
        assert!(!output.stdout.contains("file1.txt"));
    }

    #[test]
    fn find_type_filter() {
        let ctx = TestContext::new("cmd_find_type");

        let _ = ctx.create_branch("disk1", &["file.txt", "subdir/nested.txt"]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // Find directories
        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "find",
            "test:.",
            "--type",
            "d",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("subdir"));

        // Find files
        let output2 = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "find",
            "test:.",
            "--type",
            "f",
        ]);

        assert!(output2.success(), "Command failed: {}", output.stderr);
        assert!(output2.stdout.contains("file.txt"));
    }

    #[test]
    fn create_command() {
        let ctx = TestContext::new("cmd_create");

        let _ = ctx.create_branch("disk1", &[]);
        let _ = ctx.create_branch("disk2", &[]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1", "{0}/disk2"]
create_policy = "mfs"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "create",
            "test:newfile.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("newfile.txt"));
        // Should be on one of the disks
        assert!(output.stdout.contains("disk1") || output.stdout.contains("disk2"));
    }

    #[test]
    fn exists_command() {
        let ctx = TestContext::new("cmd_exists");

        let _ = ctx.create_branch("disk1/dir", &["present.txt"]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // File exists
        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "exists",
            "test:dir/present.txt",
        ]);

        assert!(output.success(), "exists should return 0 for existing file");
        assert!(output.stdout.contains("present.txt"));

        // File doesn't exist
        let output2 = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "exists",
            "test:dir/missing.txt",
        ]);

        assert!(
            !output2.success(),
            "exists should return 1 for missing file"
        );
    }

    #[test]
    fn stat_command() {
        let ctx = TestContext::new("cmd_stat");

        let _ = ctx.create_branch("disk1", &[]);
        let _ = ctx.create_branch("disk2", &[]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "stat", "-H"]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("Total:"));
        assert!(output.stdout.contains("Used:"));
        assert!(output.stdout.contains("Available:"));
        assert!(output.stdout.contains("disk1"));
        assert!(output.stdout.contains("disk2"));
    }

    #[test]
    fn info_command() {
        let ctx = TestContext::new("cmd_info");

        let _ = ctx.create_branch("disk1", &[]);
        let _ = ctx.create_branch("disk2", &[]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1", "{0}/disk2"]
create_policy = "pfrd"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "info",
            "test",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("Union Context: test"));
        assert!(output.stdout.contains("Branches:     2"));
        assert!(output.stdout.contains("Create:     pfrd"));
    }

    #[test]
    fn cat_command() {
        let ctx = TestContext::new("cmd_cat");

        let _ = ctx.create_branch("disk1/dir", &["file.txt"]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "cat",
            "test:dir/file.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("content of file.txt"));
    }

    #[test]
    fn deduplication_across_branches() {
        let ctx = TestContext::new("cmd_dedup");

        // Same filename in multiple branches
        let _ = ctx.create_branch("disk1/dir", &["shared.txt"]);
        let _ = ctx.create_branch("disk2/dir", &["shared.txt"]);
        let _ = ctx.create_branch("disk3/dir", &["shared.txt"]);

        let config = format!(
            r#"
[union.test]
paths = ["{0}/disk1", "{0}/disk2", "{0}/disk3"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "ls",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Should only show "shared.txt" once despite being in 3 branches
        let count = output.stdout.matches("shared.txt").count();
        assert_eq!(count, 1, "File should appear only once (deduplicated)");
    }
}
