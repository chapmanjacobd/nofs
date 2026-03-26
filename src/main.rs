//! nofs - A lightweight union filesystem tool
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

    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// List directory contents (like ls)
    Ls {
        /// Path within the pool
        path: String,

        /// Show detailed information
        #[arg(short, long)]
        long: bool,

        /// Show hidden files
        #[arg(short, long)]
        all: bool,
    },

    /// Find files matching a pattern
    Find {
        /// Starting path within the pool
        path: String,

        /// Filename pattern (glob)
        #[arg(long)]
        name: Option<String>,

        /// File type: f=file, d=directory
        #[arg(long)]
        type_: Option<String>,

        /// Maximum depth
        #[arg(long)]
        maxdepth: Option<usize>,
    },

    /// Find which branch contains a file
    Where {
        /// Path within the pool
        path: String,

        /// Show all branches containing the file
        #[arg(short, long)]
        all: bool,
    },

    /// Get the best branch path for creating a new file
    Create {
        /// Path within the pool
        path: String,
    },

    /// Show filesystem statistics
    Stat {
        /// Path within the pool (defaults to root)
        path: Option<String>,

        /// Show human-readable sizes
        #[arg(short = 'H', long)]
        human: bool,
    },

    /// Show pool configuration and status
    Info,

    /// Check if a file exists and return its location
    Exists {
        /// Path within the pool
        path: String,
    },

    /// Read file content (from first found branch)
    Cat {
        /// Path within the pool
        path: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize the pool based on config or ad-hoc paths
    let pool = if let Some(config_path) = &cli.config {
        pool::Pool::from_config(config_path)?
    } else if let Some(paths_str) = &cli.paths {
        pool::Pool::from_paths(paths_str, &cli.policy, &cli.minfreespace)?
    } else {
        // Try default config location
        pool::Pool::from_default_config()?
    };

    // Execute the command
    match cli.command {
        Commands::Ls { path, long, all } => {
            commands::ls::execute(&pool, &path, long, all)?;
        }
        Commands::Find { path, name, type_, maxdepth } => {
            commands::find::execute(&pool, &path, name.as_deref(), type_.as_deref(), maxdepth)?;
        }
        Commands::Where { path, all } => {
            commands::where_::execute(&pool, &path, all)?;
        }
        Commands::Create { path } => {
            commands::create::execute(&pool, &path, &cli.policy)?;
        }
        Commands::Stat { path, human } => {
            commands::stat::execute(&pool, path.as_deref(), human)?;
        }
        Commands::Info => {
            commands::info::execute(&pool)?;
        }
        Commands::Exists { path } => {
            commands::exists::execute(&pool, &path)?;
        }
        Commands::Cat { path } => {
            commands::cat::execute(&pool, &path)?;
        }
    }

    Ok(())
}
