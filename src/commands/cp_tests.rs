//! Tests for cp/mv commands

use crate::commands::cp::{
    execute as cp_execute, parse_file_over_file, CopyConfig, FileOverFileMode, FolderConflictMode,
};
use crate::commands::mv::execute as mv_execute;
use std::fs;
use std::path::{Path, PathBuf};

fn setup_test_dir(name: &str) -> PathBuf {
    let test_dir = std::env::temp_dir().join(format!("nofs_test_{name}"));
    let _ = fs::remove_dir_all(&test_dir);
    fs::create_dir_all(&test_dir).unwrap();
    test_dir
}

fn cleanup_test_dir(test_dir: &PathBuf) {
    let _ = fs::remove_dir_all(test_dir);
}

fn create_file(path: &PathBuf, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn read_file(path: &PathBuf) -> String {
    fs::read_to_string(path).unwrap()
}

fn file_exists(path: &Path) -> bool {
    path.exists()
}

#[test]
fn test_simple_copy() {
    let test_dir = setup_test_dir("simple_copy");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    create_file(&src_file, "hello world");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());

    let dest_file = dest.join("src").join("file.txt");
    assert!(file_exists(&dest_file));
    assert_eq!(read_file(&dest_file), "hello world");
    assert!(file_exists(&src_file)); // Source should still exist (copy mode)

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_simple_move() {
    let test_dir = setup_test_dir("simple_move");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    create_file(&src_file, "hello world");

    let result = mv_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        "delete-dest",
        "merge",
        "merge",
        false,
        1,
        false,
        vec![],
        vec![],
        vec![],
        None,
        None,
        None,
    );

    assert!(result.is_ok());

    let dest_file = dest.join("src").join("file.txt");
    assert!(file_exists(&dest_file));
    assert_eq!(read_file(&dest_file), "hello world");
    assert!(!file_exists(&src_file)); // Source should be gone (move mode)

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_conflict_skip() {
    let test_dir = setup_test_dir("conflict_skip");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_file = dest.join("file.txt");
    create_file(&src_file, "new content");
    create_file(&dest_file, "old content");

    let mut strategy = parse_file_over_file("skip").unwrap();
    strategy.required = FileOverFileMode::Skip;

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "old content"); // Should keep old content
    assert_eq!(read_file(&src_file), "new content"); // Source unchanged

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_conflict_delete_dest() {
    let test_dir = setup_test_dir("conflict_delete_dest");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    // When copying src to dest, the result is dest/src/file.txt
    // To create a conflict, we need dest/src/file.txt to exist first
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "new content");
    create_file(&dest_file, "old content");

    let strategy = parse_file_over_file("delete-dest").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "new content"); // Should have new content

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_conflict_rename_dest() {
    let test_dir = setup_test_dir("conflict_rename_dest");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    // When copying src to dest, the result is dest/src/file.txt
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "new content");
    create_file(&dest_file, "old content");

    let strategy = parse_file_over_file("rename-dest").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "new content"); // New content at original path
    let renamed_file = dest_src.join("file_1.txt");
    assert!(file_exists(&renamed_file));
    assert_eq!(read_file(&renamed_file), "old content"); // Old content renamed

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_conflict_rename_src() {
    let test_dir = setup_test_dir("conflict_rename_src");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    // When copying src to dest, the result is dest/src/file.txt
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "new content");
    create_file(&dest_file, "old content");

    let strategy = parse_file_over_file("rename-src").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "old content"); // Old content stays
    let renamed_file = dest_src.join("file_1.txt");
    assert!(file_exists(&renamed_file));
    assert_eq!(read_file(&renamed_file), "new content"); // New content renamed

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_hash_skip() {
    let test_dir = setup_test_dir("hash_skip");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_file = dest.join("file.txt");
    create_file(&src_file, "identical content");
    create_file(&dest_file, "identical content");

    let strategy = parse_file_over_file("skip-hash skip").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "identical content"); // Unchanged

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_size_skip() {
    let test_dir = setup_test_dir("size_skip");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_file = dest.join("file.txt");
    create_file(&src_file, "same");
    create_file(&dest_file, "same");

    let strategy = parse_file_over_file("skip-size skip").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "same"); // Unchanged

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_skip_larger() {
    let test_dir = setup_test_dir("skip_larger");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_file = dest.join("file.txt");
    create_file(&src_file, "larger content here");
    create_file(&dest_file, "small");

    let strategy = parse_file_over_file("skip-larger skip").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "small"); // Source is larger, so skipped

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_delete_dest_larger() {
    let test_dir = setup_test_dir("delete_dest_larger");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    // When copying src to dest, the result is dest/src/file.txt
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "small");
    create_file(&dest_file, "larger content here");

    let strategy = parse_file_over_file("delete-dest-larger delete-dest").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "small"); // Dest was larger, deleted and replaced

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_directory_recursive() {
    let test_dir = setup_test_dir("recursive");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    // Create nested structure
    create_file(&src.join("file1.txt"), "content1");
    create_file(&src.join("subdir").join("file2.txt"), "content2");
    create_file(
        &src.join("subdir").join("deep").join("file3.txt"),
        "content3",
    );

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());

    let dest_file1 = dest.join("src").join("file1.txt");
    let dest_file2 = dest.join("src").join("subdir").join("file2.txt");
    let dest_file3 = dest
        .join("src")
        .join("subdir")
        .join("deep")
        .join("file3.txt");

    assert!(file_exists(&dest_file1));
    assert!(file_exists(&dest_file2));
    assert!(file_exists(&dest_file3));
    assert_eq!(read_file(&dest_file1), "content1");
    assert_eq!(read_file(&dest_file2), "content2");
    assert_eq!(read_file(&dest_file3), "content3");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_simulation() {
    let test_dir = setup_test_dir("simulation");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    create_file(&src_file, "hello");

    let config = CopyConfig {
        copy: true,
        simulate: true, // Dry run
        workers: 1,
        verbose: false,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());

    // Destination should NOT have the file (simulation mode)
    let dest_file = dest.join("src").join("file.txt");
    assert!(!file_exists(&dest_file));
    assert!(file_exists(&src_file)); // Source unchanged

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_extension_filter() {
    let test_dir = setup_test_dir("ext_filter");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    create_file(&src.join("file.txt"), "text");
    create_file(&src.join("file.go"), "go code");
    create_file(&src.join("file.rs"), "rust code");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        extensions: vec![".txt".to_string(), ".go".to_string()],
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());

    assert!(file_exists(&dest.join("src").join("file.txt")));
    assert!(file_exists(&dest.join("src").join("file.go")));
    assert!(!file_exists(&dest.join("src").join("file.rs"))); // Filtered out

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_file_limit() {
    let test_dir = setup_test_dir("file_limit");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    create_file(&src.join("file1.txt"), "1");
    create_file(&src.join("file2.txt"), "2");
    create_file(&src.join("file3.txt"), "3");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        limit: Some(2),
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());

    // Should have copied only 2 files
    let dest_dir = dest.join("src");
    let entries: Vec<_> = fs::read_dir(&dest_dir)
        .unwrap()
        .filter(|e| e.as_ref().unwrap().file_type().unwrap().is_file())
        .collect();
    assert_eq!(entries.len(), 2);

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_folder_over_file_conflict() {
    let test_dir = setup_test_dir("folder_over_file");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    // Create folder in src
    fs::create_dir_all(src.join("conflict")).unwrap();
    create_file(&src.join("conflict").join("inner.txt"), "in folder");

    // When copying src to dest, the result is dest/src/conflict
    // So we need to create a file at dest/src/conflict
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    create_file(&dest_src.join("conflict"), "is a file");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        folder_over_file: FolderConflictMode::RenameDest,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());

    // Folder should exist at original path
    assert!(file_exists(&dest_src.join("conflict").join("inner.txt")));
    // File should be renamed
    assert!(file_exists(&dest_src.join("conflict_1")));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_file_over_folder_conflict() {
    let test_dir = setup_test_dir("file_over_folder");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    // Create file in src
    create_file(&src.join("conflict"), "is a file");

    // When copying src to dest, the result is dest/src/conflict
    // So we need to create a folder at dest/src/conflict
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    fs::create_dir_all(dest_src.join("conflict")).unwrap();
    create_file(&dest_src.join("conflict").join("inner.txt"), "in folder");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_folder: FolderConflictMode::Merge,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());

    // File should be moved into folder as conflict/conflict
    assert!(file_exists(&dest_src.join("conflict").join("conflict")));
    assert!(file_exists(&dest_src.join("conflict").join("inner.txt")));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_multiple_sources() {
    let test_dir = setup_test_dir("multiple_sources");
    let src1 = test_dir.join("src1");
    let src2 = test_dir.join("src2");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src1).unwrap();
    fs::create_dir_all(&src2).unwrap();
    fs::create_dir_all(&dest).unwrap();

    create_file(&src1.join("file1.txt"), "from src1");
    create_file(&src2.join("file2.txt"), "from src2");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        ..Default::default()
    };

    let sources = vec![
        src1.to_string_lossy().to_string(),
        src2.to_string_lossy().to_string(),
    ];

    let result = cp_execute(&sources, &dest.to_string_lossy(), &config, None);

    assert!(result.is_ok());

    assert!(file_exists(&dest.join("src1").join("file1.txt")));
    assert!(file_exists(&dest.join("src2").join("file2.txt")));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_parse_file_over_file() {
    // Test simple strategy
    let strategy = parse_file_over_file("skip").unwrap();
    assert_eq!(strategy.required, FileOverFileMode::Skip);

    // Test strategy with optional
    let strategy2 = parse_file_over_file("skip-hash rename-dest").unwrap();
    assert!(strategy2.skip_hash);
    assert_eq!(strategy2.required, FileOverFileMode::RenameDest);

    // Test multiple optionals
    let strategy3 =
        parse_file_over_file("skip-hash skip-size delete-dest-larger delete-dest").unwrap();
    assert!(strategy3.skip_hash);
    assert!(strategy3.skip_size);
    assert!(strategy3.delete_dest_larger);
    assert_eq!(strategy3.required, FileOverFileMode::DeleteDest);
}

