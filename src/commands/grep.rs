//! grep command - Search file contents across all branches
//!
//! This command searches for patterns in files across all branches simultaneously.

use crate::cache::OperationCache;
use crate::error::{NofsError, Result};
use crate::pool::Pool;
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

/// Output from the `grep` command
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct GrepOutput {
    pub pattern: String,
    pub path: String,
    pub matches: Vec<GrepMatch>,
}

/// A single grep match
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub struct GrepMatch {
    pub file: String,
    pub branch: String,
    pub line_number: usize,
    pub line: String,
}

/// Execute the grep command
///
/// # Errors
///
/// Returns an error if there is an IO error during search or output.
pub fn execute(
    pool: &Pool,
    path: &str,
    pattern: &str,
    ignore_case: bool,
    invert_match: bool,
    line_numbers: bool,
    files_with_matches: bool,
    recursive: bool,
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

    // Compile regex
    let re_pattern = if ignore_case {
        format!("(?i){pattern}")
    } else {
        pattern.to_string()
    };
    let re = Regex::new(&re_pattern).map_err(|e| NofsError::Command(format!("Invalid regex pattern: {e}")))?;

    // Collect all matches
    let mut all_matches: Vec<GrepMatch> = Vec::new();
    let mut files_matched: std::collections::HashSet<String> = std::collections::HashSet::new();

    // Search each branch
    for branch in &branches {
        let branch_path = branch.path.join(pool_path);

        if !branch_path.exists() {
            continue;
        }

        if branch_path.is_file() {
            // Single file search
            search_file(
                &branch_path,
                &branch.path.to_string_lossy(),
                &re,
                invert_match,
                &mut all_matches,
                &mut files_matched,
                verbose,
            )?;
        } else if recursive {
            // Directory search (recursive)
            search_directory(
                &branch_path,
                &branch.path.to_string_lossy(),
                &re,
                invert_match,
                &mut all_matches,
                &mut files_matched,
                verbose,
            )?;
        } else {
            // Directory without recursive - skip
        }
    }

    // Output results
    if json {
        let output = GrepOutput {
            pattern: pattern.to_string(),
            path: path.to_string(),
            matches: all_matches,
        };
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        output_text(&all_matches, files_with_matches, line_numbers)?;
    }

    Ok(())
}

/// Search a single file for pattern matches
fn search_file(
    file_path: &Path,
    branch_name: &str,
    re: &Regex,
    invert_match: bool,
    all_matches: &mut Vec<GrepMatch>,
    files_matched: &mut std::collections::HashSet<String>,
    verbose: bool,
) -> Result<()> {
    let file = fs::File::open(file_path).map_err(|e| {
        if verbose {
            eprintln!("nofs: warning: cannot read '{}': {}", file_path.display(), e);
        }
        NofsError::Command(format!("cannot read '{}': {}", file_path.display(), e))
    })?;

    let reader = io::BufReader::new(file);
    let file_name = file_path
        .file_name().map_or_else(|| file_path.to_string_lossy().to_string(), |n| n.to_string_lossy().to_string());

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result.unwrap_or_default();
        let matches = re.is_match(&line);

        if matches != invert_match {
            files_matched.insert(format!("{branch_name}/{file_name}"));
            all_matches.push(GrepMatch {
                file: file_name.clone(),
                branch: branch_name.to_string(),
                line_number: line_num + 1,
                line: line.clone(),
            });
        }
    }

    Ok(())
}

/// Search a directory recursively for pattern matches
fn search_directory(
    dir_path: &Path,
    branch_name: &str,
    re: &Regex,
    invert_match: bool,
    all_matches: &mut Vec<GrepMatch>,
    files_matched: &mut std::collections::HashSet<String>,
    verbose: bool,
) -> Result<()> {
    for dir_entry in fs::read_dir(dir_path).map_err(|e| {
        if verbose {
            eprintln!("nofs: warning: cannot read directory '{}': {}", dir_path.display(), e);
        }
        NofsError::Command(format!("cannot read directory '{}': {}", dir_path.display(), e))
    })? {
        let Ok(entry) = dir_entry else {
            continue;
        };

        let entry_path = entry.path();

        // Skip hidden files and directories
        if entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.starts_with('.'))
        {
            continue;
        }

        let Ok(metadata) = entry_path.metadata() else {
            continue;
        };

        if metadata.is_dir() {
            search_directory(
                &entry_path,
                branch_name,
                re,
                invert_match,
                all_matches,
                files_matched,
                verbose,
            )?;
        } else if metadata.is_file() {
            search_file(
                &entry_path,
                branch_name,
                re,
                invert_match,
                all_matches,
                files_matched,
                verbose,
            )?;
        } else {
            // Skip other file types (symlinks, etc.)
        }
    }

    Ok(())
}

/// Output results in text format
fn output_text(matches: &[GrepMatch], files_with_matches: bool, line_numbers: bool) -> Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    if files_with_matches {
        // Only print unique filenames
        let mut seen = std::collections::HashSet::new();
        for m in matches {
            let key = format!("{}/{}", m.branch, m.file);
            if seen.insert(key) {
                writeln!(handle, "{}/{}", m.branch, m.file)?;
            }
        }
    } else {
        // Print all matches
        for m in matches {
            if line_numbers {
                writeln!(handle, "{}/{}:{}:{}", m.branch, m.file, m.line_number, m.line)?;
            } else {
                writeln!(handle, "{}/{}:{}", m.branch, m.file, m.line)?;
            }
        }
    }

    Ok(())
}
