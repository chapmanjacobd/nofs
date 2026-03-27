//! nofs - A lightweight shared filesystem tool
//!
//! Provides mergerfs-like functionality without FUSE.
//! All operations happen via subcommands with optional TOML configuration.

pub mod branch;
pub mod cache;
pub mod commands;
pub mod config;
pub mod conflict;
pub mod error;
pub mod output;
pub mod policy;
pub mod pool;

use anyhow::Result;
use clap::Parser;

/// Command-line interface for nofs
#[derive(Parser, Debug)]
#[command(name = "nofs")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    // Configuration options
    /// Path to configuration file
    #[arg(short, long, global = true, help_heading = "Configuration")]
    config: Option<String>,

    /// Comma-separated list of branch paths (ad-hoc mode)
    /// Format: /path1,/path2 or /path1=RW,/path2=RO
    #[arg(long, global = true, help_heading = "Configuration")]
    paths: Option<String>,

    // Policy options
    /// Policy to use for branch selection
    #[arg(long, global = true, default_value = "pfrd", help_heading = "Policy")]
    policy: String,

    /// Minimum free space required on branch (e.g., "4G", "100M")
    #[arg(long, global = true, default_value = "4G", help_heading = "Policy")]
    minfreespace: String,

    // Output options
    /// Verbose output (print decision steps to stderr)
    #[arg(short, long, global = true, help_heading = "Output")]
    verbose: bool,

    /// Output in JSON format (for scripting/automation)
    #[arg(long, global = true, help_heading = "Output")]
    json: bool,

    /// Subcommand to execute
    #[command(subcommand)]
    command: Commands,
}

/// Available subcommands for nofs
#[derive(clap::Subcommand, Debug)]
enum Commands {
    /// List directory contents (like ls).
    #[command(after_help = "\
EXAMPLES:
    nofs ls media:/                    # List root of media share
    nofs ls -l media:/photos           # Detailed listing
    nofs ls --conflicts media:/docs    # Detect conflicting files
    nofs ls --conflicts --hash media:/ # Use hash for conflict detection

CONFLICT DETECTION:
    --conflicts
        Detect files with the same name but different content across branches.
        Files are marked in output when conflicts are found.
    
    --hash
        Use full file hash comparison instead of size/mtime for conflict detection.
        More accurate but slower on large files. Requires --conflicts flag.")]
    Ls {
        /// Path within the share (format: [context:]path).
        #[arg(value_name = "PATH")]
        path: String,

        /// Show detailed information (permissions, size, modification time).
        #[arg(short, long)]
        long: bool,

        /// Show hidden files (files starting with .).
        #[arg(short, long)]
        all: bool,

        /// Detect and report conflicts (files with same name but different content).
        #[arg(long)]
        conflicts: bool,

        /// Use hash comparison for conflict detection (slower but more accurate).
        #[arg(long, requires = "conflicts")]
        hash: bool,
    },

    /// Find files matching a pattern.
    #[command(after_help = "\
EXAMPLES:
    nofs find media:/ --name \"*.jpg\"           # Find all JPEG files
    nofs find media:/photos -t f --name \"*.png\" # Find PNG files only
    nofs find media:/ -t d --maxdepth 2          # Directories up to 2 levels deep
    nofs find media:/ --name \"**/backup/*\"      # Files in any backup folder

OPTIONS:
    --name <PATTERN>
        Glob pattern to match filenames. Supports * (any chars) and ** (any path).
    
    --type <TYPE>
        Filter by type: 'f' for files, 'd' for directories.
    
    --maxdepth <N>
        Limit directory traversal depth. 0 = only the starting directory.")]
    Find {
        /// Starting path within the share (format: [context:]path).
        #[arg(value_name = "PATH")]
        path: String,

        /// Filename pattern (glob syntax: *.txt, **/logs/*).
        #[arg(long, value_name = "PATTERN")]
        name: Option<String>,

        /// File type: 'f' for files, 'd' for directories.
        #[arg(long, value_name = "TYPE")]
        type_: Option<String>,

        /// Maximum directory traversal depth (0 = starting directory only).
        #[arg(long, value_name = "N")]
        maxdepth: Option<usize>,
    },

