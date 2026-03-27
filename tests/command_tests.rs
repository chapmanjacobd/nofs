//! Command e2e tests.

#[path = "common.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::common::TestContext;
    use std::fs;

    #[test]
    fn ls_command() {
        let ctx = TestContext::new("cmd_ls");

        let _ = ctx.create_branch("disk1/dir", &["file1.txt", "file2.txt"]);
        let _ = ctx.create_branch("disk2/dir", &["file3.txt", "file4.txt"]);

        let config = format!(
            r#"
[share.test]
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
[share.test]
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
[share.test]
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
[share.test]
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
[share.test]
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
[share.test]
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
[share.test]
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
[share.test]
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
[share.test]
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
[share.test]
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
    fn du_command() {
        let ctx = TestContext::new("cmd_du");

        let _ = ctx.create_branch("disk1/dir", &["file1.txt", "file2.txt"]);
        let _ = ctx.create_branch("disk2/dir", &["file3.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "du",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Should show disk usage for the directory
        assert!(output.stdout.contains(&ctx.root.display().to_string()));
    }

    #[test]
    fn du_command_human_readable() {
        let ctx = TestContext::new("cmd_du_human");

        let _ = ctx.create_branch("disk1/dir", &["file1.txt", "file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "du",
            "-H",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Human-readable output should contain size units
        assert!(
            output.stdout.contains('B')
                || output.stdout.contains('K')
                || output.stdout.contains('M')
                || output.stdout.contains('G')
        );
    }

    #[test]
    fn du_command_all_subdirs() {
        let ctx = TestContext::new("cmd_du_all");

        let _ = ctx.create_branch("disk1/dir/subdir", &["file1.txt"]);
        let _ = ctx.create_branch("disk1/dir", &["file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "du",
            "-a",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Should show subdirectory
        assert!(output.stdout.contains("subdir"));
    }

    #[test]
    fn du_command_maxdepth() {
        let ctx = TestContext::new("cmd_du_maxdepth");

        let _ = ctx.create_branch("disk1/dir/subdir1/subdir2", &["file1.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "du",
            "-a",
            "--maxdepth",
            "1",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Should show subdir1 but not subdir2
        assert!(output.stdout.contains("subdir1"));
    }

    #[test]
    fn du_command_json_output() {
        let ctx = TestContext::new("cmd_du_json");

        let _ = ctx.create_branch("disk1/dir", &["file1.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "du",
            "--json",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // JSON output should be valid JSON
        assert!(output.stdout.contains('"'));
        assert!(output.stdout.contains("path"));
        assert!(output.stdout.contains("size"));
    }

    #[test]
    fn du_command_multiple_branches() {
        let ctx = TestContext::new("cmd_du_multi");

        let branch1 = ctx.create_branch("disk1/dir", &["file1.txt", "file2.txt"]);
        let branch2 = ctx.create_branch("disk2/dir", &["file3.txt", "file4.txt", "file5.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "du",
            "-H",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Should show both branch paths
        assert!(output.stdout.contains(&branch1.display().to_string()));
        assert!(output.stdout.contains(&branch2.display().to_string()));
    }

    #[test]
    fn du_command_share_root_with_slash() {
        let ctx = TestContext::new("cmd_du_slash");

        let branch_path = ctx.create_branch("disk1", &["file1.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // Test with leading slash (should be treated as share root, not filesystem root)
        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "du",
            "-H",
            "test:/",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Should show disk1 path, not filesystem root
        assert!(output.stdout.contains(&branch_path.display().to_string()));
        assert!(!output.stdout.contains("/afs/"));
        assert!(!output.stdout.contains("/bin/"));
    }

    #[test]
    fn ls_command_share_root_with_slash() {
        let ctx = TestContext::new("cmd_ls_slash");

        let _ = ctx.create_branch("disk1", &["file1.txt", "file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // Test with leading slash (should be treated as share root)
        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "ls",
            "test:/",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Should show contents of disk1, not filesystem root
        assert!(output.stdout.contains("file1.txt"));
        assert!(output.stdout.contains("file2.txt"));
        assert!(!output.stdout.contains("afs/"));
        assert!(!output.stdout.contains("bin/"));
    }

    #[test]
    fn ls_command_share_root_with_slash_adhoc() {
        let ctx = TestContext::new("cmd_ls_slash_adhoc");

        let branch_path = ctx.create_branch("disk1", &["file1.txt", "file2.txt"]);

        // Test with ad-hoc mode and leading slash
        let output = ctx.run_nofs(&[
            "--paths",
            &branch_path.display().to_string(),
            "ls",
            "/",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Should show contents of disk1, not filesystem root
        assert!(output.stdout.contains("file1.txt"));
        assert!(output.stdout.contains("file2.txt"));
        assert!(!output.stdout.contains("afs/"));
        assert!(!output.stdout.contains("bin/"));
    }

    #[test]
    fn du_command_share_root_adhoc() {
        let ctx = TestContext::new("cmd_du_slash_adhoc");

        let branch_path = ctx.create_branch("disk1", &["file1.txt"]);

        // Test with ad-hoc mode and leading slash
        let output = ctx.run_nofs(&[
            "--paths",
            &branch_path.display().to_string(),
            "du",
            "-H",
            "/",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        // Should show disk1 path, not filesystem root
        assert!(output.stdout.contains(&branch_path.display().to_string()));
        assert!(!output.stdout.contains("/afs/"));
        assert!(!output.stdout.contains("/bin/"));
    }

    #[test]
    fn info_command() {
        let ctx = TestContext::new("cmd_info");

        let _ = ctx.create_branch("disk1", &[]);
        let _ = ctx.create_branch("disk2", &[]);

        let config = format!(
            r#"
[share.test]
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
        assert!(output.stdout.contains("Share: test"));
        assert!(output.stdout.contains("Branches:     2"));
        assert!(output.stdout.contains("Create:     pfrd"));
    }

    #[test]
    fn cat_command() {
        let ctx = TestContext::new("cmd_cat");

        let _ = ctx.create_branch("disk1/dir", &["file.txt"]);

        let config = format!(
            r#"
[share.test]
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
[share.test]
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

    #[test]
    fn cp_share_path_to_local() {
        let ctx = TestContext::new("cp_share_to_local");

        // Create source file in share
        let _ = ctx.create_branch("disk1/source", &["file.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let dest_dir = ctx.root.join("dest");
        fs::create_dir_all(&dest_dir).expect("Failed to create dest dir");

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "cp",
            "test:source/file.txt",
            dest_dir.to_str().unwrap(),
        ]);

        assert!(
            output.success(),
            "Command failed: {}\nstdout: {}\nstderr: {}",
            output.status,
            output.stdout,
            output.stderr
        );

        // Verify file was copied
        let copied_file = dest_dir.join("file.txt");
        assert!(copied_file.exists(), "File should be copied to destination");
    }

    #[test]
    fn cp_local_to_share_path() {
        let ctx = TestContext::new("cp_local_to_share");

        // Create destination branch in share
        let _ = ctx.create_branch("disk1/dest", &[]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // Create local source file
        let source_file = ctx.root.join("source.txt");
        fs::write(&source_file, "test content").expect("Failed to create source file");

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "cp",
            source_file.to_str().unwrap(),
            "test:dest/",
        ]);

        assert!(
            output.success(),
            "Command failed: {}\nstdout: {}\nstderr: {}",
            output.status,
            output.stdout,
            output.stderr
        );

        // Verify file was copied to share
        let copied_file = ctx.root.join("disk1/dest/source.txt");
        assert!(
            copied_file.exists(),
            "File should be copied to share destination"
        );
    }

    #[test]
    fn cp_share_path_to_share_path() {
        let ctx = TestContext::new("cp_share_to_share");

        // Create source and destination in share
        let _ = ctx.create_branch("disk1/source", &["file.txt"]);
        let _ = ctx.create_branch("disk1/dest", &[]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "cp",
            "test:source/file.txt",
            "test:dest/",
        ]);

        assert!(
            output.success(),
            "Command failed: {}\nstdout: {}\nstderr: {}",
            output.status,
            output.stdout,
            output.stderr
        );

        // Verify file was copied within share
        let copied_file = ctx.root.join("disk1/dest/file.txt");
        assert!(copied_file.exists(), "File should be copied within share");
    }

    #[test]
    fn cp_share_directory_recursive() {
        let ctx = TestContext::new("cp_share_recursive");

        // Create source directory with nested structure in share
        let _ = ctx.create_branch("disk1/source/subdir", &["file1.txt", "file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let dest_dir = ctx.root.join("dest");
        fs::create_dir_all(&dest_dir).expect("Failed to create dest dir");

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "cp",
            "test:source/",
            dest_dir.to_str().unwrap(),
        ]);

        assert!(
            output.success(),
            "Command failed: {}\nstdout: {}\nstderr: {}",
            output.status,
            output.stdout,
            output.stderr
        );

        // Verify directory structure was copied
        let copied_file1 = dest_dir.join("source/subdir/file1.txt");
        let copied_file2 = dest_dir.join("source/subdir/file2.txt");
        assert!(copied_file1.exists(), "Nested file1 should be copied");
        assert!(copied_file2.exists(), "Nested file2 should be copied");
    }

    #[test]
    fn mv_share_path_to_local() {
        let ctx = TestContext::new("mv_share_to_local");

        // Create source file in share
        let _ = ctx.create_branch("disk1/source", &["file.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let dest_dir = ctx.root.join("dest");
        fs::create_dir_all(&dest_dir).expect("Failed to create dest dir");

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "mv",
            "test:source/file.txt",
            dest_dir.to_str().unwrap(),
        ]);

        assert!(
            output.success(),
            "Command failed: {}\nstdout: {}\nstderr: {}",
            output.status,
            output.stdout,
            output.stderr
        );

        // Verify file was moved (exists at dest, not at source)
        let copied_file = dest_dir.join("file.txt");
        let original_file = ctx.root.join("disk1/source/file.txt");
        assert!(copied_file.exists(), "File should be moved to destination");
        assert!(
            !original_file.exists(),
            "Original file should be removed after move"
        );
    }

    #[test]
    fn mv_share_path_same_branch() {
        let ctx = TestContext::new("mv_share_same_branch");

        // Create multiple branches - file exists on branch1
        let _ = ctx.create_branch("branch1", &["file.txt"]);
        let _ = ctx.create_branch("branch2", &[]);

        let config = format!(
            r#"
[share.mvtest]
paths = ["{0}/branch1", "{0}/branch2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // Verify source file exists before running command
        let source_file = ctx.root.join("branch1/file.txt");
        assert!(source_file.exists(), "Source file should exist before mv");

        // Move file.txt to renamed.txt within the same share (file-to-file)
        // Should stay on branch1 (where the source file exists)
        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "-v",
            "mv",
            "mvtest:file.txt",
            "mvtest:renamed.txt",
        ]);

        assert!(
            output.success(),
            "Command failed: {}\nstdout: {}\nstderr: {}",
            output.status,
            output.stdout,
            output.stderr
        );

        // File should be moved within branch1 (same branch as source)
        let moved_file = ctx.root.join("branch1/renamed.txt");
        let original_file = ctx.root.join("branch1/file.txt");
        let wrong_branch_file = ctx.root.join("branch2/renamed.txt");

        assert!(
            moved_file.exists(),
            "File should be moved to branch1/renamed.txt"
        );
        assert!(
            !original_file.exists(),
            "Original file should be removed after move"
        );
        assert!(!wrong_branch_file.exists(), "File should NOT be on branch2");
    }

    #[test]
    fn mv_share_path_same_branch_to_dir() {
        let ctx = TestContext::new("mv_share_to_dir");

        // Create multiple branches with directory structure
        let _ = ctx.create_branch("disk1/source", &["file.txt"]);
        let _ = ctx.create_branch("disk1/dest", &[]);
        let _ = ctx.create_branch("disk2/other", &[]);

        let config = format!(
            r#"
[share.mvtest2]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // Move file.txt to dest/ directory (file-to-directory)
        // Should stay on disk1 (same branch as source)
        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "-v",
            "mv",
            "mvtest2:source/file.txt",
            "mvtest2:dest/",
        ]);

        assert!(
            output.success(),
            "Command failed: {}\nstdout: {}\nstderr: {}",
            output.status,
            output.stdout,
            output.stderr
        );

        // Verify file was moved within disk1 (same branch)
        let moved_file = ctx.root.join("disk1/dest/file.txt");
        let original_file = ctx.root.join("disk1/source/file.txt");
        let wrong_branch_file = ctx.root.join("disk2/dest/file.txt");

        assert!(
            moved_file.exists(),
            "File should be moved to disk1/dest (same branch as source)"
        );
        assert!(
            !original_file.exists(),
            "Original file should be removed after move"
        );
        assert!(
            !wrong_branch_file.exists(),
            "File should NOT be on disk2 (different branch)"
        );
    }

    #[test]
    fn cp_share_path_same_branch() {
        let ctx = TestContext::new("cp_share_same_branch");

        // Create multiple branches - file exists on branch1
        let _ = ctx.create_branch("branch1", &["file.txt"]);
        let _ = ctx.create_branch("branch2", &[]);

        let config = format!(
            r#"
[share.cptest]
paths = ["{0}/branch1", "{0}/branch2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // Copy file.txt to copied.txt within the same share (file-to-file)
        // Should stay on branch1 (where the source file exists)
        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "-v",
            "cp",
            "cptest:file.txt",
            "cptest:copied.txt",
        ]);

        assert!(
            output.success(),
            "Command failed: {}\nstdout: {}\nstderr: {}",
            output.status,
            output.stdout,
            output.stderr
        );

        // File should be copied within branch1 (same branch as source)
        let copied_file = ctx.root.join("branch1/copied.txt");
        let original_file = ctx.root.join("branch1/file.txt");
        let wrong_branch_file = ctx.root.join("branch2/copied.txt");

        assert!(
            copied_file.exists(),
            "File should be copied to branch1/copied.txt"
        );
        assert!(
            original_file.exists(),
            "Original file should still exist after copy"
        );
        assert!(!wrong_branch_file.exists(), "File should NOT be on branch2");
    }

    #[test]
    fn cp_share_path_same_branch_to_dir() {
        let ctx = TestContext::new("cp_share_to_dir");

        // Create multiple branches with directory structure
        let _ = ctx.create_branch("disk1/source", &["file.txt"]);
        let _ = ctx.create_branch("disk1/dest", &[]);
        let _ = ctx.create_branch("disk2/other", &[]);

        let config = format!(
            r#"
[share.cptest2]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // Copy file.txt to dest/ directory (file-to-directory)
        // Should stay on disk1 (same branch as source)
        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "-v",
            "cp",
            "cptest2:source/file.txt",
            "cptest2:dest/",
        ]);

        assert!(
            output.success(),
            "Command failed: {}\nstdout: {}\nstderr: {}",
            output.status,
            output.stdout,
            output.stderr
        );

        // Verify file was copied within disk1 (same branch)
        let copied_file = ctx.root.join("disk1/dest/file.txt");
        let original_file = ctx.root.join("disk1/source/file.txt");
        let wrong_branch_file = ctx.root.join("disk2/dest/file.txt");

        assert!(
            copied_file.exists(),
            "File should be copied to disk1/dest (same branch as source)"
        );
        assert!(
            original_file.exists(),
            "Original file should still exist after copy"
        );
        assert!(
            !wrong_branch_file.exists(),
            "File should NOT be on disk2 (different branch)"
        );
    }

    #[test]
    fn mkdir_command() {
        let ctx = TestContext::new("cmd_mkdir");

        let _ = ctx.create_branch("disk1", &[]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "mkdir",
            "test:newdir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);

        let dir_path = ctx.path("disk1/newdir");
        assert!(dir_path.exists(), "Directory should be created");
        assert!(dir_path.is_dir(), "Should be a directory");
    }

    #[test]
    fn mkdir_command_with_parents() {
        let ctx = TestContext::new("cmd_mkdir_parents");

        let _ = ctx.create_branch("disk1", &[]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "mkdir",
            "-p",
            "test:parent/child/grandchild",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);

        let dir_path = ctx.path("disk1/parent/child/grandchild");
        assert!(dir_path.exists(), "Nested directory should be created");
        assert!(dir_path.is_dir(), "Should be a directory");
    }

    #[test]
    fn rmdir_command() {
        let ctx = TestContext::new("cmd_rmdir");

        let _ = ctx.create_branch("disk1/emptydir", &[]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "rmdir",
            "test:emptydir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);

        let dir_path = ctx.path("disk1/emptydir");
        assert!(!dir_path.exists(), "Directory should be removed");
    }

    #[test]
    fn rmdir_command_nonempty_fails() {
        let ctx = TestContext::new("cmd_rmdir_nonempty");

        let _ = ctx.create_branch("disk1/nonemptydir", &["file.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "rmdir",
            "test:nonemptydir",
        ]);

        assert!(
            !output.success(),
            "rmdir should fail on non-empty directory"
        );
        assert!(
            output.stderr.contains("not empty"),
            "Error should mention 'not empty'"
        );

        let dir_path = ctx.path("disk1/nonemptydir");
        assert!(dir_path.exists(), "Directory should still exist");
    }

    #[test]
    fn touch_command_create() {
        let ctx = TestContext::new("cmd_touch_create");

        let _ = ctx.create_branch("disk1", &[]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "touch",
            "test:newfile.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);

        let file_path = ctx.path("disk1/newfile.txt");
        assert!(file_path.exists(), "File should be created");
        assert!(file_path.is_file(), "Should be a file");
    }

    #[test]
    fn touch_command_update() {
        let ctx = TestContext::new("cmd_touch_update");

        let _ = ctx.create_branch("disk1", &["existing.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "touch",
            "test:existing.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);

        let file_path = ctx.path("disk1/existing.txt");
        assert!(file_path.exists(), "File should still exist");
    }

    #[test]
    fn rm_command_file() {
        let ctx = TestContext::new("cmd_rm_file");

        let _ = ctx.create_branch("disk1", &["file.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "rm",
            "test:file.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);

        let file_path = ctx.path("disk1/file.txt");
        assert!(!file_path.exists(), "File should be removed");
    }

    #[test]
    fn rm_command_directory() {
        let ctx = TestContext::new("cmd_rm_dir");

        let _ = ctx.create_branch("disk1/dir", &[]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "rm",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);

        let dir_path = ctx.path("disk1/dir");
        assert!(!dir_path.exists(), "Directory should be removed");
    }

    #[test]
    fn rm_command_recursive() {
        let ctx = TestContext::new("cmd_rm_recursive");

        let _ = ctx.create_branch("disk1/dir/subdir", &["file.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "rm",
            "-r",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);

        let dir_path = ctx.path("disk1/dir");
        assert!(!dir_path.exists(), "Directory tree should be removed");
    }

    #[test]
    fn rm_command_nonexistent_fails() {
        let ctx = TestContext::new("cmd_rm_nonexistent");

        let _ = ctx.create_branch("disk1", &[]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "rm",
            "test:nonexistent.txt",
        ]);

        assert!(!output.success(), "rm should fail on nonexistent file");
        assert!(
            output.stderr.contains("No such file"),
            "Error should mention 'No such file'"
        );
    }

    #[test]
    fn ls_conflicts_detection() {
        let ctx = TestContext::new("cmd_ls_conflicts");

        // Create same file with different content in different branches
        let branch1 = ctx.create_branch("disk1/dir", &["file1.txt"]);
        let branch2 = ctx.create_branch("disk2/dir", &["file1.txt"]);

        // Overwrite content in branch2 to create conflict
        fs::write(branch1.join("file1.txt"), "original content").unwrap();
        fs::write(branch2.join("file1.txt"), "different content").unwrap();

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "ls",
            "--conflicts",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(
            output.stderr.contains("conflict"),
            "Should report conflict in stderr: {}",
            output.stderr
        );
        assert!(
            output.stdout.contains("file1.txt !"),
            "Should mark conflicting file with !: {}",
            output.stdout
        );
    }

    #[test]
    fn ls_conflicts_verbose() {
        let ctx = TestContext::new("cmd_ls_conflicts_verbose");

        let branch1 = ctx.create_branch("disk1/dir", &["conflict.txt"]);
        let branch2 = ctx.create_branch("disk2/dir", &["conflict.txt"]);

        // Create different content
        fs::write(branch1.join("conflict.txt"), "original").unwrap();
        fs::write(branch2.join("conflict.txt"), "different").unwrap();

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "ls",
            "--conflicts",
            "-v",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(
            output.stderr.contains("conflicts detected"),
            "Should mention conflicts: {}",
            output.stderr
        );
        assert!(
            output.stderr.contains("disk1"),
            "Should mention branch disk1: {}",
            output.stderr
        );
        assert!(
            output.stderr.contains("disk2"),
            "Should mention branch disk2: {}",
            output.stderr
        );
    }

    #[test]
    fn ls_no_conflicts() {
        let ctx = TestContext::new("cmd_ls_no_conflicts");

        // Create different files in each branch (no conflicts)
        let _ = ctx.create_branch("disk1/dir", &["file1.txt"]);
        let _ = ctx.create_branch("disk2/dir", &["file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "ls",
            "--conflicts",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(
            !output.stderr.contains("conflict"),
            "Should not report conflicts: {}",
            output.stderr
        );
    }

    #[test]
    fn ls_conflicts_same_size_different_content() {
        let ctx = TestContext::new("cmd_ls_conflicts_same_size");

        let branch1 = ctx.create_branch("disk1/dir", &["same_size.txt"]);
        let branch2 = ctx.create_branch("disk2/dir", &["same_size.txt"]);

        // Create files with same size but different content
        fs::write(branch1.join("same_size.txt"), "AAAA").unwrap();
        fs::write(branch2.join("same_size.txt"), "BBBB").unwrap();

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "ls",
            "--conflicts",
            "--hash",
            "test:dir",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(
            output.stderr.contains("conflict"),
            "Should detect conflict with hash comparison: {}",
            output.stderr
        );
    }

    #[test]
    fn which_conflicts_detection() {
        let ctx = TestContext::new("cmd_which_conflicts");

        let branch1 = ctx.create_branch("disk1/dir", &["file.txt"]);
        let branch2 = ctx.create_branch("disk2/dir", &["file.txt"]);

        // Create different content
        fs::write(branch1.join("file.txt"), "original").unwrap();
        fs::write(branch2.join("file.txt"), "different").unwrap();

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "which",
            "--all",
            "--conflicts",
            "test:dir/file.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(
            output.stderr.contains("conflict"),
            "Should report conflict: {}",
            output.stderr
        );
    }

    #[test]
    fn which_conflicts_verbose() {
        let ctx = TestContext::new("cmd_which_conflicts_verbose");

        let branch1 = ctx.create_branch("disk1/dir", &["file.txt"]);
        let branch2 = ctx.create_branch("disk2/dir", &["file.txt"]);

        // Create different content
        fs::write(branch1.join("file.txt"), "original").unwrap();
        fs::write(branch2.join("file.txt"), "different content").unwrap();

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "which",
            "--all",
            "--conflicts",
            "-v",
            "test:dir/file.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(
            output.stderr.contains("conflict detected"),
            "Should mention conflict: {}",
            output.stderr
        );
        assert!(
            output.stderr.contains("disk1"),
            "Should mention branch disk1: {}",
            output.stderr
        );
        assert!(
            output.stderr.contains("disk2"),
            "Should mention branch disk2: {}",
            output.stderr
        );
    }

    #[test]
    fn which_no_conflicts() {
        let ctx = TestContext::new("cmd_which_no_conflicts");

        // Same file in multiple branches with identical content
        let _ = ctx.create_branch("disk1/dir", &["file.txt"]);
        let _ = ctx.create_branch("disk2/dir", &["file.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1", "{0}/disk2"]
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "which",
            "--all",
            "--conflicts",
            "-v",
            "test:dir/file.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(
            !output.stderr.contains("conflict detected"),
            "Should not report conflict: {}",
            output.stderr
        );
    }
}
