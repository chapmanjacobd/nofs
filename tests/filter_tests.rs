//! Filtering feature tests for cp/mv commands.

#[path = "common.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::common::TestContext;
    use std::fs;

    #[test]
    fn cp_with_extension_filter() {
        let ctx = TestContext::new("filter_ext");

        let branch = ctx.create_branch("disk1/source", &["file.txt", "file.mkv", "data.log"]);
        let dest_dir = ctx.root.join("dest");
        fs::create_dir_all(&dest_dir).unwrap();

        // Copy only .mkv files using ad-hoc mode
        let output = ctx.run_nofs(&[
            "--paths",
            &branch.parent().unwrap().display().to_string(),
            "cp",
            "-e",
            ".mkv",
            &branch.display().to_string(),
            dest_dir.to_str().unwrap(),
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        let dest_path = dest_dir.join("source/file.mkv");
        assert!(dest_path.exists(), ".mkv file should be copied to {dest_path:?}");
    }

    #[test]
    fn cp_with_size_filter_max() {
        let ctx = TestContext::new("filter_size_max");

        let branch = ctx.create_branch("disk1/source", &["small.txt", "large.txt"]);
        fs::write(branch.join("small.txt"), "x".repeat(100)).unwrap(); // 100 bytes
        fs::write(branch.join("large.txt"), "x".repeat(1000)).unwrap(); // 1000 bytes

        let dest_dir = ctx.root.join("dest");
        fs::create_dir_all(&dest_dir).unwrap();

        // Copy only files smaller than 500 bytes
        let output = ctx.run_nofs(&[
            "--paths",
            &branch.parent().unwrap().display().to_string(),
            "cp",
            "--max-size",
            "500B",
            &branch.display().to_string(),
            dest_dir.to_str().unwrap(),
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        let small_dest = dest_dir.join("source/small.txt");
        let large_dest = dest_dir.join("source/large.txt");
        assert!(small_dest.exists(), "small.txt (100B) should be copied");
        assert!(!large_dest.exists(), "large.txt (1000B) should not be copied");
    }

    #[test]
    fn cp_with_size_filter_min() {
        let ctx = TestContext::new("filter_size_min");

        let branch = ctx.create_branch("disk1/source", &["small.txt", "large.txt"]);
        fs::write(branch.join("small.txt"), "x".repeat(100)).unwrap(); // 100 bytes
        fs::write(branch.join("large.txt"), "x".repeat(1000)).unwrap(); // 1000 bytes

        let dest_dir = ctx.root.join("dest");
        fs::create_dir_all(&dest_dir).unwrap();

        // Copy only files larger than 500 bytes
        let output = ctx.run_nofs(&[
            "--paths",
            &branch.parent().unwrap().display().to_string(),
            "cp",
            "--min-size",
            "500B",
            &branch.display().to_string(),
            dest_dir.to_str().unwrap(),
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        let small_dest = dest_dir.join("source/small.txt");
        let large_dest = dest_dir.join("source/large.txt");
        assert!(!small_dest.exists(), "small.txt (100B) should not be copied");
        assert!(large_dest.exists(), "large.txt (1000B) should be copied");
    }

    #[test]
    fn cp_with_size_filter_range() {
        let ctx = TestContext::new("filter_size_range");

        let branch = ctx.create_branch("disk1/source", &["small.txt", "medium.txt", "large.txt"]);
        fs::write(branch.join("small.txt"), "x".repeat(100)).unwrap(); // 100 bytes
        fs::write(branch.join("medium.txt"), "x".repeat(500)).unwrap(); // 500 bytes
        fs::write(branch.join("large.txt"), "x".repeat(1000)).unwrap(); // 1000 bytes

        let dest_dir = ctx.root.join("dest");
        fs::create_dir_all(&dest_dir).unwrap();

        // Copy only files between 200B and 800B
        let output = ctx.run_nofs(&[
            "--paths",
            &branch.parent().unwrap().display().to_string(),
            "cp",
            "--min-size",
            "200B",
            "--max-size",
            "800B",
            &branch.display().to_string(),
            dest_dir.to_str().unwrap(),
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        let small_dest = dest_dir.join("source/small.txt");
        let medium_dest = dest_dir.join("source/medium.txt");
        let large_dest = dest_dir.join("source/large.txt");
        assert!(!small_dest.exists(), "small.txt (100B) should not be copied");
        assert!(medium_dest.exists(), "medium.txt (500B) should be copied");
        assert!(!large_dest.exists(), "large.txt (1000B) should not be copied");
    }

    #[test]
    fn cp_with_file_limit() {
        let ctx = TestContext::new("filter_limit");

        let branch = ctx.create_branch("disk1/source", &["file1.txt", "file2.txt", "file3.txt", "file4.txt"]);
        let dest_dir = ctx.root.join("dest");
        fs::create_dir_all(&dest_dir).unwrap();

        // Limit to 2 files
        let output = ctx.run_nofs(&[
            "--paths",
            &branch.parent().unwrap().display().to_string(),
            "cp",
            "-l",
            "2",
            &branch.display().to_string(),
            dest_dir.to_str().unwrap(),
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        let dest_path = dest_dir.join("source");
        assert!(dest_path.exists(), "Destination should exist");

        let mut copied_count = 0;
        for entry in fs::read_dir(&dest_path).unwrap().flatten() {
            if entry.path().is_file() {
                copied_count += 1;
            }
        }
        assert_eq!(copied_count, 2, "Should copy exactly 2 files");
    }

    #[test]
    fn cp_with_size_limit() {
        let ctx = TestContext::new("filter_size_limit");

        let branch = ctx.create_branch("disk1/source", &["small.txt", "medium.txt", "large.txt"]);
        fs::write(branch.join("small.txt"), "x".repeat(100)).unwrap(); // 100 bytes
        fs::write(branch.join("medium.txt"), "x".repeat(300)).unwrap(); // 300 bytes
        fs::write(branch.join("large.txt"), "x".repeat(1000)).unwrap(); // 1000 bytes

        let dest_dir = ctx.root.join("dest");
        fs::create_dir_all(&dest_dir).unwrap();

        // Limit total size to 500 bytes
        let output = ctx.run_nofs(&[
            "--paths",
            &branch.parent().unwrap().display().to_string(),
            "cp",
            "--size-limit",
            "500B",
            &branch.display().to_string(),
            dest_dir.to_str().unwrap(),
        ]);

        assert!(output.success(), "Command failed: {}", output.stderr);
        let dest_path = dest_dir.join("source");
        assert!(dest_path.exists(), "Destination should exist");
    }
}