    /// Find which branch contains a file.
    #[command(
        alias = "where",
        after_help = "\
EXAMPLES:
    nofs which media:/photos/vacation.jpg        # Find branch containing file
    nofs which -a media:/docs/readme.txt         # Show all branches with file
    nofs which --conflicts media:/config.toml    # Check for conflicts

OUTPUT:
    Shows the branch path(s) containing the specified file.
    With --all, shows all branches that contain the file.

CONFLICT DETECTION:
    --conflicts
        Check if file exists in multiple branches with different content.
        Reports conflicts when file content differs between branches.
    
    --hash
        Use full file hash comparison for conflict detection.
        More accurate but slower. Requires --conflicts flag."
    )]
    Which {
        /// Path within the share (format: [context:]path).
        #[arg(value_name = "PATH")]
        path: String,

        /// Show all branches containing the file (not just the first).
        #[arg(short, long)]
        all: bool,

        /// Detect and report conflicts (files with same name but different content).
        #[arg(long)]
        conflicts: bool,

        /// Use hash comparison for conflict detection (slower but more accurate).
        #[arg(long, requires = "conflicts")]
        hash: bool,
    },

    /// Get the best branch path for creating a new file.
    #[command(after_help = "\
EXAMPLES:
    nofs create media:/newfile.txt              # Get path for new file
    nofs create media:/photos/vacation.jpg      # Get path in subdirectory

OUTPUT:
    Returns the full filesystem path where a new file should be created.
    Uses the configured branch selection policy (e.g., pfrd, mfs, rand).")]
    Create {
        /// Path within the share (format: [context:]path).
        #[arg(value_name = "PATH")]
        path: String,
    },

    /// Show filesystem statistics.
    #[command(after_help = "\
EXAMPLES:
    nofs stat media:/                  # Stats for entire share
    nofs stat -H media:/photos         # Human-readable sizes
    nofs stat media:/docs/report.pdf   # Stats for specific path

OUTPUT:
    Shows total size, free space, and file counts across all branches.
    With -H, sizes are shown in KB, MB, GB instead of bytes.")]
    Stat {
        /// Path within the share (defaults to root).
        #[arg(value_name = "PATH")]
        path: Option<String>,

        /// Show human-readable sizes (KB, MB, GB instead of bytes).
        #[arg(short = 'H', long)]
        human: bool,
    },

    /// Show share configuration and status.
    #[command(after_help = "\
EXAMPLES:
    nofs info media                    # Show media share config
    nofs info                          # Show all shares

OUTPUT:
    Displays share configuration including:
    - Branch paths and their types (RW, RO, NC)
    - Policy settings (create policy, minfreespace)
    - Branch status and statistics")]
    Info {
        /// Context name (optional, shows all shares if not specified).
        #[arg(value_name = "CONTEXT")]
        context: Option<String>,
    },

    /// Check if a file exists and return its location.
    #[command(after_help = "\
EXAMPLES:
    nofs exists media:/photos/pic.jpg      # Check if file exists
    nofs exists media:/docs/missing.txt    # Returns error if not found

EXIT CODES:
    0 - File exists
    1 - File does not exist

OUTPUT:
    Prints the branch path containing the file if it exists.")]
    Exists {
        /// Path within the share (format: [context:]path).
        #[arg(value_name = "PATH")]
        path: String,
    },

    /// Read file content (from first found branch).
    #[command(after_help = "\
EXAMPLES:
    nofs cat media:/config.toml            # Print file contents
    nofs cat media:/docs/readme.txt        # View text file

NOTES:
    Reads from the first branch containing the file.
    For binary files, output may not be readable in terminal.")]
    Cat {
        /// Path within the share (format: [context:]path).
        #[arg(value_name = "PATH")]
        path: String,
    },

    /// Copy files/directories (supports nofs context paths).
    #[command(after_help = "\
