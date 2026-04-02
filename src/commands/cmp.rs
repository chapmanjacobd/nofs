//! cmp command - Compare files byte-by-byte
//!
//! This command compares files across branches to check if they are identical.

use crate::branch::Branch;
use crate::cache::OperationCache;
use crate::error::{NofsError, Result};
use crate::pool::Pool;
use serde::Serialize;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

/// Output from the `cmp` command
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct CmpOutput {
    pub files: Vec<String>,
    pub identical: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub difference: Option<Difference>,
}

/// Information about a difference
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct Difference {
    pub byte_offset: u64,
    pub line_number: usize,
    pub file1_value: String,
    pub file2_value: String,
}

/// Execute the cmp command
///
/// # Errors
///
/// Returns an error if there is an IO error during comparison or output.
///
/// # Panics
///
/// This function should not panic as it checks the length before accessing elements.
#[allow(clippy::fn_params_excessive_bools, clippy::unwrap_used, clippy::get_unwrap)]
pub fn execute(
    pool: &Pool,
    path: &str,
    branch1_name: Option<&str>,
    branch2_name: Option<&str>,
    verbose: bool,
    json: bool,
) -> Result<()> {
    let pool_path = Path::new(path);

    // Create operation cache for this command execution
    let cache = OperationCache::new();

    // Find all branches with this path (cached)
    let branches = pool.find_all_branches_cached(pool_path, &cache);

    if branches.is_empty() {
        return Err(NofsError::Command(format!(
            "cannot access '{path}': No such file or directory"
        )));
    }

    // Check if it's a file
    let is_file = branches
        .iter()
        .find_map(|b| {
            let full_path = b.path.join(pool_path);
            full_path.exists().then(|| full_path.is_file())
        })
        .unwrap_or(false);

    if !is_file {
        return Err(NofsError::Command(format!("'{path}' is not a regular file")));
    }

    // Get files from specified branches or first two branches
    let files_to_compare: Vec<(&Branch, std::path::PathBuf)> = branches
        .iter()
        .filter(|b| branch1_name.is_none_or(|b1| b.path.to_string_lossy().contains(b1)))
        .filter(|b| branch2_name.is_none_or(|b2| b.path.to_string_lossy().contains(b2)))
        .take(2)
        .map(|b| (*b, b.path.join(pool_path)))
        .collect();

    if files_to_compare.len() < 2 {
        return Err(NofsError::Command(format!(
            "need at least 2 files to compare, found {}",
            files_to_compare.len()
        )));
    }

    let (_branch1, path1) = files_to_compare.first().unwrap();
    let (_branch2, path2) = files_to_compare.get(1).unwrap();

    // Compare files
    let comparison = compare_files(path1, path2)?;

    if json {
        let output = CmpOutput {
            files: vec![path1.to_string_lossy().to_string(), path2.to_string_lossy().to_string()],
            identical: comparison.identical,
            difference: comparison.difference.map(|d| Difference {
                byte_offset: d.byte_offset,
                line_number: d.line_number,
                file1_value: format_byte(d.file1_byte),
                file2_value: format_byte(d.file2_byte),
            }),
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else if comparison.identical {
        if verbose {
            let stdout = io::stdout();
            let mut handle = stdout.lock();
            writeln!(handle, "{} and {} are identical", path1.display(), path2.display())?;
        }
        // Exit code 0 for identical files (success)
    } else if let Some(diff) = comparison.difference {
        let stderr = io::stderr();
        let mut handle = stderr.lock();
        writeln!(
            handle,
            "{} {} differ: byte {}, line {}",
            path1.display(),
            path2.display(),
            diff.byte_offset,
            diff.line_number
        )?;
        // Return error exit code for different files
        return Err(NofsError::Command("files differ".to_string()));
    } else {
        // This shouldn't happen, but handle it gracefully
        return Err(NofsError::Command("unexpected comparison result".to_string()));
    }

    Ok(())
}

/// Result of file comparison
#[allow(clippy::exhaustive_structs)]
struct ComparisonResult {
    /// Whether the files are identical
    identical: bool,
    /// Information about the first difference found, if any
    difference: Option<ByteDifference>,
}

/// Information about a byte difference
#[allow(clippy::exhaustive_structs)]
struct ByteDifference {
    /// Byte offset where the difference occurs
    byte_offset: u64,
    /// Line number where the difference occurs
    line_number: usize,
    /// Byte value from the first file
    file1_byte: u8,
    /// Byte value from the second file
    file2_byte: u8,
}

/// Compare two files byte-by-byte
#[allow(clippy::indexing_slicing, clippy::arithmetic_side_effects, clippy::as_conversions)]
fn compare_files(path1: &Path, path2: &Path) -> Result<ComparisonResult> {
    let mut file1 =
        fs::File::open(path1).map_err(|e| NofsError::Command(format!("cannot open '{}': {}", path1.display(), e)))?;
    let mut file2 =
        fs::File::open(path2).map_err(|e| NofsError::Command(format!("cannot open '{}': {}", path2.display(), e)))?;

    let mut buf1 = [0_u8; 4096];
    let mut buf2 = [0_u8; 4096];
    let mut total_offset: u64 = 0;
    let mut line_number: usize = 1;

    loop {
        let n1 = file1
            .read(&mut buf1)
            .map_err(|e| NofsError::Command(format!("error reading '{}': {}", path1.display(), e)))?;
        let n2 = file2
            .read(&mut buf2)
            .map_err(|e| NofsError::Command(format!("error reading '{}': {}", path2.display(), e)))?;

        if n1 == 0 && n2 == 0 {
            // Both files ended
            return Ok(ComparisonResult {
                identical: true,
                difference: None,
            });
        }

        if n1 != n2 {
            // Files have different lengths
            return Ok(ComparisonResult {
                identical: false,
                difference: Some(ByteDifference {
                    byte_offset: total_offset,
                    line_number,
                    file1_byte: if n1 > 0 { buf1[0] } else { 0 },
                    file2_byte: if n2 > 0 { buf2[0] } else { 0 },
                }),
            });
        }

        // Compare buffers
        for i in 0..n1 {
            if buf1[i] != buf2[i] {
                return Ok(ComparisonResult {
                    identical: false,
                    difference: Some(ByteDifference {
                        byte_offset: total_offset + i as u64,
                        line_number,
                        file1_byte: buf1[i],
                        file2_byte: buf2[i],
                    }),
                });
            }
            if buf1[i] == b'\n' {
                line_number += 1;
            }
        }

        total_offset += n1 as u64;
    }
}

/// Format a byte for display
#[allow(clippy::as_conversions, clippy::uninlined_format_args)]
fn format_byte(b: u8) -> String {
    if b.is_ascii_graphic() || b == b' ' {
        format!("{}", b as char)
    } else {
        format!("0x{b:02X}")
    }
}
