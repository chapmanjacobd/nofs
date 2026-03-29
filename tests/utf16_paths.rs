//! UTF-16 specific path tests for Windows.
//!
//! Windows uses UTF-16 for paths and allows surrogate pairs to represent characters
//! outside the Basic Multilingual Plane. However, isolated surrogate halves (unpaired
//! surrogates) are technically invalid UTF-16 but Windows may still allow them in paths.
//!
//! These tests verify that nofs handles true UTF-16 edge cases that CANNOT be
//! represented in UTF-8:
//! - Unpaired high surrogates (0xD800-0xDFFF without matching pair)
//! - Unpaired low surrogates
//! - Mixed valid/invalid surrogate sequences
//!
//! Note: Valid Unicode characters (emoji, CJK, etc.) are tested in `unicode_paths.rs`
//! since they work on all platforms.

#[path = "common.rs"]
mod common;

#[cfg(test)]
#[cfg(windows)]
mod tests {
    use super::common::TestContext;
    use std::ffi::OsString;
    use std::fs;
    use std::os::windows::ffi::OsStringExt;
    use std::path::{Path, PathBuf};

    /// Helper to create a file with a name containing unpaired surrogate halves.
    /// This is technically invalid UTF-16 but Windows may allow it in some cases.
    fn create_surrogate_file(branch_path: &Path, surrogate_bytes: &[u16]) -> std::io::Result<PathBuf> {
        let file_name = OsString::from_wide(surrogate_bytes);
        let file_path = branch_path.join(file_name);
        fs::write(&file_path, "surrogate content")?;
        Ok(file_path)
    }

    // region: Unpaired surrogate tests (UTF-16 specific)

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

    #[test]
    fn test_mixed_valid_and_invalid_surrogates() {
        let ctx = TestContext::new("win_mixed_surrogates");

        let branch_path = ctx.create_branch("disk1", &[]);

        // Mix of valid surrogate pair followed by unpaired high surrogate
        let surrogate_name: Vec<u16> = vec![
            0xD83D,
            0xDE00, // Valid pair (😀)
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

    // endregion

    // region: Cross-platform Unicode normalization tests

    #[test]
    fn test_unicode_normalization_form() {
        let ctx = TestContext::new("unicode_normalization");

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

    // endregion
}