CONFLICT RESOLUTION OPTIONS:
    --file-over-file <STRATEGY>
            Handle file-over-file conflicts. Format: \"[CONDITIONS...] MODE\"

            MODE (required, default: delete-dest):
              skip          Skip copying the source file
              rename-src    Rename source file with _N suffix
              rename-dest   Rename destination file with _N suffix, then copy
              delete-src    Delete source file, skip copy
              delete-dest   Delete destination file, then copy source

            CONDITIONS (optional, checked before MODE):
              skip-hash          Skip if file hashes match
              skip-size          Skip if file sizes match
              skip-larger        Skip if source is larger than dest
              skip-smaller       Skip if source is smaller than dest
              skip-modified-newer Skip if source modified time is newer
              skip-modified-older Skip if source modified time is older
              skip-created-newer  Skip if source created time is newer
              skip-created-older  Skip if source created time is older
              delete-dest-hash   Delete dest if hashes match, then copy
              delete-dest-size   Delete dest if sizes match, then copy
              delete-dest-larger Delete dest if source is larger, then copy
              delete-dest-smaller Delete dest if source is smaller, then copy
              delete-dest-modified-newer Delete dest if src modified newer
              delete-dest-modified-older Delete dest if src modified older
              delete-dest-created-newer  Delete dest if src created newer
              delete-dest-created-older  Delete dest if src created older
              delete-src-hash    Delete src if hashes match, skip copy
              delete-src-size    Delete src if sizes match, skip copy
              delete-src-larger  Delete src if source is larger, skip copy
              delete-src-smaller Delete src if source is smaller, skip copy
              delete-src-modified-newer Delete src if src modified newer
              delete-src-modified-older Delete src if src modified older
              delete-src-created-newer  Delete src if src created newer
              delete-src-created-older  Delete src if src created older

            Examples:
              \"skip-hash\" - Skip if hashes match, otherwise delete-dest and copy
              \"delete-src-hash skip\" - Delete src if hashes match, else skip
              \"skip-size rename-dest\" - Skip if sizes match, else rename dest
              \"skip-modified-newer\" - Skip if source is modified newer
              \"delete-dest-created-older delete-dest\" - Delete dest if src created older

    --file-over-folder <MODE>
            Handle file-over-folder conflicts (default: merge)

            skip          Skip the file
            rename-src    Rename file and place beside folder
            rename-dest   Rename folder, place file at original path
            delete-src    Delete the source file
            delete-dest   Delete the folder, place file at original path
            merge         Place file inside the folder (folder/filename)

    --folder-over-file <MODE>
            Handle folder-over-file conflicts (default: merge)

            skip          Skip the folder
            rename-src    Rename folder to unique name
            rename-dest   Rename file to unique name, create folder
            delete-src    Delete the source folder
            delete-dest   Delete the file, create folder
            merge         Rename file, create folder (same as rename-dest)

FILTERING OPTIONS:
    -e, --ext <EXT>           Filter by file extensions (e.g., .mkv, .jpg)
    -E, --exclude <PATTERN>   Exclude files matching glob pattern
    -I, --include <PATTERN>   Include only files matching glob pattern
    -S, --size <SIZE>         Filter by file size (e.g., +5M, -10M)
    -l, --limit <N>           Limit number of files transferred
        --size-limit <SIZE>   Limit total size transferred (e.g., 100M, 1G)