#[test]
fn test_format_size() {
    use crate::commands::cp::format_size;

    assert_eq!(format_size(0), "0 B");
    assert_eq!(format_size(500), "500 B");
    assert_eq!(format_size(1000), "1.0 KB");
    assert_eq!(format_size(1500), "1.5 KB");
    assert_eq!(format_size(1_000_000), "1.0 MB");
    assert_eq!(format_size(1_500_000), "1.5 MB");
    assert_eq!(format_size(1_000_000_000), "1.0 GB");
    assert_eq!(format_size(2_500_000_000), "2.5 GB");
}

#[test]
fn test_copy_with_conflict_delete_src() {
    let test_dir = setup_test_dir("conflict_delete_src");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "new content");
    create_file(&dest_file, "old content");

    let strategy = parse_file_over_file("delete-src").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "old content"); // Dest unchanged
    assert!(!file_exists(&src_file)); // Source deleted

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_skip_smaller() {
    let test_dir = setup_test_dir("skip_smaller");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "small");
    create_file(&dest_file, "larger content here");

    let strategy = parse_file_over_file("skip-smaller skip").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "larger content here"); // Dest unchanged (src is smaller)

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_delete_dest_smaller() {
    let test_dir = setup_test_dir("delete_dest_smaller");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "larger content here");
    create_file(&dest_file, "small");

    let strategy = parse_file_over_file("delete-dest-smaller delete-dest").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "larger content here"); // Dest was smaller, replaced

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_delete_src_smaller() {
    let test_dir = setup_test_dir("delete_src_smaller");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "small");
    create_file(&dest_file, "larger content here");

    let strategy = parse_file_over_file("delete-src-smaller skip").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "larger content here"); // Dest unchanged
    assert!(!file_exists(&src_file)); // Source was smaller, deleted

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_delete_src_larger() {
    let test_dir = setup_test_dir("delete_src_larger");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "larger content here");
    create_file(&dest_file, "small");

    let strategy = parse_file_over_file("delete-src-larger skip").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "small"); // Dest unchanged
    assert!(!file_exists(&src_file)); // Source was larger, deleted

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_file_over_folder_skip() {
    let test_dir = setup_test_dir("file_over_folder_skip");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    create_file(&src.join("conflict"), "is a file");

    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    fs::create_dir_all(dest_src.join("conflict")).unwrap();
    create_file(&dest_src.join("conflict").join("inner.txt"), "in folder");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_folder: FolderConflictMode::Skip,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    // Folder unchanged, file not copied
    assert!(file_exists(&dest_src.join("conflict").join("inner.txt")));
    assert!(!file_exists(&dest_src.join("conflict").join("conflict")));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_file_over_folder_delete_dest() {
    let test_dir = setup_test_dir("file_over_folder_delete_dest");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    create_file(&src.join("conflict"), "is a file");

    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    fs::create_dir_all(dest_src.join("conflict")).unwrap();
    create_file(&dest_src.join("conflict").join("inner.txt"), "in folder");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_folder: FolderConflictMode::DeleteDest,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    // Folder deleted, file at original path
    assert!(file_exists(&dest_src.join("conflict")));
    assert_eq!(read_file(&dest_src.join("conflict")), "is a file");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_folder_over_file_skip() {
    let test_dir = setup_test_dir("folder_over_file_skip");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    fs::create_dir_all(src.join("conflict")).unwrap();
    create_file(&src.join("conflict").join("inner.txt"), "in folder");

    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    create_file(&dest_src.join("conflict"), "is a file");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        folder_over_file: FolderConflictMode::Skip,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    // File unchanged, folder not copied
    assert!(file_exists(&dest_src.join("conflict")));
    assert_eq!(read_file(&dest_src.join("conflict")), "is a file");
    assert!(!file_exists(&dest_src.join("conflict").join("inner.txt")));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_folder_over_file_delete_src() {
    let test_dir = setup_test_dir("folder_over_file_delete_src");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    fs::create_dir_all(src.join("conflict")).unwrap();
    create_file(&src.join("conflict").join("inner.txt"), "in folder");

    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    create_file(&dest_src.join("conflict"), "is a file");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        folder_over_file: FolderConflictMode::DeleteSrc,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    // File unchanged, folder deleted from source
    assert!(file_exists(&dest_src.join("conflict")));
    assert_eq!(read_file(&dest_src.join("conflict")), "is a file");
    assert!(!file_exists(&src.join("conflict").join("inner.txt")));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_folder_over_file_delete_dest() {
    let test_dir = setup_test_dir("folder_over_file_delete_dest");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    fs::create_dir_all(src.join("conflict")).unwrap();
    create_file(&src.join("conflict").join("inner.txt"), "in folder");

    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    create_file(&dest_src.join("conflict"), "is a file");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        folder_over_file: FolderConflictMode::DeleteDest,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    // File deleted, folder created
    assert!(file_exists(&dest_src.join("conflict").join("inner.txt")));
    assert!(!file_exists(&dest_src.join("conflict_1")));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_folder_over_file_rename_src() {
    let test_dir = setup_test_dir("folder_over_file_rename_src");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    fs::create_dir_all(src.join("conflict")).unwrap();
    create_file(&src.join("conflict").join("inner.txt"), "in folder");

    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    create_file(&dest_src.join("conflict"), "is a file");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        folder_over_file: FolderConflictMode::RenameSrc,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    // File unchanged, folder at renamed path
    assert!(file_exists(&dest_src.join("conflict")));
    assert!(file_exists(&dest_src.join("conflict_1").join("inner.txt")));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_folder_over_folder_merge() {
    let test_dir = setup_test_dir("folder_over_folder_merge");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    // Source has subdir with files
    fs::create_dir_all(src.join("subdir")).unwrap();
    create_file(&src.join("subdir").join("file1.txt"), "from src");

    // Dest already has subdir with different file
    fs::create_dir_all(dest.join("src").join("subdir")).unwrap();
    create_file(
        &dest.join("src").join("subdir").join("file2.txt"),
        "from dest",
    );

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    // Both files should exist (merged)
    assert!(file_exists(
        &dest.join("src").join("subdir").join("file1.txt")
    ));
    assert!(file_exists(
        &dest.join("src").join("subdir").join("file2.txt")
    ));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_delete_dest_hash() {
    let test_dir = setup_test_dir("delete_dest_hash");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "identical content");
    create_file(&dest_file, "identical content");

    let strategy = parse_file_over_file("delete-dest-hash delete-dest").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "identical content"); // Replaced with same content

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_delete_dest_size() {
    let test_dir = setup_test_dir("delete_dest_size");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "same size!");
    create_file(&dest_file, "same size?");

    let strategy = parse_file_over_file("delete-dest-size delete-dest").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "same size!"); // Replaced

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_delete_src_hash() {
    let test_dir = setup_test_dir("delete_src_hash");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "identical content");
    create_file(&dest_file, "identical content");

    let strategy = parse_file_over_file("delete-src-hash skip").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "identical content"); // Dest unchanged
    assert!(!file_exists(&src_file)); // Source deleted (hash matched)

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_copy_with_delete_src_size() {
    let test_dir = setup_test_dir("delete_src_size");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    let src_file = src.join("file.txt");
    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    let dest_file = dest_src.join("file.txt");

    create_file(&src_file, "same size!");
    create_file(&dest_file, "same size?");

    let strategy = parse_file_over_file("delete-src-size skip").unwrap();

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_file: strategy,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    assert_eq!(read_file(&dest_file), "same size?"); // Dest unchanged
    assert!(!file_exists(&src_file)); // Source deleted (size matched)

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_file_over_folder_rename_src() {
    let test_dir = setup_test_dir("file_over_folder_rename_src");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    create_file(&src.join("conflict"), "is a file");

    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    fs::create_dir_all(dest_src.join("conflict")).unwrap();
    create_file(&dest_src.join("conflict").join("inner.txt"), "in folder");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_folder: FolderConflictMode::RenameSrc,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    // File should be renamed (conflict_1) at dest/src level
    assert!(file_exists(&dest_src.join("conflict_1")));
    assert_eq!(read_file(&dest_src.join("conflict_1")), "is a file");
    // Folder unchanged
    assert!(file_exists(&dest_src.join("conflict").join("inner.txt")));

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_file_over_folder_rename_dest() {
    let test_dir = setup_test_dir("file_over_folder_rename_dest");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    create_file(&src.join("conflict"), "is a file");

    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    fs::create_dir_all(dest_src.join("conflict")).unwrap();
    create_file(&dest_src.join("conflict").join("inner.txt"), "in folder");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        file_over_folder: FolderConflictMode::RenameDest,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    // Folder should be renamed
    assert!(file_exists(&dest_src.join("conflict_1").join("inner.txt")));
    // File at original path
    assert!(file_exists(&dest_src.join("conflict")));
    assert_eq!(read_file(&dest_src.join("conflict")), "is a file");

    cleanup_test_dir(&test_dir);
}

#[test]
fn test_folder_over_file_rename_dest() {
    let test_dir = setup_test_dir("folder_over_file_rename_dest");
    let src = test_dir.join("src");
    let dest = test_dir.join("dest");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&dest).unwrap();

    fs::create_dir_all(src.join("conflict")).unwrap();
    create_file(&src.join("conflict").join("inner.txt"), "in folder");

    let dest_src = dest.join("src");
    fs::create_dir_all(&dest_src).unwrap();
    create_file(&dest_src.join("conflict"), "is a file");

    let config = CopyConfig {
        copy: true,
        simulate: false,
        workers: 1,
        verbose: false,
        folder_over_file: FolderConflictMode::RenameDest,
        ..Default::default()
    };

    let result = cp_execute(
        &[src.to_string_lossy().to_string()],
        &dest.to_string_lossy(),
        &config,
        None,
    );

    assert!(result.is_ok());
    // File should be renamed
    assert!(file_exists(&dest_src.join("conflict_1")));
    assert_eq!(read_file(&dest_src.join("conflict_1")), "is a file");
    // Folder at original path with contents
    assert!(file_exists(&dest_src.join("conflict").join("inner.txt")));

    cleanup_test_dir(&test_dir);
}
