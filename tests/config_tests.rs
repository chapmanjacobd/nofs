//! Configuration parsing tests

#[path = "common.rs"]
mod common;

use common::TestContext;

#[test]
fn test_basic_config_parsing() {
    let ctx = TestContext::new("config_basic");

    ctx.create_branch("disk1/media", &["file1.txt"]);
    ctx.create_branch("disk2/media", &["file2.txt"]);

    let config = format!(
        r#"
[union.media]
paths = ["{0}/disk1/media", "{0}/disk2/media"]
create_policy = "pfrd"
search_policy = "ff"
"#,
        ctx.root.display()
    );

    ctx.write_config(&config);

    let output = ctx.run_nofs(&[
        "--config",
        ctx.config_path.to_str().unwrap(),
        "info",
        "media",
    ]);

    assert!(output.success(), "Command failed: {}", output.stderr);
    assert!(output.stdout.contains("Union Context: media"));
    assert!(output.stdout.contains("Branches:     2"));
}

#[test]
fn test_ro_paths_config() {
    let ctx = TestContext::new("config_ro");

    ctx.create_branch("rw_branch", &["file1.txt"]);
    ctx.create_branch("ro_branch", &["file2.txt"]);

    let config = format!(
        r#"
[union.test]
paths = ["{0}/rw_branch"]
ro_paths = ["{0}/ro_branch"]
create_policy = "mfs"
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
    assert!(output.stdout.contains("[RW]"));
    assert!(output.stdout.contains("[RO]"));
}

#[test]
fn test_nc_paths_config() {
    let ctx = TestContext::new("config_nc");

    ctx.create_branch("rw_branch", &["file1.txt"]);
    ctx.create_branch("nc_branch", &["file2.txt"]);

    let config = format!(
        r#"
[union.test]
paths = ["{0}/rw_branch"]
nc_paths = ["{0}/nc_branch"]
create_policy = "mfs"
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
    assert!(output.stdout.contains("[RW]"));
    assert!(output.stdout.contains("[NC]"));
}

#[test]
fn test_multiple_unions() {
    let ctx = TestContext::new("config_multi");

    ctx.create_branch("media1", &["movie.mkv"]);
    ctx.create_branch("media2", &["show.mkv"]);
    ctx.create_branch("backup1", &["data.txt"]);
    ctx.create_branch("backup2", &["archive.txt"]);

    let config = format!(
        r#"
[union.media]
paths = ["{0}/media1", "{0}/media2"]
create_policy = "pfrd"

[union.backup]
paths = ["{0}/backup1", "{0}/backup2"]
create_policy = "mfs"
"#,
        ctx.root.display()
    );

    ctx.write_config(&config);

    let output = ctx.run_nofs(&["--config", ctx.config_path.to_str().unwrap(), "info"]);

    assert!(output.success(), "Command failed: {}", output.stderr);
    assert!(output.stdout.contains("media:"));
    assert!(output.stdout.contains("backup:"));
}

#[test]
fn test_all_path_types_combined() {
    let ctx = TestContext::new("config_combined");

    ctx.create_branch("rw1", &["file1.txt"]);
    ctx.create_branch("rw2", &["file2.txt"]);
    ctx.create_branch("ro1", &["file3.txt"]);
    ctx.create_branch("nc1", &["file4.txt"]);

    let config = format!(
        r#"
[union.test]
paths = ["{0}/rw1", "{0}/rw2"]
ro_paths = ["{0}/ro1"]
nc_paths = ["{0}/nc1"]
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
    assert!(output.stdout.contains("Branches:     4"));
    assert!(output.stdout.contains("Writable:   2"));
}

#[test]
fn test_context_path_syntax() {
    let ctx = TestContext::new("context_syntax");

    ctx.create_branch("disk1/media/movies", &["blade_runner.mkv"]);
    ctx.create_branch("disk2/media/movies", &["aliens.mkv"]);

    let config = format!(
        r#"
[union.media]
paths = ["{0}/disk1/media", "{0}/disk2/media"]
create_policy = "pfrd"
"#,
        ctx.root.display()
    );

    ctx.write_config(&config);

    // Test context:path syntax
    let output = ctx.run_nofs(&[
        "--config",
        ctx.config_path.to_str().unwrap(),
        "ls",
        "media:movies",
    ]);

    assert!(output.success(), "Command failed: {}", output.stderr);
    assert!(output.stdout.contains("blade_runner.mkv"));
    assert!(output.stdout.contains("aliens.mkv"));
}

#[test]
fn test_minfreespace_config() {
    let ctx = TestContext::new("config_minfree");

    ctx.create_branch("disk1", &["file1.txt"]);
    ctx.create_branch("disk2", &["file2.txt"]);

    let config = format!(
        r#"
[union.test]
paths = ["{0}/disk1", "{0}/disk2"]
minfreespace = "1G"
create_policy = "mfs"
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
    assert!(output.stdout.contains("Min Free Space: 1073741824 bytes"));
}
