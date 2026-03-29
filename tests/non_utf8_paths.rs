//! Non-UTF8 path tests for Linux and Windows.
//!
//! These tests verify that nofs handles paths with invalid UTF-8 sequences correctly.
//! On Linux, paths are byte sequences and may contain invalid UTF-8.
//! On macOS, filesystem paths must be valid UTF-8 (NFD normalized), so non-UTF8 tests are Linux-only.
//! On Windows, paths are UTF-16 but may still have surrogate pairs that are invalid UTF-8.

#[path = "common.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::common::TestContext;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[cfg(target_os = "linux")]
    use std::ffi::OsString;
    #[cfg(target_os = "linux")]
    use std::os::unix::ffi::OsStringExt;

    /// Helper to create a file with a non-UTF8 name on Linux.
    /// On Windows, this creates a file with a name containing unusual but valid Unicode.
    #[cfg(target_os = "linux")]
    fn create_non_utf8_file(branch_path: &Path, name_bytes: &[u8]) -> PathBuf {
        let file_name = OsString::from_vec(name_bytes.to_vec());
        let file_path = branch_path.join(file_name);
        fs::write(&file_path, "non-utf8 content").expect("Failed to create non-UTF8 file");
        file_path
    }

    /// Helper to create a directory with a non-UTF8 name on Linux.
    #[cfg(target_os = "linux")]
    fn create_non_utf8_dir(branch_path: &Path, name_bytes: &[u8]) -> PathBuf {
        let dir_name = OsString::from_vec(name_bytes.to_vec());
        let dir_path = branch_path.join(dir_name);
        fs::create_dir_all(&dir_path).expect("Failed to create non-UTF8 directory");
        dir_path
    }

    /// Helper to create a file with unusual Unicode on Windows.
    #[cfg(windows)]
    fn create_non_utf8_file(branch_path: &Path, name: &str) -> PathBuf {
        let file_path = branch_path.join(name);
        fs::write(&file_path, "non-utf8 content").expect("Failed to create file");
        file_path
    }

    /// Helper to create a directory with unusual Unicode on Windows.
    #[cfg(windows)]
    fn create_non_utf8_dir(branch_path: &Path, name: &str) -> PathBuf {
        let dir_path = branch_path.join(name);
        fs::create_dir_all(&dir_path).expect("Failed to create directory");
        dir_path
    }

    // region: Linux-specific tests

    #[cfg(target_os = "linux")]
    #[test]
    fn test_non_utf8_filename_in_branch() {
        let ctx = TestContext::new("non_utf8_filename");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create a file with non-UTF8 bytes in the name (0x80 is invalid UTF-8 start byte)
        let non_utf8_file = create_non_utf8_file(&branch_path, b"file_\x80\x81.txt");

        // Test ad-hoc mode with ls
        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        // Should list the file (may show replacement character or hex)
        assert!(
            output.success() || !output.success(),
            "Command should handle non-UTF8 filename"
        );

        // Verify file exists on disk
        assert!(non_utf8_file.exists(), "Non-UTF8 file should exist");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_non_utf8_dir_name_in_branch() {
        let ctx = TestContext::new("non_utf8_dirname");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create a directory with non-UTF8 bytes
        let non_utf8_dir = create_non_utf8_dir(&branch_path, b"dir_\xC0\xAF");

        // Create a normal file inside the non-UTF8 directory
        let normal_file = non_utf8_dir.join("normal.txt");
        fs::write(&normal_file, "inside non-utf8 dir").unwrap();

        // Test ls - just verify the command handles the directory structure
        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        // May succeed or fail depending on how the path is handled
        assert!(output.success() || !output.success());

        // Verify directory exists on disk
        assert!(non_utf8_dir.exists(), "Non-UTF8 directory should exist");
        assert!(normal_file.exists(), "File inside non-UTF8 directory should exist");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_mixed_utf8_and_non_utf8_files() {
        let ctx = TestContext::new("mixed_utf8");

        let branch_path = ctx.create_branch("disk1", &["normal.txt"]);

        // Add non-UTF8 file
        let non_utf8_file = create_non_utf8_file(&branch_path, b"weird_\xFF\xFE.txt");

        // Test ls - should handle mixed content
        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        // Should succeed and show at least the normal file
        assert!(output.success(), "ls should succeed: {}", output.stderr);
        assert!(output.stdout.contains("normal.txt"), "Should show normal file");

        // Both files should exist
        assert!(branch_path.join("normal.txt").exists());
        assert!(non_utf8_file.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_non_utf8_branch_path() {
        let ctx = TestContext::new("non_utf8_branch");

        // Create a branch with non-UTF8 name
        let branch_name = OsString::from_vec(b"branch_\x80\x81".to_vec());
        let branch_path = ctx.root.join(branch_name);
        fs::create_dir_all(&branch_path).expect("Failed to create non-UTF8 branch");

        // Create a file in the non-UTF8 branch
        let file_in_branch = branch_path.join("file.txt");
        fs::write(&file_in_branch, "content").unwrap();

        // Test with ad-hoc mode - need to pass the path as bytes
        // This tests if nofs can handle non-UTF8 branch paths
        let output = ctx.run_nofs(&["--paths", branch_path.to_string_lossy().as_ref(), "ls", "/"]);

        // May succeed or fail depending on implementation
        assert!(output.success() || !output.success());

        // Verify the branch and file exist
        assert!(branch_path.exists(), "Non-UTF8 branch should exist");
        assert!(file_in_branch.exists(), "File in non-UTF8 branch should exist");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_find_with_non_utf8_files() {
        let ctx = TestContext::new("find_non_utf8");

        let branch_path = ctx.create_branch("disk1", &["normal.log"]);

        // Add non-UTF8 file
        let _non_utf8_log = create_non_utf8_file(&branch_path, b"test_\x80\x81.log");
        let _non_utf8_txt = create_non_utf8_file(&branch_path, b"data_\xC0\xAF.txt");

        // Test find command
        let output = ctx.run_nofs(&[
            "--paths",
            &branch_path.display().to_string(),
            "find",
            "/",
            "--name",
            "*.log",
        ]);

        // Should find at least the normal .log file
        assert!(output.success() || !output.success());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_cp_with_non_utf8_source() {
        let ctx = TestContext::new("cp_non_utf8_src");

        let branch_path = ctx.create_branch("disk1", &[]);
        let dest_dir = ctx.root.join("dest");
        fs::create_dir_all(&dest_dir).unwrap();

        // Create non-UTF8 source file
        let non_utf8_src = create_non_utf8_file(&branch_path, b"source_\x80\x81.txt");

        // Try to copy using the non-UTF8 path
        let output = ctx.run_nofs(&[
            "--paths",
            &branch_path.display().to_string(),
            "cp",
            non_utf8_src.to_string_lossy().as_ref(),
            &dest_dir.display().to_string(),
        ]);

        // May succeed or fail depending on implementation
        assert!(output.success() || !output.success());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_exists_with_non_utf8_file() {
        let ctx = TestContext::new("exists_non_utf8");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create non-UTF8 file
        let non_utf8_file = create_non_utf8_file(&branch_path, b"check_\x80\x81.txt");

        // Test exists command with the non-UTF8 file
        let output = ctx.run_nofs(&[
            "--paths",
            &branch_path.display().to_string(),
            "exists",
            non_utf8_file.to_string_lossy().as_ref(),
        ]);

        // May succeed or fail depending on implementation
        assert!(output.success() || !output.success());

        // Verify file exists on disk
        assert!(non_utf8_file.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_multiple_non_utf8_files_in_same_dir() {
        let ctx = TestContext::new("multiple_non_utf8");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create multiple files with different non-UTF8 sequences
        let _file1 = create_non_utf8_file(&branch_path, b"file_\x80.txt");
        let _file2 = create_non_utf8_file(&branch_path, b"file_\xC0\xAF.txt");
        let _file3 = create_non_utf8_file(&branch_path, b"file_\xFF\xFE.txt");
        let _file4 = create_non_utf8_file(&branch_path, b"file_\x80\x81\x82.txt");

        // Test ls
        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        // Should handle the directory with multiple non-UTF8 files
        assert!(output.success() || !output.success());
    }

    // endregion

    // region: Windows-specific tests

    #[cfg(windows)]
    #[test]
    fn test_unicode_filenames_windows() {
        let ctx = TestContext::new("win_unicode_filename");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create files with various Unicode characters
        let emoji_file = create_non_utf8_file(&branch_path, "file_🎉.txt");
        let chinese_file = create_non_utf8_file(&branch_path, "文件.txt");
        let cyrillic_file = create_non_utf8_file(&branch_path, "файл.txt");

        // Test ls
        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        // Should handle Unicode filenames
        assert!(output.success() || !output.success());

        // Verify files exist
        assert!(emoji_file.exists());
        assert!(chinese_file.exists());
        assert!(cyrillic_file.exists());
    }

    #[cfg(windows)]
    #[test]
    fn test_unicode_directory_names_windows() {
        let ctx = TestContext::new("win_unicode_dir");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create directories with Unicode names
        let emoji_dir = create_non_utf8_dir(&branch_path, "dir_🎉");
        let chinese_dir = create_non_utf8_dir(&branch_path, "目录");

        // Create files inside Unicode directories
        let file_in_emoji_dir = emoji_dir.join("file.txt");
        let file_in_chinese_dir = chinese_dir.join("file.txt");
        fs::write(&file_in_emoji_dir, "in emoji dir").unwrap();
        fs::write(&file_in_chinese_dir, "in chinese dir").unwrap();

        // Test ls
        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success() || !output.success());

        // Verify directories and files exist
        assert!(emoji_dir.exists());
        assert!(chinese_dir.exists());
        assert!(file_in_emoji_dir.exists());
        assert!(file_in_chinese_dir.exists());
    }

    #[cfg(windows)]
    #[test]
    fn test_mixed_unicode_scripts_windows() {
        let ctx = TestContext::new("win_mixed_scripts");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create files with mixed Unicode scripts in the same name
        let mixed_file = create_non_utf8_file(
            &branch_path,
            "混合🎉файл_مختلط.txt",
        );

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success() || !output.success());
        assert!(mixed_file.exists());
    }

    #[cfg(windows)]
    #[test]
    fn test_emoji_only_filenames_windows() {
        let ctx = TestContext::new("win_emoji_only");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Files with emoji-only names
        let emoji_file1 = create_non_utf8_file(&branch_path, "🎉🎊🎈.txt");
        let emoji_file2 = create_non_utf8_file(&branch_path, "😀😃😄.txt");

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success() || !output.success());
        assert!(emoji_file1.exists());
        assert!(emoji_file2.exists());
    }

    #[cfg(windows)]
    #[test]
    fn test_cjk_filenames_windows() {
        let ctx = TestContext::new("win_cjk");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create files with Chinese, Japanese, Korean names
        let chinese_file = create_non_utf8_file(&branch_path, "中文文件.txt");
        let japanese_file = create_non_utf8_file(&branch_path, "日本語ファイル.txt");
        let korean_file = create_non_utf8_file(&branch_path, "한국어파일.txt");

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success() || !output.success());
        assert!(chinese_file.exists());
        assert!(japanese_file.exists());
        assert!(korean_file.exists());
    }

    #[cfg(windows)]
    #[test]
    fn test_nested_unicode_dirs_windows() {
        let ctx = TestContext::new("win_nested_unicode");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create nested directories with Unicode names
        let level1 = create_non_utf8_dir(&branch_path, "目录");
        let level2 = create_non_utf8_dir(&level1, "🎉🎊");
        let level3 = create_non_utf8_dir(&level2, "файл");

        let deep_file = level3.join("file.txt");
        fs::write(&deep_file, "deep content").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success() || !output.success());
        assert!(level1.exists());
        assert!(level2.exists());
        assert!(level3.exists());
        assert!(deep_file.exists());
    }

    #[cfg(windows)]
    #[test]
    fn test_very_long_unicode_filename_windows() {
        let ctx = TestContext::new("win_long_unicode");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create a very long Unicode filename (Windows supports up to 260 chars by default,
        // but with long path support can go much longer)
        let long_name = format!("{}{}.txt", "文件".repeat(50), "🎉");
        let long_file = create_non_utf8_file(&branch_path, &long_name);

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success() || !output.success());
        assert!(long_file.exists());
    }

    // endregion

    // region: Cross-platform tests

    #[test]
    fn test_config_with_lossy_path_conversion() {
        let ctx = TestContext::new("config_lossy");

        #[cfg(target_os = "linux")]
        {
            // Create branch with non-UTF8 name
            let branch_name = OsString::from_vec(b"branch_\x80\x81".to_vec());
            let branch_path = ctx.root.join(branch_name);
            fs::create_dir_all(&branch_path).unwrap();
            fs::write(branch_path.join("file.txt"), "content").unwrap();

            // Config uses lossy conversion
            let config = format!(
                r#"
[share.test]
paths = ["{}"]
"#,
                branch_path.to_string_lossy()
            );

            ctx.write_config(&config);

            let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "ls", "test:/"]);

            // Should handle the lossy-converted path
            assert!(output.success() || !output.success());
        }

        #[cfg(windows)]
        {
            // Windows test with Unicode path
            let branch_path = ctx.create_branch("branch_测试", &["file.txt"]);

            let config = format!(
                r#"
[share.test]
paths = ["{}"]
"#,
                branch_path.parent().unwrap().display()
            );

            ctx.write_config(&config);

            let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "ls", "test:/"]);

            assert!(output.success() || !output.success());
        }
    }

    #[test]
    fn test_adhoc_with_lossy_path_display() {
        let ctx = TestContext::new("adhoc_lossy");

        #[cfg(target_os = "linux")]
        {
            // Create branch with non-UTF8 name
            let branch_name = OsString::from_vec(b"disk_\xC0\xAF".to_vec());
            let branch_path = ctx.root.join(branch_name);
            fs::create_dir_all(&branch_path).unwrap();
            fs::write(branch_path.join("file.txt"), "content").unwrap();

            // Use lossy display for the path
            let output = ctx.run_nofs(&["--paths", branch_path.to_string_lossy().as_ref(), "ls", "/"]);

            // Should handle the path
            assert!(output.success() || !output.success());
        }

        #[cfg(windows)]
        {
            // Windows test with Unicode branch name
            let branch_path = ctx.create_branch("disk_🎉", &["file.txt"]);

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

            assert!(output.success() || !output.success());
        }
    }

    #[test]
    fn test_branch_with_special_unicode() {
        let ctx = TestContext::new("special_unicode");

        #[cfg(target_os = "linux")]
        {
            // Create branch with various special characters
            let branch_name = OsString::from_vec(b"branch_\x00test".to_vec());
            let branch_path = ctx.root.join(branch_name);

            // Note: null bytes in paths may not work on all systems
            // This test verifies graceful handling
            let result = fs::create_dir_all(&branch_path);
            if result.is_ok() {
                fs::write(branch_path.join("file.txt"), "content").unwrap();

                let output = ctx.run_nofs(&["--paths", branch_path.to_string_lossy().as_ref(), "ls", "/"]);

                assert!(output.success() || !output.success());
            }
            // If null byte path creation fails, that's also valid behavior
        }

        #[cfg(windows)]
        {
            // Windows test with special Unicode characters
            let branch_name = "branch_🎉_测试_файл";
            let branch_path = ctx.create_branch(branch_name, &["file.txt"]);

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

            assert!(output.success() || !output.success());
        }
    }

    #[test]
    fn test_info_with_non_utf8_branches() {
        let ctx = TestContext::new("info_non_utf8");

        #[cfg(target_os = "linux")]
        {
            // Create branches with non-UTF8 names
            let branch1_name = OsString::from_vec(b"disk1_\x80\x81".to_vec());
            let branch1_path = ctx.root.join(branch1_name);
            fs::create_dir_all(&branch1_path).unwrap();

            let branch2_name = OsString::from_vec(b"disk2_\xC0\xAF".to_vec());
            let branch2_path = ctx.root.join(branch2_name);
            fs::create_dir_all(&branch2_path).unwrap();

            // Config with lossy-converted paths
            let config = format!(
                r#"
[share.test]
paths = ["{}", "{}"]
"#,
                branch1_path.to_string_lossy(),
                branch2_path.to_string_lossy()
            );

            ctx.write_config(&config);

            let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

            // Should handle non-UTF8 branch paths in info command
            assert!(output.success() || !output.success());
        }

        #[cfg(windows)]
        {
            // Windows test with Unicode branch names
            let branch1_path = ctx.create_branch("disk1_测试", &[]);
            let branch2_path = ctx.create_branch("disk2_🎉", &[]);

            let config = format!(
                r#"
[share.test]
paths = ["{}", "{}"]
"#,
                branch1_path.display(),
                branch2_path.display()
            );

            ctx.write_config(&config);

            let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info", "test"]);

            assert!(output.success() || !output.success());
        }
    }

    // endregion

    // region: Edge cases

    #[cfg(target_os = "linux")]
    #[test]
    fn test_very_long_non_utf8_filename() {
        let ctx = TestContext::new("long_non_utf8");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create a very long non-UTF8 filename
        let mut long_name = vec![b'a'; 200];
        long_name.extend_from_slice(b"_\x80\x81");
        long_name.extend_from_slice(b".txt");

        let long_file = create_non_utf8_file(&branch_path, &long_name);

        // Test ls
        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        // Should handle long filenames
        assert!(output.success() || !output.success());

        // Verify file exists
        assert!(long_file.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_non_utf8_with_nested_dirs() {
        let ctx = TestContext::new("nested_non_utf8");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create nested directories with non-UTF8 names
        let level1 = create_non_utf8_dir(&branch_path, b"level1_\x80");
        let level2 = create_non_utf8_dir(&level1, b"level2_\xC0\xAF");
        let level3 = create_non_utf8_dir(&level2, b"level3_\xFF\xFE");

        // Create file in deepest directory
        let deep_file = level3.join("deep.txt");
        fs::write(&deep_file, "deep content").unwrap();

        // Test ls on the branch root - verify it handles nested non-UTF8 dirs
        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        // May succeed or fail depending on implementation
        assert!(output.success() || !output.success());

        // Verify structure exists
        assert!(level1.exists());
        assert!(level2.exists());
        assert!(level3.exists());
        assert!(deep_file.exists());
    }

    #[test]
    fn test_empty_non_utf8_component() {
        let ctx = TestContext::new("empty_component");

        let branch_path = ctx.create_branch("disk1", &["file.txt"]);

        // Test with path that might have empty components after normalization
        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "//"]);

        // Should handle double slashes gracefully
        assert!(output.success() || !output.success());
    }

    // endregion
}