PERFORMANCE OPTIONS:
    -j, --workers <N>     Number of parallel workers (default: 4)
    -n, --dry-run         Simulate without making changes")]
    Cp {
        /// Source paths [...] and destination (last argument).
        /// Format: [context:]path or regular path.
        #[arg(required = true, value_name = "PATHS")]
        paths: Vec<String>,

        // Conflict resolution options
        /// File-over-file conflict strategy.
        ///
        /// Format: "[CONDITIONS...] MODE" where MODE is one of:
        /// skip, rename-src, rename-dest, delete-src, delete-dest
        ///
        /// CONDITIONS: skip-hash, skip-size, skip-larger, skip-smaller,
        /// delete-dest-hash, delete-dest-size, delete-dest-larger, delete-dest-smaller,
        /// delete-src-hash, delete-src-size, delete-src-larger, delete-src-smaller
        ///
        /// Examples: "skip-hash", "delete-src-hash skip", "skip-size rename-dest"
        #[arg(
            long,
            default_value = "delete-src-hash rename-dest",
            value_name = "STRATEGY",
            help_heading = "Conflict Resolution"
        )]
        file_over_file: String,

        /// File-over-folder conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge
        #[arg(
            long,
            default_value = "merge",
            value_name = "MODE",
            help_heading = "Conflict Resolution"
        )]
        file_over_folder: String,

        /// Folder-over-file conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge
        #[arg(
            long,
            default_value = "merge",
            value_name = "MODE",
            help_heading = "Conflict Resolution"
        )]
        folder_over_file: String,

        // Performance options
        /// Simulate without making changes (dry-run)
        #[arg(short = 'n', long, alias = "simulate", help_heading = "Performance")]
        dry_run: bool,

        /// Number of parallel workers
        #[arg(
            short = 'j',
            long,
            default_value = "4",
            value_name = "N",
            help_heading = "Performance"
        )]
        workers: usize,

        // Filtering options
        /// Filter by file extensions (e.g., .mkv, .jpg)
        #[arg(short = 'e', long, value_name = "EXT", help_heading = "Filtering")]
        ext: Vec<String>,

        /// Exclude files matching glob pattern
        #[arg(short = 'E', long, value_name = "PATTERN", help_heading = "Filtering")]
        exclude: Vec<String>,

        /// Include only files matching glob pattern
        #[arg(short = 'I', long, value_name = "PATTERN", help_heading = "Filtering")]
        include: Vec<String>,

        /// Filter by file size (e.g., +5M, -10M)
        #[arg(short = 'S', long, value_name = "SIZE", help_heading = "Filtering")]
        size: Vec<String>,

        /// Limit number of files transferred
        #[arg(short = 'l', long, value_name = "N", help_heading = "Filtering")]
        limit: Option<u64>,

        /// Limit total size transferred (e.g., 100M, 1G)
        #[arg(long, value_name = "SIZE", help_heading = "Filtering")]
        size_limit: Option<String>,
    },

    /// Move files/directories (supports nofs context paths).
    #[command(after_help = "\
CONFLICT RESOLUTION OPTIONS:
    --file-over-file <STRATEGY>
            Handle file-over-file conflicts. Format: \"[CONDITIONS...] MODE\"

            MODE (required, default: delete-dest):
              skip          Skip moving the source file
              rename-src    Rename source file with _N suffix
              rename-dest   Rename destination file with _N suffix, then move
              delete-src    Delete source file, skip move
              delete-dest   Delete destination file, then move source

            CONDITIONS (optional, checked before MODE):
              skip-hash          Skip if file hashes match
              skip-size          Skip if file sizes match
              skip-larger        Skip if source is larger than dest
              skip-smaller       Skip if source is smaller than dest
              skip-modified-newer Skip if source modified time is newer
              skip-modified-older Skip if source modified time is older
              skip-created-newer  Skip if source created time is newer
              skip-created-older  Skip if source created time is older
              delete-dest-hash   Delete dest if hashes match, then move
              delete-dest-size   Delete dest if sizes match, then move
              delete-dest-larger Delete dest if source is larger, then move
              delete-dest-smaller Delete dest if source is smaller, then move
              delete-dest-modified-newer Delete dest if src modified newer
              delete-dest-modified-older Delete dest if src modified older
              delete-dest-created-newer  Delete dest if src created newer
              delete-dest-created-older  Delete dest if src created older
              delete-src-hash    Delete src if hashes match, skip move
              delete-src-size    Delete src if sizes match, skip move
              delete-src-larger  Delete src if source is larger, skip move
              delete-src-smaller Delete src if source is smaller, skip move
              delete-src-modified-newer Delete src if src modified newer
              delete-src-modified-older Delete src if src modified older
              delete-src-created-newer  Delete src if src created newer
              delete-src-created-older  Delete src if src created older

            Examples:
              \"skip-hash\" - Skip if hashes match, otherwise delete-dest and move
              \"delete-src-hash skip\" - Delete src if hashes match, else skip
              \"skip-size rename-dest\" - Skip if sizes match, else rename dest
              \"skip-modified-newer\" - Skip if source is modified newer
              \"delete-dest-created-older delete-dest\" - Delete dest if src created older

    --file-over-folder <MODE>
            Handle file-over-folder conflicts (default: merge)

            skip          Skip the file
            rename-src    Rename file and place beside folder
            rename-dest   Rename folder, place file at original path
            delete-src    Delete the source file
            delete-dest   Delete the folder, place file at original path
            merge         Place file inside the folder (folder/filename)

    --folder-over-file <MODE>
            Handle folder-over-file conflicts (default: merge)
            
            skip          Skip the folder
            rename-src    Rename folder to unique name
            rename-dest   Rename file to unique name, create folder
            delete-src    Delete the source folder
            delete-dest   Delete the file, create folder
            merge         Rename file, create folder (same as rename-dest)

