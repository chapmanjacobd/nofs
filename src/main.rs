//! nofs - A lightweight shared filesystem tool
//!
//! Provides mergerfs-like functionality without FUSE.
//! All operations happen via subcommands with optional TOML configuration.

pub mod branch;
pub mod commands;
pub mod config;
pub mod error;
pub mod policy;
pub mod pool;

use anyhow::Result;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "nofs")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, global = true)]
    config: Option<String>,

    /// Comma-separated list of branch paths (ad-hoc mode)
    /// Format: /path1,/path2 or /path1=RW,/path2=RO
    #[arg(long, global = true)]
    paths: Option<String>,

    /// Policy to use for branch selection
    #[arg(long, global = true, default_value = "pfrd")]
    policy: String,

    /// Minimum free space required on branch (e.g., "4G", "100M")
    #[arg(long, global = true, default_value = "4G")]
    minfreespace: String,

    /// Verbose output (print decision steps to stderr)
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// List directory contents (like ls).
    Ls {
        /// Path within the share (format: [context:]path).
        path: String,

        /// Show detailed information.
        #[arg(short, long)]
        long: bool,

        /// Show hidden files.
        #[arg(short, long)]
        all: bool,
    },

    /// Find files matching a pattern.
    Find {
        /// Starting path within the share (format: [context:]path).
        path: String,

        /// Filename pattern (glob).
        #[arg(long)]
        name: Option<String>,

        /// File type: f=file, d=directory.
        #[arg(long)]
        type_: Option<String>,

        /// Maximum depth.
        #[arg(long)]
        maxdepth: Option<usize>,
    },

    /// Find which branch contains a file.
    #[command(alias = "where")]
    Which {
        /// Path within the share (format: [context:]path).
        path: String,

        /// Show all branches containing the file.
        #[arg(short, long)]
        all: bool,
    },

    /// Get the best branch path for creating a new file.
    Create {
        /// Path within the share (format: [context:]path).
        path: String,
    },

    /// Show filesystem statistics.
    Stat {
        /// Path within the share (defaults to root).
        path: Option<String>,

        /// Show human-readable sizes.
        #[arg(short = 'H', long)]
        human: bool,
    },

    /// Show share configuration and status.
    Info {
        /// Context name (optional, shows all if not specified).
        context: Option<String>,
    },

    /// Check if a file exists and return its location.
    Exists {
        /// Path within the share (format: [context:]path).
        path: String,
    },

    /// Read file content (from first found branch).
    Cat {
        /// Path within the share (format: [context:]path).
        path: String,
    },

    /// Copy files/directories (supports nofs context paths).
    Cp {
        /// Source paths [...] and destination (last argument).
        /// Format: [context:]path or regular path.
        #[arg(required = true)]
        paths: Vec<String>,

        /// File-over-file conflict strategy (e.g., "skip-hash rename-dest").
        #[arg(long, default_value = "delete-src-hash rename-dest")]
        file_over_file: String,

        /// File-over-folder conflict strategy.
        #[arg(long, default_value = "merge")]
        file_over_folder: String,

        /// Folder-over-file conflict strategy.
        #[arg(long, default_value = "merge")]
        folder_over_file: String,

        /// Simulate without making changes (dry-run).
        #[arg(short = 'n', long, alias = "simulate")]
        dry_run: bool,

        /// Number of parallel workers.
        #[arg(short = 'j', long, default_value = "4")]
        workers: usize,

        /// Filter by file extensions (e.g., .mkv, .jpg).
        #[arg(short = 'e', long)]
        ext: Vec<String>,

        /// Exclude patterns (glob).
        #[arg(short = 'E', long)]
        exclude: Vec<String>,

        /// Include patterns (glob).
        #[arg(short = 'I', long)]
        include: Vec<String>,

        /// Filter by file size (e.g., +5M, -10M).
        #[arg(short = 'S', long)]
        size: Vec<String>,

        /// Limit number of files transferred.
        #[arg(short = 'l', long)]
        limit: Option<u64>,

        /// Limit total size transferred (e.g., 100M, 1G).
        #[arg(long)]
        size_limit: Option<String>,
    },

    /// Move files/directories (supports nofs context paths).
    Mv {
        /// Source paths [...] and destination (last argument).
        /// Format: [context:]path or regular path.
        #[arg(required = true)]
        paths: Vec<String>,

        /// File-over-file conflict strategy (e.g., "skip-hash rename-dest").
        #[arg(long, default_value = "delete-src-hash rename-dest")]
        file_over_file: String,

        /// File-over-folder conflict strategy.
        #[arg(long, default_value = "merge")]
        file_over_folder: String,

        /// Folder-over-file conflict strategy.
        #[arg(long, default_value = "merge")]
        folder_over_file: String,

        /// Simulate without making changes (dry-run).
        #[arg(short = 'n', long, alias = "simulate")]
        dry_run: bool,

        /// Number of parallel workers.
        #[arg(short = 'j', long, default_value = "4")]
        workers: usize,

        /// Filter by file extensions (e.g., .mkv, .jpg).
        #[arg(short = 'e', long)]
        ext: Vec<String>,

        /// Exclude patterns (glob).
        #[arg(short = 'E', long)]
        exclude: Vec<String>,

        /// Include patterns (glob).
        #[arg(short = 'I', long)]
        include: Vec<String>,

        /// Filter by file size (e.g., +5M, -10M).
        #[arg(short = 'S', long)]
        size: Vec<String>,

        /// Limit number of files transferred.
        #[arg(short = 'l', long)]
        limit: Option<u64>,

        /// Limit total size transferred (e.g., 100M, 1G).
        #[arg(long)]
        size_limit: Option<String>,
    },

    /// Remove files or directories.
    Rm {
        /// Path(s) within the share (format: [context:]path).
        #[arg(required = true)]
        paths: Vec<String>,

        /// Remove directories and their contents recursively.
        #[arg(short, long)]
        recursive: bool,

        /// Verbose output.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Create directories.
    Mkdir {
        /// Path within the share (format: [context:]path).
        #[arg(required = true)]
        path: String,

        /// Create parent directories as needed.
        #[arg(short, long)]
        parents: bool,

        /// Verbose output.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Remove empty directories.
    Rmdir {
        /// Path within the share (format: [context:]path).
        #[arg(required = true)]
        path: String,

        /// Verbose output.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Create or update files.
    Touch {
        /// Path within the share (format: [context:]path).
        #[arg(required = true)]
        path: String,

        /// Verbose output.
        #[arg(short, long)]
        verbose: bool,
    },
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize the share manager based on config or ad-hoc paths
    let pool_mgr = if let Some(config_path) = &cli.config {
        pool::PoolManager::from_config(config_path)?
    } else if let Some(paths_str) = &cli.paths {
        pool::PoolManager::from_paths(paths_str, &cli.policy, &cli.minfreespace)?
    } else {
        // Try default config location
        pool::PoolManager::from_default_config()?
    };

    // Execute the command
    match cli.command {
        Commands::Ls { path, long, all } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::ls::execute(pool, pool_path, long, all, cli.verbose)?;
        }
        Commands::Find {
            path,
            name,
            type_,
            maxdepth,
        } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::find::execute(
                pool,
                pool_path,
                name.as_deref(),
                type_.as_deref(),
                maxdepth,
                cli.verbose,
            )?;
        }
        Commands::Which { path, all } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::which::execute(pool, pool_path, all, cli.verbose)?;
        }
        Commands::Create { path } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::create::execute(pool, pool_path, cli.verbose)?;
        }
        Commands::Stat { path, human } => {
            let pool = if let Some(p) = &path {
                let (pool, _) = pool_mgr.resolve_context_path(p)?;
                pool
            } else {
                pool_mgr.default_pool()?
            };
            commands::stat::execute(pool, human, cli.verbose)?;
        }
        Commands::Info { context } => {
            if let Some(ctx) = &context {
                let pool = pool_mgr.get_pool(ctx)?;
                commands::info::execute_single(pool, cli.verbose)?;
            } else {
                commands::info::execute_all(&pool_mgr, cli.verbose)?;
            }
        }
        Commands::Exists { path } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::exists::execute(pool, pool_path, cli.verbose)?;
        }
        Commands::Cat { path } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::cat::execute(pool, pool_path, cli.verbose)?;
        }
        Commands::Cp {
            paths,
            file_over_file,
            file_over_folder,
            folder_over_file,
            dry_run,
            workers,
            ext,
            exclude,
            include,
            size: _,
            limit,
            size_limit,
        } => {
            // Parse sources and destination
            if paths.len() < 2 {
                return Err(anyhow::anyhow!(
                    "At least one source and one destination are required"
                ));
            }
            #[allow(clippy::expect_used)]
            let (destination, sources) = paths
                .split_last()
                .expect("paths must have at least 2 elements");

            // Parse size limit
            let parsed_size_limit = size_limit.as_ref().and_then(|s| parse_size(s).ok());

            // Get share for context-aware paths
            let share = extract_share_from_paths(&pool_mgr, sources, destination)?;

            let config = commands::cp::CopyConfig {
                copy: true,
                simulate: dry_run,
                workers,
                verbose: cli.verbose,
                file_over_file: commands::cp::parse_file_over_file(&file_over_file)?,
                file_over_folder: parse_folder_conflict_mode(&file_over_folder)?,
                folder_over_file: parse_folder_conflict_mode(&folder_over_file)?,
                extensions: ext,
                exclude,
                include,
                limit,
                size_limit: parsed_size_limit,
            };

            commands::cp::execute(sources, destination, &config, share)?;
        }
        Commands::Mv {
            paths,
            file_over_file,
            file_over_folder,
            folder_over_file,
            dry_run,
            workers,
            ext,
            exclude,
            include,
            size: _,
            limit,
            size_limit,
        } => {
            // Parse sources and destination
            if paths.len() < 2 {
                return Err(anyhow::anyhow!(
                    "At least one source and one destination are required"
                ));
            }
            #[allow(clippy::expect_used)]
            let (destination, sources) = paths
                .split_last()
                .expect("paths must have at least 2 elements");

            // Parse size limit
            let parsed_size_limit = size_limit.as_ref().and_then(|s| parse_size(s).ok());

            // Get share for context-aware paths
            let share = extract_share_from_paths(&pool_mgr, sources, destination)?;

            commands::mv::execute(
                sources,
                destination,
                &file_over_file,
                &file_over_folder,
                &folder_over_file,
                dry_run,
                workers,
                cli.verbose,
                ext,
                exclude,
                include,
                limit,
                parsed_size_limit,
                share,
            )?;
        }
        Commands::Rm {
            paths,
            recursive,
            verbose,
        } => {
            let mut any_failed = false;
            for path in paths {
                let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
                if let Err(e) =
                    commands::rm::execute(pool, pool_path, recursive, verbose || cli.verbose)
                {
                    eprintln!("nofs: {e}");
                    any_failed = true;
                }
            }
            if any_failed {
                return Err(anyhow::anyhow!("Some removal operations failed"));
            }
        }
        Commands::Mkdir {
            path,
            parents,
            verbose,
        } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::mkdir::execute(pool, pool_path, parents, verbose || cli.verbose)?;
        }
        Commands::Rmdir { path, verbose } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::rmdir::execute(pool, pool_path, verbose || cli.verbose)?;
        }
        Commands::Touch { path, verbose } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::touch::execute(pool, pool_path, verbose || cli.verbose)?;
        }
    }

    Ok(())
}

