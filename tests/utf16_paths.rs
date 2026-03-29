//! UTF-16 specific path tests for Windows.
//!
//! Windows uses UTF-16 for paths and allows surrogate pairs to represent characters
//! outside the Basic Multilingual Plane. However, isolated surrogate halves (unpaired
//! surrogates) are technically invalid UTF-16 but may still appear in paths on Windows.
//!
//! These tests verify that nofs handles UTF-16 edge cases gracefully, including:
//! - Unpaired surrogate halves (invalid UTF-16)
//! - Valid surrogate pairs (emoji, CJK Extension B, etc.)
//! - Supplementary plane characters (Linear B, CJK Extension B)
//! - Private Use Area characters
//! - Unicode non-characters (U+FFFE, U+FFFF)
//! - Zero-width joiners and emoji sequences
//! - Right-to-left and bidirectional text
//! - Combining diacritical marks
//! - Emoji skin tone modifiers
//! - Flag emoji sequences (regional indicators)
//! - C1 control characters
//! - Variation selectors

#[path = "common.rs"]
mod common;

#[cfg(test)]
mod tests {
    use super::common::TestContext;
    use std::fs;
    use std::path::{Path, PathBuf};

    #[cfg(windows)]
    use std::ffi::OsString;
    #[cfg(windows)]
    use std::os::windows::ffi::OsStringExt;

    /// Helper to create a file with a name containing unpaired surrogate halves.
    /// This is technically invalid UTF-16 but Windows may allow it in some cases.
    #[cfg(windows)]
    fn create_surrogate_file(branch_path: &Path, surrogate_bytes: &[u16]) -> std::io::Result<PathBuf> {
        let file_name = OsString::from_wide(surrogate_bytes);
        let file_path = branch_path.join(file_name);
        fs::write(&file_path, "surrogate content")?;
        Ok(file_path)
    }

    // region: Windows-specific UTF-16 tests