FILTERING OPTIONS:
    -e, --ext <EXT>           Filter by file extensions (e.g., .mkv, .jpg)
    -E, --exclude <PATTERN>   Exclude files matching glob pattern
    -I, --include <PATTERN>   Include only files matching glob pattern
    -S, --size <SIZE>         Filter by file size (e.g., +5M, -10M)
    -l, --limit <N>           Limit number of files moved
        --size-limit <SIZE>   Limit total size moved (e.g., 100M, 1G)

PERFORMANCE OPTIONS:
    -j, --workers <N>     Number of parallel workers (default: 4)
    -n, --dry-run         Simulate without making changes")]
    Mv {
        /// Source paths [...] and destination (last argument).
        /// Format: [context:]path or regular path.
        #[arg(required = true, value_name = "PATHS")]
        paths: Vec<String>,

        // Conflict resolution options
        /// File-over-file conflict strategy.
        ///
        /// Format: "[CONDITIONS...] MODE" where MODE is one of:
        /// skip, rename-src, rename-dest, delete-src, delete-dest
        ///
        /// CONDITIONS: skip-hash, skip-size, skip-larger, skip-smaller,
        /// delete-dest-hash, delete-dest-size, delete-dest-larger, delete-dest-smaller,
        /// delete-src-hash, delete-src-size, delete-src-larger, delete-src-smaller
        ///
        /// Examples: "skip-hash", "delete-src-hash skip", "skip-size rename-dest"
        #[arg(
            long,
            default_value = "delete-src-hash rename-dest",
            value_name = "STRATEGY",
            help_heading = "Conflict Resolution"
        )]
        file_over_file: String,

        /// File-over-folder conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge
        #[arg(
            long,
            default_value = "merge",
            value_name = "MODE",
            help_heading = "Conflict Resolution"
        )]
        file_over_folder: String,

        /// Folder-over-file conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge
        #[arg(
            long,
            default_value = "merge",
            value_name = "MODE",
            help_heading = "Conflict Resolution"
        )]
        folder_over_file: String,

        // Performance options
        /// Simulate without making changes (dry-run)
        #[arg(short = 'n', long, alias = "simulate", help_heading = "Performance")]
        dry_run: bool,

        /// Number of parallel workers
        #[arg(
            short = 'j',
            long,
            default_value = "4",
            value_name = "N",
            help_heading = "Performance"
        )]
        workers: usize,

        // Filtering options
        /// Filter by file extensions (e.g., .mkv, .jpg)
        #[arg(short = 'e', long, value_name = "EXT", help_heading = "Filtering")]
        ext: Vec<String>,

        /// Exclude files matching glob pattern
        #[arg(short = 'E', long, value_name = "PATTERN", help_heading = "Filtering")]
        exclude: Vec<String>,

        /// Include only files matching glob pattern
        #[arg(short = 'I', long, value_name = "PATTERN", help_heading = "Filtering")]
        include: Vec<String>,

        /// Filter by file size (e.g., +5M, -10M)
        #[arg(short = 'S', long, value_name = "SIZE", help_heading = "Filtering")]
        size: Vec<String>,

        /// Limit number of files moved
        #[arg(short = 'l', long, value_name = "N", help_heading = "Filtering")]
        limit: Option<u64>,

        /// Limit total size moved (e.g., 100M, 1G)
        #[arg(long, value_name = "SIZE", help_heading = "Filtering")]
        size_limit: Option<String>,
    },

    /// Remove files or directories.
    #[command(after_help = "\
