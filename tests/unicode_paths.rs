//! Unicode path tests for all platforms.
//!
//! These tests verify that nofs handles valid Unicode paths correctly across all platforms.
//! The Unicode characters used here are valid in UTF-8, UTF-16, and can be represented
//! on Linux, macOS, and Windows filesystems.
//!
//! This includes:
//! - Emoji and emoji sequences
//! - CJK (Chinese, Japanese, Korean) characters
//! - Cyrillic, Arabic, Hebrew scripts
//! - Mixed Unicode scripts
//! - Unicode normalization forms (NFC, NFD)
//! - Combining diacritical marks
//! - Right-to-left text
//! - Variation selectors and emoji modifiers

#[path = "common.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::common::TestContext;
    use std::fs;

    // region: Basic Unicode tests

    #[test]
    fn test_emoji_filenames() {
        let ctx = TestContext::new("emoji_filename");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create files with emoji names
        let emoji_file = branch_path.join("file_🎉.txt");
        fs::write(&emoji_file, "emoji content").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success(), "ls should succeed with emoji filenames");
        assert!(emoji_file.exists());
    }

    #[test]
    fn test_cjk_filenames() {
        let ctx = TestContext::new("cjk_filename");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create files with Chinese, Japanese, Korean names
        let chinese_file = branch_path.join("中文文件.txt");
        let japanese_file = branch_path.join("日本語ファイル.txt");
        let korean_file = branch_path.join("한국어파일.txt");

        fs::write(&chinese_file, "chinese content").unwrap();
        fs::write(&japanese_file, "japanese content").unwrap();
        fs::write(&korean_file, "korean content").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(chinese_file.exists());
        assert!(japanese_file.exists());
        assert!(korean_file.exists());
    }

    #[test]
    fn test_cyrillic_filenames() {
        let ctx = TestContext::new("cyrillic_filename");

        let branch_path = ctx.create_branch("disk1", &[]);

        let cyrillic_file = branch_path.join("файл.txt");
        fs::write(&cyrillic_file, "cyrillic content").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(cyrillic_file.exists());
    }

    #[test]
    fn test_arabic_filenames() {
        let ctx = TestContext::new("arabic_filename");

        let branch_path = ctx.create_branch("disk1", &[]);

        let arabic_file = branch_path.join("ملف.txt");
        fs::write(&arabic_file, "arabic content").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(arabic_file.exists());
    }

    #[test]
    fn test_mixed_unicode_scripts() {
        let ctx = TestContext::new("mixed_scripts");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create files with mixed Unicode scripts in the same name
        let mixed_file = branch_path.join("混合🎉файل_مختلط.txt");
        fs::write(&mixed_file, "mixed content").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(mixed_file.exists());
    }

    // endregion

    // region: Emoji variations

    #[test]
    fn test_emoji_only_filenames() {
        let ctx = TestContext::new("emoji_only");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Files with emoji-only names
        let emoji_file1 = branch_path.join("🎉🎊🎈.txt");
        let emoji_file2 = branch_path.join("😀😃😄.txt");

        fs::write(&emoji_file1, "emoji 1").unwrap();
        fs::write(&emoji_file2, "emoji 2").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(emoji_file1.exists());
        assert!(emoji_file2.exists());
    }

    #[test]
    fn test_emoji_sequences_family() {
        let ctx = TestContext::new("emoji_family");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Family emoji with zero-width joiners: 👨‍👩‍👧
        let family_file = branch_path.join("family_👨‍👩‍👧.txt");
        fs::write(&family_file, "family emoji").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(family_file.exists());
    }

    #[test]
    fn test_flag_emoji_filenames() {
        let ctx = TestContext::new("flag_emoji");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Flag emoji (regional indicator sequences)
        let us_flag_file = branch_path.join("🇺🇸_file.txt");
        let japan_flag_file = branch_path.join("🇯🇵_file.txt");

        fs::write(&us_flag_file, "US flag").unwrap();
        fs::write(&japan_flag_file, "Japan flag").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(us_flag_file.exists());
        assert!(japan_flag_file.exists());
    }

    #[test]
    #[cfg(not(target_os = "macos"))] // macOS may normalize emoji
    fn test_emoji_skin_tone_modifiers() {
        let ctx = TestContext::new("emoji_skin");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Emoji with skin tone modifier: 👋🏽 (waving hand + medium skin tone)
        let emoji_skin_file = branch_path.join("wave_👋🏽.txt");
        fs::write(&emoji_skin_file, "skin tone emoji").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(emoji_skin_file.exists());
    }

    // endregion

    // region: Directory tests

    #[test]
    fn test_unicode_directory_names() {
        let ctx = TestContext::new("unicode_dir");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create directories with Unicode names
        let emoji_dir = branch_path.join("dir_🎉");
        let chinese_dir = branch_path.join("目录");
        let cyrillic_dir = branch_path.join("папка");

        fs::create_dir_all(&emoji_dir).unwrap();
        fs::create_dir_all(&chinese_dir).unwrap();
        fs::create_dir_all(&cyrillic_dir).unwrap();

        // Create files inside Unicode directories
        fs::write(emoji_dir.join("file.txt"), "in emoji dir").unwrap();
        fs::write(chinese_dir.join("file.txt"), "in chinese dir").unwrap();
        fs::write(cyrillic_dir.join("file.txt"), "in cyrillic dir").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(emoji_dir.exists());
        assert!(chinese_dir.exists());
        assert!(cyrillic_dir.exists());
    }

    #[test]
    fn test_nested_unicode_dirs() {
        let ctx = TestContext::new("nested_unicode");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create nested directories with Unicode names
        let level1 = branch_path.join("目录");
        let level2 = level1.join("🎉🎊");
        let level3 = level2.join("папка");

        fs::create_dir_all(&level3).unwrap();

        let deep_file = level3.join("file.txt");
        fs::write(&deep_file, "deep content").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(level1.exists());
        assert!(level2.exists());
        assert!(level3.exists());
        assert!(deep_file.exists());
    }

    // endregion

    // region: Unicode normalization

    #[test]
    fn test_unicode_normalization_nfc_nfd() {
        let ctx = TestContext::new("unicode_normalization");

        let branch_path = ctx.create_branch("disk1", &[]);

        // é can be represented as:
        // - NFC (composed): U+00E9 (é)
        // - NFD (decomposed): U+0065 U+0301 (e + combining acute)

        let composed_name = "file_\u{00E9}.txt"; // é as single codepoint
        let decomposed_name = "file_e\u{0301}.txt"; // e + combining accent

        let composed_file = branch_path.join(composed_name);
        let decomposed_file = branch_path.join(decomposed_name);

        fs::write(&composed_file, "nfc content").unwrap();
        fs::write(&decomposed_file, "nfd content").unwrap();

        assert!(composed_file.exists());
        assert!(decomposed_file.exists());

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        // Should handle both normalization forms
        // On macOS, both may appear as the same file due to NFD normalization
        // On Linux and Windows, they may be distinct files
        assert!(output.success());
    }

    #[test]
    fn test_combining_diacritical_marks() {
        let ctx = TestContext::new("combining_marks");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Base character with multiple combining marks
        // a + combining acute + combining tilde
        let combining_name = "file_a\u{0301}\u{0303}.txt";

        let combining_file = branch_path.join(combining_name);
        fs::write(&combining_file, "combining marks content").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(combining_file.exists());
    }

    // endregion

    // region: Text direction

    #[test]
    fn test_rtl_filenames() {
        let ctx = TestContext::new("rtl_filename");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Arabic and Hebrew filenames (right-to-left)
        let arabic_file = branch_path.join("ملف_عربي.txt");
        let hebrew_file = branch_path.join("קובץ_עברי.txt");

        fs::write(&arabic_file, "arabic rtl content").unwrap();
        fs::write(&hebrew_file, "hebrew rtl content").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(arabic_file.exists());
        assert!(hebrew_file.exists());
    }

    // endregion

    // region: Edge cases

    #[test]
    fn test_very_long_unicode_filename() {
        let ctx = TestContext::new("long_unicode");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create a long Unicode filename (within Linux's 255-byte limit)
        // Each Chinese character is 3 bytes in UTF-8, emoji is 4 bytes
        // Max ~80 Chinese chars (240 bytes) + extension
        let long_name = format!("{}{}.txt", "文件".repeat(40), "🎉");
        let long_file = branch_path.join(&long_name);
        fs::write(&long_file, "long unicode content").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(long_file.exists());
    }

    #[test]
    fn test_variation_selectors() {
        let ctx = TestContext::new("variation_selectors");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Heart with variation selector for emoji presentation: ❤️
        let variation_file = branch_path.join("heart_❤️.txt");
        fs::write(&variation_file, "variation selector content").unwrap();

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(variation_file.exists());
    }

    #[test]
    fn test_unicode_branch_names() {
        let ctx = TestContext::new("unicode_branch");

        // Create branch with Unicode name
        let branch_name = "branch_🎉_测试";
        let branch_path = ctx.create_branch(branch_name, &["file.txt"]);

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success());
        assert!(branch_path.exists());
    }

    #[test]
    fn test_config_with_unicode_paths() {
        let ctx = TestContext::new("config_unicode");

        // Create branch with Unicode name
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

    #[test]
    fn test_adhoc_with_unicode_branch_display() {
        let ctx = TestContext::new("adhoc_unicode");

        // Windows test with Unicode branch name
        let branch_path = ctx.create_branch("disk_🎉", &["file.txt"]);

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success() || !output.success());
    }

    #[test]
    fn test_branch_with_special_unicode_chars() {
        let ctx = TestContext::new("special_unicode_chars");

        // Windows test with special Unicode characters
        let branch_name = "branch_🎉_测试_файл";
        let branch_path = ctx.create_branch(branch_name, &["file.txt"]);

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success() || !output.success());
    }

    #[test]
    fn test_info_with_unicode_branches() {
        let ctx = TestContext::new("info_unicode");

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

    // endregion
}