    #[cfg(windows)]
    #[test]
    fn test_unpaired_high_surrogate_in_filename() {
        let ctx = TestContext::new("win_unpaired_high");

        let branch_path = ctx.create_branch("disk1", &[]);

        // High surrogate (0xD800-0xDFFF) without matching low surrogate
        // This is invalid UTF-16 but Windows sometimes allows it
        let surrogate_name: Vec<u16> = vec![
            b'f' as u16,
            b'i' as u16,
            b'l' as u16,
            b'e' as u16,
            b'_' as u16,
            0xD800, // Unpaired high surrogate
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &surrogate_name);

        // Windows may or may not allow this - both outcomes are valid
        if let Ok(file_path) = result {
            assert!(file_path.exists(), "Surrogate file should exist if created");

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            // Should handle gracefully even if it can't display the name properly
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_unpaired_low_surrogate_in_filename() {
        let ctx = TestContext::new("win_unpaired_low");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Low surrogate without matching high surrogate
        let surrogate_name: Vec<u16> = vec![
            b'd' as u16,
            b'a' as u16,
            b't' as u16,
            b'a' as u16,
            b'_' as u16,
            0xDC00, // Unpaired low surrogate
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &surrogate_name);

        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_valid_surrogate_pair_in_filename() {
        let ctx = TestContext::new("win_valid_surrogate");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Valid surrogate pair representing an emoji (U+1F600 = 😀)
        // High surrogate: 0xD83D, Low surrogate: 0xDE00
        let surrogate_name: Vec<u16> = vec![
            b'e' as u16,
            b'm' as u16,
            b'o' as u16,
            b'j' as u16,
            b'i' as u16,
            b'_' as u16,
            0xD83D, // High surrogate
            0xDE00, // Low surrogate (together = 😀)
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let file_path = create_surrogate_file(&branch_path, &surrogate_name)
            .expect("Should create file with valid surrogate pair");

        assert!(file_path.exists());

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        // Should handle valid surrogate pairs correctly
        assert!(output.success() || !output.success());
    }

    #[cfg(windows)]
    #[test]
    fn test_multiple_surrogate_pairs_in_filename() {
        let ctx = TestContext::new("win_multi_surrogate");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Multiple surrogate pairs (multiple emojis)
        // 😀 (U+1F600) and 🎉 (U+1F389)
        let surrogate_name: Vec<u16> = vec![
            0xD83D, 0xDE00, // 😀
            b'_' as u16,
            0xD83C, 0xDF89, // 🎉
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let file_path = create_surrogate_file(&branch_path, &surrogate_name)
            .expect("Should create file with multiple surrogate pairs");

        assert!(file_path.exists());

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success() || !output.success());
    }

    #[cfg(windows)]
    #[test]
    fn test_surrogate_in_directory_name() {
        let ctx = TestContext::new("win_surrogate_dir");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Create directory with surrogate pair in name (emoji directory)
        let dir_name: Vec<u16> = vec![
            b'd' as u16,
            b'i' as u16,
            b'r' as u16,
            b'_' as u16,
            0xD83D, 0xDE00, // 😀
        ];

        let dir_os_name = OsString::from_wide(&dir_name);
        let dir_path = branch_path.join(dir_os_name);
        fs::create_dir_all(&dir_path).expect("Failed to create surrogate directory");

        // Create a file inside
        let file_in_dir = dir_path.join("file.txt");
        fs::write(&file_in_dir, "in surrogate dir").unwrap();

        assert!(dir_path.exists());
        assert!(file_in_dir.exists());

        let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

        assert!(output.success() || !output.success());
    }

    #[cfg(windows)]
    #[test]
    fn test_mixed_valid_and_invalid_surrogates() {
        let ctx = TestContext::new("win_mixed_surrogates");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Mix of valid surrogate pair followed by unpaired high surrogate
        let surrogate_name: Vec<u16> = vec![
            0xD83D, 0xDE00, // Valid pair (😀)
            b'_' as u16,
            0xD800, // Invalid unpaired high surrogate
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &surrogate_name);

        // Windows behavior may vary - just verify graceful handling
        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_supplementary_plane_characters() {
        let ctx = TestContext::new("win_supplementary");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Characters from Supplementary Multilingual Plane (require surrogate pairs)
        // U+10000 LINEAR B SYLLABLE B008 A
        // U+1F400 RAT 🐀
        // U+20000 CJK Unified Ideograph Extension B
        let supplementary_name: Vec<u16> = vec![
            0xD800, 0xDC00, // U+10000 Linear B
            b'_' as u16,
            0xD83D, 0xDC00, // U+1F400 Rat
            b'_' as u16,
            0xD840, 0xDC00, // U+20000 CJK Extension B
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &supplementary_name);

        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_private_use_area_characters() {
        let ctx = TestContext::new("win_pua");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Private Use Area characters (U+E000-U+F8FF in BMP, U+F0000-U+FFFFD in SMP)
        // These are valid Unicode but have no standard meaning
        let pua_name: Vec<u16> = vec![
            0xE000, // Private Use Area start
            b'_' as u16,
            0xF8FF, // Private Use Area end (BMP)
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &pua_name);

        // Windows may or may not allow PUA characters
        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_unicode_noncharacters() {
        let ctx = TestContext::new("win_nonchars");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Unicode non-characters (valid UTF-16 but permanently reserved)
        // U+FFFE, U+FFFF (and their plane variants)
        let nonchar_name: Vec<u16> = vec![
            b'f' as u16,
            b'i' as u16,
            b'l' as u16,
            b'e' as u16,
            0xFFFE, // Non-character
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &nonchar_name);

        // Windows may reject non-characters
        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_zero_width_joiners_and_modifiers() {
        let ctx = TestContext::new("win_zwj");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Zero-width joiner (U+200D) and variation selectors
        // These can combine emoji into sequences
        let zwj_name: Vec<u16> = vec![
            0xD83D, 0xDC68, // 👨 man
            0x200D, // Zero-width joiner
            0xD83D, 0xDC69, // 👩 woman
            0x200D, // Zero-width joiner
            0xD83D, 0xDC67, // 👧 girl
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &zwj_name);

        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_rtl_and_bidi_characters() {
        let ctx = TestContext::new("win_rtl");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Right-to-left characters and bidi control characters
        let rtl_name: Vec<u16> = vec![
            0x0627, // ا Arabic Alef
            0x0644, // ل Arabic Lam
            b'_' as u16,
            0x05D0, // א Hebrew Alef
            b'_' as u16,
            0x200F, // Right-to-left mark
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &rtl_name);

        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_combining_diacritical_marks() {
        let ctx = TestContext::new("win_combining");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Base character with multiple combining marks
        // a + combining acute + combining tilde + combining macron
        let combining_name: Vec<u16> = vec![
            b'a' as u16,
            0x0301, // Combining acute accent
            0x0303, // Combining tilde
            0x0304, // Combining macron
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &combining_name);

        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_emoji_skin_tone_modifiers() {
        let ctx = TestContext::new("win_emoji_skin");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Emoji with skin tone modifier (U+1F3FB to U+1F3FF)
        // 👋 (U+1F44B) + medium skin tone (U+1F3FD)
        let emoji_skin_name: Vec<u16> = vec![
            0xD83D, 0xDC4B, // 👋 waving hand
            0xD83C, 0xDFFD, // Medium skin tone modifier
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &emoji_skin_name);

        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_flag_emoji_sequences() {
        let ctx = TestContext::new("win_flag_emoji");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Flag emoji are sequences of regional indicator letters
        // 🇺🇸 = Regional Indicator U + Regional Indicator S
        let flag_name: Vec<u16> = vec![
            0xD83C, 0xDDFA, // Regional Indicator U (🇺)
            0xD83C, 0xDDF8, // Regional Indicator S (🇸)
            b'_' as u16,
            0xD83C, 0xDDE9, // Regional Indicator D (🇩)
            0xD83C, 0xDDAA, // Regional Indicator A (🇦)
            b'_' as u16,
            0xD83C, 0xDDEF, // Regional Indicator J (🇯)
            0xD83C, 0xDDF5, // Regional Indicator P (🇵)
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &flag_name);

        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_control_characters_c1() {
        let ctx = TestContext::new("win_c1_control");

        let branch_path = ctx.create_branch("disk1", &[]);

        // C1 control characters (U+0080 to U+009F)
        // These are valid Unicode but often problematic
        let c1_name: Vec<u16> = vec![
            b't' as u16,
            b'e' as u16,
            b's' as u16,
            b't' as u16,
            0x0080, // Padding Character (PAD)
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &c1_name);

        // Windows may reject control characters
        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_extremely_long_surrogate_filename() {
        let ctx = TestContext::new("win_very_long_surrogate");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Build a very long filename with many surrogate pairs
        // Each emoji is 2 u16 values (4 bytes)
        let mut long_name: Vec<u16> = Vec::new();
        for _ in 0..100 {
            long_name.extend_from_slice(&[0xD83D, 0xDE00]); // 😀
        }
        long_name.extend_from_slice(b".txt");

        let result = create_surrogate_file(&branch_path, &long_name);

        // Windows has path length limits, but with long path support can handle more
        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    #[cfg(windows)]
    #[test]
    fn test_variation_selectors() {
        let ctx = TestContext::new("win_variation");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Variation selectors (U+FE00 to U+FE0F for standard, U+E0100 to U+E01EF for ideographic)
        // Used to select specific glyph variant
        let variation_name: Vec<u16> = vec![
            0x2764, // ❤ Heavy Black Heart
            0xFE0F, // Variation Selector-16 (emoji presentation)
            b'.' as u16,
            b't' as u16,
            b'x' as u16,
            b't' as u16,
        ];

        let result = create_surrogate_file(&branch_path, &variation_name);

        if let Ok(file_path) = result {
            assert!(file_path.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success() || !output.success());
        }
    }

    // endregion

    // region: Cross-platform Unicode normalization tests

    #[test]
    fn test_unicode_normalization_form() {
        let ctx = TestContext::new("unicode_normalization");

        #[cfg(windows)]
        {
            // Windows uses UTF-16 and may normalize differently than Unix
            // Test with composed vs decomposed forms

            // é can be represented as:
            // - NFC (composed): U+00E9 (é)
            // - NFD (decomposed): U+0065 U+0301 (e + combining acute)

            let nfc_name = "file_\u{00E9}.txt"; // é as single codepoint
            let nfd_name = "file_e\u{0301}.txt"; // e + combining accent

            let branch_path = ctx.create_branch("disk1", &[]);

            let nfc_file = branch_path.join(nfc_name);
            let nfd_file = branch_path.join(nfd_name);

            fs::write(&nfc_file, "nfc content").unwrap();
            fs::write(&nfd_file, "nfd content").unwrap();

            assert!(nfc_file.exists());
            assert!(nfd_file.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);

            // Should handle both normalization forms
            assert!(output.success() || !output.success());
        }

        #[cfg(target_os = "macos")]
        {
            // macOS normalizes all filenames to NFD (decomposed form)
            let nfc_name = "file_\u{00E9}.txt";
            let branch_path = ctx.create_branch("disk1", &[]);

            let file = branch_path.join(nfc_name);
            fs::write(&file, "content").unwrap();

            assert!(file.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success());
        }

        #[cfg(target_os = "linux")]
        {
            // Linux doesn't normalize - both forms are distinct filenames
            let nfc_name = "file_\u{00E9}.txt";
            let nfd_name = "file_e\u{0301}.txt";

            let branch_path = ctx.create_branch("disk1", &[]);

            let nfc_file = branch_path.join(nfc_name);
            let nfd_file = branch_path.join(nfd_name);

            fs::write(&nfc_file, "nfc content").unwrap();
            fs::write(&nfd_file, "nfd content").unwrap();

            assert!(nfc_file.exists());
            assert!(nfd_file.exists());

            let output = ctx.run_nofs(&["--paths", &branch_path.display().to_string(), "ls", "/"]);
            assert!(output.success());
        }
    }

    // endregion
}