EXAMPLES:
    nofs rm media:/temp.txt                  # Remove a file
    nofs rm -r media:/old_folder             # Remove folder and contents
    nofs rm -rv media:/logs/*.log            # Verbose removal of multiple files

WARNING:
    Recursive removal (-r) deletes directories and ALL their contents.
    This operation cannot be undone. Use --verbose to see what will be deleted.")]
    Rm {
        /// Path(s) within the share (format: [context:]path). Supports glob patterns.
        #[arg(required = true, value_name = "PATHS")]
        paths: Vec<String>,

        /// Remove directories and their contents recursively.
        #[arg(short, long)]
        recursive: bool,

        /// Print each file/directory as it is removed.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Create directories.
    #[command(after_help = "\
EXAMPLES:
    nofs mkdir media:/new_folder             # Create single directory
    nofs mkdir -p media:/a/b/c               # Create nested directories
    nofs mkdir -pv media:/photos/2024        # Verbose creation

NOTES:
    Without -p, fails if parent directories don't exist.
    With -p, creates all missing parent directories.")]
    Mkdir {
        /// Path within the share (format: [context:]path).
        #[arg(value_name = "PATH")]
        path: String,

        /// Create parent directories as needed.
        #[arg(short, long)]
        parents: bool,

        /// Print each directory as it is created.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Remove empty directories.
    #[command(after_help = "\
EXAMPLES:
    nofs rmdir media:/empty_folder           # Remove empty directory
    nofs rmdir -v media:/old_logs            # Verbose removal

NOTES:
    Only works on empty directories. Use 'rm -r' for non-empty directories.")]
    Rmdir {
        /// Path within the share (format: [context:]path).
        #[arg(value_name = "PATH")]
        path: String,

        /// Print the directory as it is removed.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Create or update files.
    #[command(after_help = "\
EXAMPLES:
    nofs touch media:/newfile.txt            # Create empty file
    nofs touch media:/existing.txt           # Update modification time
    nofs touch -v media:/data.log            # Verbose creation

NOTES:
    Creates an empty file if it doesn't exist.
    Updates the modification time if the file already exists.")]
    Touch {
        /// Path within the share (format: [context:]path).
        #[arg(value_name = "PATH")]
        path: String,

        /// Print the file path after creation/update.
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show disk usage (recursive directory size calculation).
    #[command(after_help = "\
EXAMPLES:
    nofs du media:/                  # Show disk usage for entire share
    nofs du -H media:/photos         # Human-readable sizes
    nofs du -a media:/docs           # Show all subdirectory sizes
    nofs du --maxdepth 1 media:/     # Only show top-level directories

OUTPUT:
    Shows disk usage for the specified path across all branches.
    With -a, shows sizes for all subdirectories.
    With -H, sizes are shown in KB, MB, GB instead of bytes.")]
    Du {
        /// Path within the share (format: [context:]path).
        #[arg(value_name = "PATH")]
        path: String,

        /// Show human-readable sizes (KB, MB, GB instead of bytes).
        #[arg(short = 'H', long)]
        human: bool,

        /// Show all subdirectory sizes.
        #[arg(short, long)]
        all: bool,

        /// Maximum directory traversal depth (0 = starting directory only).
        #[arg(long, value_name = "N")]
        maxdepth: Option<usize>,
    },

    /// Generate shell completion scripts.
    ///
    /// Usage: nofs completions <SHELL> > completions.<shell>
    ///
    /// Supported shells: bash, zsh, fish, elvish, powershell
    Completions {
        /// Shell type (bash, zsh, fish, elvish, powershell)
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },

    /// Generate man page.
    ///
    /// Usage: nofs manpage > nofs.1
    Manpage,
}

#[allow(clippy::too_many_lines)]
fn main() -> Result<()> {
    let cli = Cli::parse();

    // Handle commands that don't require config initialization
    match cli.command {
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            clap_complete::generate(shell, &mut Cli::command(), "nofs", &mut std::io::stdout());
            return Ok(());
        }
        Commands::Manpage => {
            use clap::CommandFactory;
            let man = clap_mangen::Man::new(Cli::command());
            man.render(&mut std::io::stdout())
                .map_err(|e| anyhow::anyhow!("Failed to render man page: {e}"))?;
            return Ok(());
        }
        Commands::Ls { .. }
        | Commands::Find { .. }
        | Commands::Which { .. }
        | Commands::Create { .. }
        | Commands::Stat { .. }
        | Commands::Info { .. }
        | Commands::Exists { .. }
        | Commands::Cat { .. }
        | Commands::Cp { .. }
        | Commands::Mv { .. }
        | Commands::Rm { .. }
        | Commands::Mkdir { .. }
        | Commands::Rmdir { .. }
        | Commands::Touch { .. }
        | Commands::Du { .. } => {}
    }

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
        Commands::Ls {
            path,
            long,
            all,
            conflicts,
            hash,
        } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::ls::execute(
                pool,
                pool_path,
                long,
                all,
                cli.verbose,
                conflicts,
                hash,
                cli.json,
            )?;
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
                cli.json,
            )?;
        }
        Commands::Which {
            path,
            all,
            conflicts,
            hash,
        } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::which::execute(pool, pool_path, all, cli.verbose, conflicts, hash, cli.json)?;
        }
        Commands::Create { path } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::create::execute(pool, pool_path, cli.verbose, cli.json)?;
        }
        Commands::Stat { path, human } => {
            let pool = if let Some(p) = &path {
                let (pool, _) = pool_mgr.resolve_context_path(p)?;
                pool
            } else {
                pool_mgr.default_pool()?
            };
            commands::stat::execute(pool, human, cli.verbose, cli.json)?;
        }
        Commands::Info { context } => {
            if let Some(ctx) = &context {
                let pool = pool_mgr.get_pool(ctx)?;
                commands::info::execute_single(pool, cli.verbose, cli.json)?;
            } else {
                commands::info::execute_all(&pool_mgr, cli.verbose, cli.json)?;
            }
        }
        Commands::Exists { path } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::exists::execute(pool, pool_path, cli.verbose, cli.json)?;
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

            let stats = commands::cp::execute(sources, destination, &config, share)?;
            if stats.errors.load(std::sync::atomic::Ordering::Relaxed) > 0 {
                return Err(anyhow::anyhow!("Some copy operations failed"));
            }
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

            let stats = commands::mv::execute(
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

            if stats.errors.load(std::sync::atomic::Ordering::Relaxed) > 0 {
                return Err(anyhow::anyhow!("Some move operations failed"));
            }
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
        Commands::Du {
            path,
            human,
            all,
            maxdepth,
        } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&path)?;
            commands::du::execute(pool, pool_path, human, maxdepth, all, cli.json)?;
        }
        // These commands are handled earlier and don't reach here
        Commands::Completions { .. } | Commands::Manpage => unreachable!(),
    }

    Ok(())
}

/// Parse folder conflict mode from string
///
/// # Errors
///
/// Returns an error if the mode string is not recognized.
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

/// Parse human-readable size string to bytes
///
/// # Errors
///
/// Returns an error if the size string cannot be parsed.
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