fn parse_folder_conflict_mode(s: &str) -> Result<commands::cp::FolderConflictMode> {
    use commands::cp::FolderConflictMode;
    match s.to_lowercase().as_str() {
        "skip" => Ok(FolderConflictMode::Skip),
        "rename-src" => Ok(FolderConflictMode::RenameSrc),
        "rename-dest" => Ok(FolderConflictMode::RenameDest),
        "delete-src" => Ok(FolderConflictMode::DeleteSrc),
        "delete-dest" => Ok(FolderConflictMode::DeleteDest),
        "merge" => Ok(FolderConflictMode::Merge),
        _ => Err(anyhow::anyhow!("Unknown folder conflict mode: {s}")),
    }
}

fn parse_size(s: &str) -> Result<u64> {
    use crate::policy::parse_size as policy_parse_size;
    policy_parse_size(s).map_err(|e| anyhow::anyhow!("Parse error: {e}"))
}

/// Try to extract a share from paths that contain context prefixes
fn extract_share_from_paths<'a>(
    pool_mgr: &'a pool::PoolManager,
    sources: &[String],
    destination: &str,
) -> Result<Option<&'a pool::Pool>> {
    // Check if any path has a context prefix
    for path in sources
        .iter()
        .chain(std::iter::once(&destination.to_string()))
    {
        if let Some((ctx, _)) = path.split_once(':') {
            if !ctx.contains('/') {
                // This looks like a context prefix
                let share = pool_mgr.get_pool(ctx)?;
                return Ok(Some(share));
            }
        }
    }
    Ok(None)
}
