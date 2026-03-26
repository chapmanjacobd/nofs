//! Policy selection tests.

#[path = "common.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::common::TestContext;

    #[test]
    fn pfrd_policy() {
        let ctx = TestContext::new("policy_pfrd");

        let _ = ctx.create_branch("disk1", &["file1.txt"]);
        let _ = ctx.create_branch("disk2", &["file2.txt"]);

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
            "create",
            "test:newfile.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stdout.contains("newfile.txt"));
    }

    #[test]
    fn mfs_policy() {
        let ctx = TestContext::new("policy_mfs");

        let _ = ctx.create_branch("disk1", &["file1.txt"]);
        let _ = ctx.create_branch("disk2", &["file2.txt"]);

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
        // MFS should select the disk with most free space
        assert!(output.stdout.contains("newfile.txt"));
    }

    #[test]
    fn rand_policy() {
        let ctx = TestContext::new("policy_rand");

        let _ = ctx.create_branch("disk1", &["file1.txt"]);
        let _ = ctx.create_branch("disk2", &["file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1", "{0}/disk2"]
create_policy = "rand"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        // Run multiple times to verify randomness doesn't crash
        for _ in 0..5 {
            let output = ctx.run_nofs(&[
                "--config",
                ctx.config_path.to_str().unwrap(),
                "create",
                "test:newfile.txt",
            ]);

            assert!(output.success(), "Command failed: {}", output.stderr);
            assert!(output.stdout.contains("newfile.txt"));
        }
    }

    #[test]
    fn ff_search_policy() {
        let ctx = TestContext::new("policy_ff");

        let _ = ctx.create_branch("disk1/subdir", &["file1.txt"]);
        let _ = ctx.create_branch("disk2/subdir", &["file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/disk1", "{0}/disk2"]
search_policy = "ff"
"#,
            ctx.root.display()
        );

        ctx.write_config(&config);

        let output = ctx.run_nofs(&[
            "--config",
            ctx.config_path.to_str().unwrap(),
            "-v",
            "where",
            "test:subdir/file1.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stderr.contains("first-found policy"));
    }

    #[test]
    fn ro_branch_excluded_from_create() {
        let ctx = TestContext::new("policy_ro_exclude");

        let _ = ctx.create_branch("rw_disk", &["file1.txt"]);
        let _ = ctx.create_branch("ro_disk", &["file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/rw_disk"]
ro_paths = ["{0}/ro_disk"]
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
        // Should select RW disk, not RO disk
        assert!(output.stdout.contains("rw_disk"));
        assert!(!output.stdout.contains("ro_disk"));
    }

    #[test]
    fn nc_branch_excluded_from_create() {
        let ctx = TestContext::new("policy_nc_exclude");

        let _ = ctx.create_branch("rw_disk", &["file1.txt"]);
        let _ = ctx.create_branch("nc_disk", &["file2.txt"]);

        let config = format!(
            r#"
[share.test]
paths = ["{0}/rw_disk"]
nc_paths = ["{0}/nc_disk"]
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
        // Should select RW disk, not NC disk
        assert!(output.stdout.contains("rw_disk"));
        assert!(!output.stdout.contains("nc_disk"));
    }

    #[test]
    fn verbose_shows_policy() {
        let ctx = TestContext::new("policy_verbose");

        let _ = ctx.create_branch("disk1", &["file1.txt"]);
        let _ = ctx.create_branch("disk2", &["file2.txt"]);

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
            "-v",
            "create",
            "test:newfile.txt",
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        assert!(output.stderr.contains("selected:"));
        assert!(output.stderr.contains("pfrd policy"));
    }
}
