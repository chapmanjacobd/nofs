//! nofs - A lightweight shared filesystem tool
//!
//! Provides mergerfs-like functionality without FUSE.
//! All operations happen via subcommands with optional TOML configuration.

// Centralized lint suppressions for non-test code
// These are needed throughout the codebase and are centralized here
// to avoid scattering allow attributes across multiple files
#![allow(
    // Documentation lints - not all functions need full documentation
    clippy::missing_panics_doc,
    clippy::missing_errors_doc,
)]

pub mod branch;
pub mod cache;
pub mod commands;
pub mod config;
pub mod conflict;
pub mod error;
pub mod output;
pub mod policy;
pub mod pool;
pub mod utils;

use clap::Parser;
use error::{NofsError, Result};
use std::path::Path;

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
    #[arg(long, global = true, help_heading = "Configuration", num_args = 1)]
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
#[derive(clap::Subcommand, Debug, Clone)]
enum Commands {
    /// List directory contents (like ls).
    #[command(after_help = "\
EXAMPLES:
    nofs ls media:/                    # List root of media share
    nofs ls -l media:/photos           # Detailed listing
    nofs ls --conflicts media:/docs    # Detect conflicting files
    nofs ls --conflicts --hash media:/ # Use hash for conflict detection
    nofs ls dir1/ dir2/                # List multiple directories

CONFLICT DETECTION:
    --conflicts
        Detect files with the same name but different content across branches.
        Files are marked in output when conflicts are found.

    --hash
        Use full file hash comparison instead of size/mtime for conflict detection.
        More accurate but slower on large files. Requires --conflicts flag.")]
    Ls {
        /// Path(s) within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATHS")]
        ls_paths: Vec<String>,

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
    nofs find dir1/ dir2/ --name \"*.log\"        # Find in multiple directories

OPTIONS:
    --name <PATTERN>
        Glob pattern to match filenames. Supports * (any chars) and ** (any path).

    --type <TYPE>
        Filter by type: 'f' for files, 'd' for directories.

    --maxdepth <N>
        Limit directory traversal depth. 0 = only the starting directory.")]
    Find {
        /// Starting path(s) within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATHS")]
        find_paths: Vec<String>,

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
    nofs which -a dump/image/ dump/video/        # Multiple paths, all matches

OUTPUT:
    Shows the branch path(s) containing the specified file(s).
    With --all, shows all branches that contain each file.
    Multiple paths can be specified; without --all, only the first match
    is shown per path following the normal policy.

CONFLICT DETECTION:
    --conflicts
        Check if file exists in multiple branches with different content.
        Reports conflicts when file content differs between branches.

    --hash
        Use full file hash comparison for conflict detection.
        More accurate but slower. Requires --conflicts flag."
    )]
    Which {
        /// Path(s) within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATHS")]
        which_paths: Vec<String>,

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
    nofs create file1.txt file2.txt             # Get paths for multiple files

OUTPUT:
    Returns the full filesystem path where a new file should be created.
    Uses the configured branch selection policy (e.g., pfrd, mfs, rand).")]
    Create {
        /// Path(s) within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATHS")]
        create_paths: Vec<String>,
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
        #[arg(value_name = "PATHS")]
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
    nofs exists file1.txt file2.txt        # Check multiple files

EXIT CODES:
    0 - File exists
    1 - File does not exist

OUTPUT:
    Prints the branch path containing the file if it exists.")]
    Exists {
        /// Path(s) within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATHS")]
        exists_paths: Vec<String>,
    },

    /// Read file content (from first found branch).
    #[command(after_help = "\
EXAMPLES:
    nofs cat media:/config.toml            # Print file contents
    nofs cat media:/docs/readme.txt        # View text file
    nofs cat file1.txt file2.txt           # View multiple files

NOTES:
    Reads from the first branch containing the file.
    For binary files, output may not be readable in terminal.")]
    Cat {
        /// Path(s) within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATHS")]
        cat_paths: Vec<String>,
    },

    /// Show differences between branches.
    #[command(after_help = "\
EXAMPLES:
    nofs diff media:/                    # Show all conflicting files
    nofs diff media:/config.toml         # Check specific file for conflicts
    nofs diff -H media:/                 # Use hash comparison (more accurate)
    nofs diff --json media:/             # Output in JSON format
    nofs diff -v media:/                 # Verbose output with timestamps

OUTPUT:
    Shows files that exist in multiple branches with different content.
    For directories, lists all conflicting files.
    For single files, shows detailed comparison across branches.

OPTIONS:
    -H, --hash
        Use full file hash comparison instead of size/mtime.
        More accurate but slower on large files.

    --json
        Output in JSON format for scripting/automation.

    -v, --verbose
        Show detailed information including timestamps and hashes.")]
    Diff {
        /// Path within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATH")]
        diff_path: String,

        /// Use hash comparison for conflict detection (slower but more accurate).
        #[arg(short = 'H', long)]
        hash: bool,

        /// Verbose output (show timestamps and hashes).
        #[arg(short, long)]
        verbose: bool,
    },

    /// Compare files byte-by-byte.
    #[command(after_help = "\
EXAMPLES:
    nofs cmp media:/config.toml            # Compare file across first two branches
    nofs cmp -v media:/data.bin            # Verbose output if identical

OUTPUT:
    Compares the same file across two branches.
    By default, compares first two branches containing the file.
    Exit code 0 if files are identical, 1 if they differ.")]
    Cmp {
        /// Path within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATH")]
        cmp_path: String,

        /// Verbose output (print message if files are identical).
        #[arg(short, long)]
        verbose: bool,
    },

    /// Show disk free space (df-like output).
    #[command(after_help = "\
EXAMPLES:
    nofs df                                # Show all branches
    nofs df media                          # Show branches for media share
    nofs df -H                             # Human-readable sizes
    nofs df -T                             # Include total

OUTPUT:
    Shows disk space usage for each branch in a standard df-like format.")]
    Df {
        /// Context/share name (optional, shows all if not specified).
        #[arg(value_name = "CONTEXT")]
        context: Option<String>,

        /// Show human-readable sizes (K, M, G).
        #[arg(short = 'H', long)]
        human: bool,

        /// Show total for all branches.
        #[arg(short = 'T', long)]
        total: bool,
    },

    /// Search file contents across all branches (grep).
    #[command(after_help = "\
EXAMPLES:
    nofs grep media:/ error                # Search for 'error' in media:/
    nofs grep -r media:/ \"TODO\"            # Recursive search
    nofs grep -i media:/ warning           # Case-insensitive search
    nofs grep -l media:/ config            # Show only filenames
    nofs grep -v media:/ debug             # Invert match (lines without 'debug')
    nofs grep --json media:/ pattern       # JSON output

OPTIONS:
    -i, --ignore-case       Case-insensitive search
    -v, --invert-match      Invert match (show non-matching lines)
    -n, --line-number       Show line numbers
    -l, --files-with-matches  Show only filenames
    -r, --recursive         Search recursively in directories")]
    Grep {
        /// Pattern to search for.
        #[arg(required = true, value_name = "PATTERN")]
        pattern: String,

        /// Path(s) to search within (format: [context:]path).
        #[arg(required = true, value_name = "PATHS")]
        grep_paths: Vec<String>,

        /// Case-insensitive search.
        #[arg(short = 'i', long)]
        ignore_case: bool,

        /// Invert match (show non-matching lines).
        #[arg(short = 'v', long)]
        invert_match: bool,

        /// Show line numbers.
        #[arg(short = 'n', long)]
        line_number: bool,

        /// Show only filenames with matches.
        #[arg(short = 'l', long)]
        files_with_matches: bool,

        /// Recursive search (for directories).
        #[arg(short = 'r', long)]
        recursive: bool,
    },

    /// Show directory tree structure.
    #[command(after_help = "\
EXAMPLES:
    nofs tree media:/                        # Show tree view
    nofs tree -a media:/photos               # Show all branches for each file
    nofs tree -d media:/                     # Directories only
    nofs tree -H media:/                     # Human-readable file sizes
    nofs tree --max-depth 2 media:/          # Limit depth

OPTIONS:
    -a, --all-branches      Show which branches contain each file
    -d, --directories       Show directories only
    -f, --files             Show files only
    -H, --human             Human-readable file sizes
    --max-depth <N>         Maximum depth to display")]
    Tree {
        /// Path within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATH")]
        tree_path: String,

        /// Show all branches for each file.
        #[arg(short = 'a', long)]
        all_branches: bool,

        /// Maximum depth to display.
        #[arg(long, value_name = "N")]
        max_depth: Option<usize>,

        /// Show directories only.
        #[arg(short = 'd', long)]
        directories: bool,

        /// Show files only.
        #[arg(short = 'f', long)]
        files: bool,

        /// Human-readable file sizes.
        #[arg(short = 'H', long)]
        human: bool,
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
              skip-hash                    Skip if file hashes match
              skip-size                    Skip if file sizes match
              skip-larger                  Skip if source is larger than dest
              skip-smaller                 Skip if source is smaller than dest
              skip-modified-newer          Skip if source modified time is newer
              skip-modified-older          Skip if source modified time is older
              skip-created-newer           Skip if source created time is newer
              skip-created-older           Skip if source created time is older
              delete-dest-hash             Delete dest if hashes match, then copy
              delete-dest-size             Delete dest if sizes match, then copy
              delete-dest-larger           Delete dest if source is larger, then copy
              delete-dest-smaller          Delete dest if source is smaller, then copy
              delete-dest-modified-newer   Delete dest if src modified newer
              delete-dest-modified-older   Delete dest if src modified older
              delete-dest-created-newer    Delete dest if src created newer
              delete-dest-created-older    Delete dest if src created older
              delete-src-hash              Delete src if hashes match, skip copy
              delete-src-size              Delete src if sizes match, skip copy
              delete-src-larger            Delete src if source is larger, skip copy
              delete-src-smaller           Delete src if source is smaller, skip copy
              delete-src-modified-newer    Delete src if src modified newer
              delete-src-modified-older    Delete src if src modified older
              delete-src-created-newer     Delete src if src created newer
              delete-src-created-older     Delete src if src created older

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
        cp_paths: Vec<String>,

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

        /// Minimum file size to include (e.g., 5M, 1G)
        #[arg(long, value_name = "SIZE", value_parser = policy::parse_size, help_heading = "Filtering")]
        min_size: Option<u64>,

        /// Maximum file size to include (e.g., 10M, 2G)
        #[arg(long, value_name = "SIZE", value_parser = policy::parse_size, help_heading = "Filtering")]
        max_size: Option<u64>,

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
              skip                      Skip moving the source file
              rename-src                Rename source file with _N suffix
              rename-dest               Rename destination file with _N suffix, then move
              delete-src                Delete source file, skip move
              delete-dest               Delete destination file, then move source

            CONDITIONS (optional, checked before MODE):
              skip-hash                 Skip if file hashes match
              skip-size                 Skip if file sizes match
              skip-larger               Skip if source is larger than dest
              skip-smaller              Skip if source is smaller than dest
              skip-modified-newer       Skip if source modified time is newer
              skip-modified-older       Skip if source modified time is older
              skip-created-newer        Skip if source created time is newer
              skip-created-older        Skip if source created time is older
              delete-dest-hash          Delete dest if hashes match, then move
              delete-dest-size          Delete dest if sizes match, then move
              delete-dest-larger        Delete dest if source is larger, then move
              delete-dest-smaller       Delete dest if source is smaller, then move
              delete-dest-modified-newer  Delete dest if src modified newer
              delete-dest-modified-older  Delete dest if src modified older
              delete-dest-created-newer   Delete dest if src created newer
              delete-dest-created-older   Delete dest if src created older
              delete-src-hash           Delete src if hashes match, skip move
              delete-src-size           Delete src if sizes match, skip move
              delete-src-larger         Delete src if source is larger, skip move
              delete-src-smaller        Delete src if source is smaller, skip move
              delete-src-modified-newer   Delete src if src modified newer
              delete-src-modified-older   Delete src if src modified older
              delete-src-created-newer    Delete src if src created newer
              delete-src-created-older    Delete src if src created older

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
        mv_paths: Vec<String>,

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

        /// Minimum file size to include (e.g., 5M, 1G)
        #[arg(long, value_name = "SIZE", value_parser = policy::parse_size, help_heading = "Filtering")]
        min_size: Option<u64>,

        /// Maximum file size to include (e.g., 10M, 2G)
        #[arg(long, value_name = "SIZE", value_parser = policy::parse_size, help_heading = "Filtering")]
        max_size: Option<u64>,

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
        rm_paths: Vec<String>,

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
    nofs mkdir dir1/ dir2/ dir3/             # Create multiple directories

NOTES:
    Without -p, fails if parent directories don't exist.
    With -p, creates all missing parent directories.")]
    Mkdir {
        /// Path(s) within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATHS")]
        mkdir_paths: Vec<String>,

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
    nofs rmdir empty1/ empty2/               # Remove multiple empty directories

NOTES:
    Only works on empty directories. Use 'rm -r' for non-empty directories.")]
    Rmdir {
        /// Path(s) within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATHS")]
        rmdir_paths: Vec<String>,

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
    nofs touch file1.txt file2.txt           # Touch multiple files

NOTES:
    Creates an empty file if it doesn't exist.
    Updates the modification time if the file already exists.")]
    Touch {
        /// Path(s) within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATHS")]
        touch_paths: Vec<String>,

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
    nofs du dir1/ dir2/              # Show disk usage for multiple directories

OUTPUT:
    Shows disk usage for the specified path across all branches.
    With -a, shows sizes for all subdirectories.
    With -H, sizes are shown in KB, MB, GB instead of bytes.")]
    Du {
        /// Path(s) within the share (format: [context:]path).
        #[arg(required = true, value_name = "PATHS")]
        du_paths: Vec<String>,

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

    /// Generate man pages.
    ///
    /// Usage:
    ///   nofs manpage                  # Generate all man pages to ./man/
    ///   nofs manpage --subcommand ls  # Generate only nofs-ls.1
    ///   nofs manpage > nofs.1         # Generate main page to stdout
    Manpage {
        /// Generate man page for a specific subcommand
        #[arg(long, value_name = "SUBCOMMAND")]
        subcommand: Option<String>,

        /// Output directory for man pages (default: ./man/)
        #[arg(short, long, default_value = "man", value_name = "DIR")]
        outdir: String,
    },
}

/// Run the nofs command line application.
///
/// # Errors
/// Returns an error if the command fails, invalid arguments are provided, or any file system operation fails.
pub fn run() -> Result<()> {
    let cli = Cli::parse();

    // Handle commands that don't require config initialization
    if handle_early_commands(&cli)? {
        return Ok(());
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

    // Execute the main commands
    run_main_commands(&cli, &pool_mgr)
}

/// Handle early commands (completions, manpage) that don't need config
/// Returns true if command was handled and should return early
fn handle_early_commands(cli: &Cli) -> Result<bool> {
    match &cli.command {
        Commands::Completions { shell } => {
            use clap::CommandFactory;
            clap_complete::generate(*shell, &mut Cli::command(), "nofs", &mut std::io::stdout());
            Ok(true)
        }
        Commands::Manpage { subcommand, outdir } => {
            handle_manpage_command(subcommand.as_ref(), outdir)?;
            Ok(true)
        }
        Commands::Ls { .. } | Commands::Find { .. } | Commands::Which { .. } | Commands::Create { .. } |
        Commands::Stat { .. } | Commands::Info { .. } | Commands::Exists { .. } | Commands::Cat { .. } |
        Commands::Diff { .. } | Commands::Cmp { .. } | Commands::Df { .. } | Commands::Grep { .. } |
        Commands::Tree { .. } | Commands::Cp { .. } | Commands::Mv { .. } | Commands::Rm { .. } |
        Commands::Mkdir { .. } | Commands::Rmdir { .. } | Commands::Touch { .. } | Commands::Du { .. } => Ok(false),
    }
}

/// Handle manpage generation command
fn handle_manpage_command(subcommand: Option<&String>, outdir: &str) -> Result<()> {
    use std::fs;
    use std::path::Path;

    // Create output directory if it doesn't exist
    let outdir_path = Path::new(outdir);
    fs::create_dir_all(outdir_path)
        .map_err(|e| NofsError::Command(format!("Failed to create output directory {outdir}: {e}")))?;

    if let Some(subcmd_name) = subcommand {
        generate_single_man_page(subcmd_name, outdir_path)?;
    } else {
        generate_all_man_pages(outdir_path)?;
    }
    Ok(())
}

/// Generate man page for a single subcommand
fn generate_single_man_page(subcmd_name: &str, outdir_path: &Path) -> Result<()> {
    use clap::CommandFactory;
    use std::fs;

    let cmd = Cli::command();
    let subcmd = cmd
        .get_subcommands()
        .find(|c| c.get_name() == subcmd_name)
        .ok_or_else(|| NofsError::Command(format!("Unknown subcommand: {subcmd_name}")))?;
    let man = clap_mangen::Man::new(subcmd.clone());
    let filename = format!("nofs-{subcmd_name}.1");
    let filepath = outdir_path.join(&filename);
    let mut file = fs::File::create(&filepath)
        .map_err(|e| NofsError::Command(format!("Failed to create {filename}: {e}")))?;
    man.render(&mut file)
        .map_err(|e| NofsError::Command(format!("Failed to render man page for {subcmd_name}: {e}")))?;
    eprintln!("Generated: {filename}");
    Ok(())
}

/// Generate man pages for all subcommands
fn generate_all_man_pages(outdir_path: &Path) -> Result<()> {
    use clap::CommandFactory;
    use std::fs;

    let cmd = Cli::command();

    // Generate main page
    let main_man = clap_mangen::Man::new(cmd.clone());
    let main_filename = "nofs.1";
    let main_filepath = outdir_path.join(main_filename);
    let mut main_file = fs::File::create(&main_filepath)
        .map_err(|e| NofsError::Command(format!("Failed to create {main_filename}: {e}")))?;
    main_man
        .render(&mut main_file)
        .map_err(|e| NofsError::Command(format!("Failed to render main man page: {e}")))?;
    eprintln!("Generated: {main_filename}");

    // Generate subcommand pages
    for subcmd in cmd.get_subcommands() {
        let subcmd_name = subcmd.get_name();

        // Skip certain internal subcommands
        if subcmd_name == "help" || subcmd_name == "completions" || subcmd_name == "manpage" {
            continue;
        }

        generate_single_man_page(subcmd_name, outdir_path)?;
    }
    Ok(())
}

/// Run main commands that require config initialization
fn run_main_commands(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let command = cli.command.clone();
    match command {
        Commands::Ls { .. } | Commands::Find { .. } | Commands::Which { .. } => {
            run_query_commands(cli, pool_mgr)
        }
        Commands::Create { .. } | Commands::Exists { .. } | Commands::Cat { .. } => {
            run_simple_path_commands(cli, pool_mgr)
        }
        Commands::Stat { .. } => run_stat_command(cli, pool_mgr),
        Commands::Info { .. } => run_info_command(cli, pool_mgr),
        Commands::Diff { .. } | Commands::Cmp { .. } => run_diff_commands(cli, pool_mgr),
        Commands::Df { .. } => run_df_command(cli, pool_mgr),
        Commands::Grep { .. } => run_grep_command(cli, pool_mgr),
        Commands::Tree { .. } => run_tree_command(cli, pool_mgr),
        Commands::Cp { .. } => run_copy_command(cli, pool_mgr),
        Commands::Mv { .. } => run_move_command(cli, pool_mgr),
        Commands::Rm { .. } => run_remove_command(cli, pool_mgr),
        Commands::Mkdir { .. } => run_mkdir_command(cli, pool_mgr),
        Commands::Rmdir { .. } => run_rmdir_command(cli, pool_mgr),
        Commands::Touch { .. } => run_touch_command(cli, pool_mgr),
        Commands::Du { .. } => run_du_command(cli, pool_mgr),
        Commands::Completions { .. } | Commands::Manpage { .. } => unreachable!(),
    }
}

/// Run query commands (ls, find, which)
#[allow(clippy::wildcard_enum_match_arm)]
fn run_query_commands(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let command = cli.command.clone();
    match command {
        Commands::Ls {
            ls_paths,
            long,
            all,
            conflicts,
            hash,
        } => {
            for path in &ls_paths {
                let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
                commands::ls::execute(
                    pool,
                    pool_path,
                    &commands::ls::LsOptions {
                        long,
                        all,
                        verbose: cli.verbose,
                        conflicts,
                        hash,
                        json: cli.json,
                    },
                )?;
            }
        }
        Commands::Find {
            find_paths,
            name,
            type_,
            maxdepth,
        } => {
            for path in &find_paths {
                let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
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
        }
        Commands::Which {
            which_paths,
            all,
            conflicts,
            hash,
        } => {
            for path in &which_paths {
                let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
                commands::which::execute(pool, pool_path, commands::which::WhichOptions { all, verbose: cli.verbose, conflicts, hash, json: cli.json })?;
            }
        }
        #[allow(clippy::wildcard_enum_match_arm)]
        _ => {}
    }
    Ok(())
}

/// Run simple path commands (create, exists, cat)
#[allow(clippy::wildcard_enum_match_arm)]
fn run_simple_path_commands(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let command = cli.command.clone();
    match command {
        Commands::Create { create_paths } => {
            for path in &create_paths {
                let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
                commands::create::execute(pool, pool_path, cli.verbose, cli.json)?;
            }
        }
        Commands::Exists { exists_paths } => {
            for path in &exists_paths {
                let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
                commands::exists::execute(pool, pool_path, cli.verbose, cli.json)?;
            }
        }
        Commands::Cat { cat_paths } => {
            for path in &cat_paths {
                let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
                commands::cat::execute(pool, pool_path, cli.verbose)?;
            }
        }
        #[allow(clippy::wildcard_enum_match_arm)]
        _ => {}
    }
    Ok(())
}

/// Run stat command
fn run_stat_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Stat { path, human } = cli.command.clone() else { return Ok(()) };
    let pool = if let Some(p) = &path {
        let (pool, _) = pool_mgr.resolve_context_path(p)?;
        pool
    } else {
        pool_mgr.default_pool()?
    };
    commands::stat::execute(pool, commands::stat::StatOptions { human, verbose: cli.verbose, json: cli.json })?;
    Ok(())
}

/// Run info command
fn run_info_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Info { context } = cli.command.clone() else { return Ok(()) };
    if let Some(ctx) = &context {
        let pool = pool_mgr.get_pool(ctx)?;
        commands::info::execute_single(pool, cli.verbose, cli.json)?;
    } else {
        commands::info::execute_all(pool_mgr, cli.verbose, cli.json)?;
    }
    Ok(())
}

/// Run diff commands (diff, cmp)
#[allow(clippy::wildcard_enum_match_arm)]
fn run_diff_commands(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let command = cli.command.clone();
    match command {
        Commands::Diff {
            diff_path,
            hash,
            verbose,
        } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&diff_path)?;
            commands::diff::execute(pool, pool_path, commands::diff::DiffOptions { verbose, hash, json: cli.json })?;
        }
        Commands::Cmp { cmp_path, verbose } => {
            let (pool, pool_path) = pool_mgr.resolve_context_path(&cmp_path)?;
            commands::cmp::execute(
                pool,
                pool_path,
                &commands::cmp::CmpOptions {
                    branch1_name: None,
                    branch2_name: None,
                    verbose,
                    json: cli.json,
                },
            )?;
        }
        #[allow(clippy::wildcard_enum_match_arm)]
        _ => {}
    }
    Ok(())
}

/// Run df command
fn run_df_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Df { context, human, total } = cli.command.clone() else { return Ok(()) };
    commands::df::execute(
        pool_mgr,
        context.as_deref(),
        &commands::df::DfOptions {
            human,
            total,
            verbose: cli.verbose,
            json: cli.json,
        },
    )?;
    Ok(())
}

/// Run grep command
fn run_grep_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Grep {
        pattern,
        grep_paths,
        ignore_case,
        invert_match,
        line_number,
        files_with_matches,
        recursive,
    } = cli.command.clone()
    else {
        return Ok(());
    };
    for path in &grep_paths {
        let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
        commands::grep::execute(
            pool,
            pool_path,
            &pattern,
            &commands::grep::GrepOptions {
                ignore_case,
                invert_match,
                line_numbers: line_number,
                files_with_matches,
                recursive,
                verbose: cli.verbose,
                json: cli.json,
            },
        )?;
    }
    Ok(())
}

/// Run tree command
fn run_tree_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Tree {
        tree_path,
        all_branches,
        max_depth,
        directories,
        files,
        human,
    } = cli.command.clone()
    else {
        return Ok(());
    };
    let (pool, pool_path) = pool_mgr.resolve_context_path(&tree_path)?;
    commands::tree::execute(
        pool,
        pool_path,
        commands::tree::TreeOptions {
            all_branches,
            max_depth,
            directories_only: directories,
            files_only: files,
            human_size: human,
            verbose: cli.verbose,
            json: cli.json,
        },
    )?;
    Ok(())
}

/// Run copy command
fn run_copy_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Cp {
        cp_paths,
        file_over_file,
        file_over_folder,
        folder_over_file,
        dry_run,
        workers,
        ext,
        exclude,
        include,
        min_size,
        max_size,
        limit,
        size_limit,
    } = cli.command.clone()
    else {
        return Ok(());
    };
    // Parse sources and destination
    let Some((destination, sources)) = cp_paths.split_last() else {
        return Err(NofsError::Config(
            "At least one source and one destination are required".to_string(),
        ));
    };

    // Parse size limit
    let parsed_size_limit = size_limit.as_ref().and_then(|s| parse_size(s).ok());

    // Build SizeFilter if min or max size is provided
    let parsed_size = (min_size.is_some() || max_size.is_some()).then_some(commands::cp::SizeFilter {
        min: min_size,
        max: max_size,
    });

    // Get share for context-aware paths
    let share = extract_share_from_paths(pool_mgr, sources, destination);

    let config = commands::cp::CopyConfig {
        is_copy: true,
        dry_run,
        workers,
        verbose: cli.verbose,
        file_over_file: commands::cp::parse_file_over_file(&file_over_file)?,
        file_over_folder: commands::cp::parse_folder_conflict_mode(&file_over_folder)?,
        folder_over_file: commands::cp::parse_folder_conflict_mode(&folder_over_file)?,
        extensions: ext,
        exclude,
        include,
        limit,
        size_limit: parsed_size_limit,
        size: parsed_size,
    };

    let stats = commands::cp::execute(sources, destination, &config, share)?;
    if stats.errors.load(std::sync::atomic::Ordering::Relaxed) > 0 {
        return Err(NofsError::Command("Some copy operations failed".to_string()));
    }
    Ok(())
}

/// Run move command
fn run_move_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Mv {
        mv_paths,
        file_over_file,
        file_over_folder,
        folder_over_file,
        dry_run,
        workers,
        ext,
        exclude,
        include,
        min_size,
        max_size,
        limit,
        size_limit,
    } = cli.command.clone()
    else {
        return Ok(());
    };
    // Parse sources and destination
    let Some((destination, sources)) = mv_paths.split_last() else {
        return Err(NofsError::CopyMove(
            "At least one source and one destination are required".to_string(),
        ));
    };

    // Parse size limit
    let parsed_size_limit = size_limit.as_ref().and_then(|s| parse_size(s).ok());

    // Build SizeFilter if min or max size is provided
    let parsed_size = (min_size.is_some() || max_size.is_some()).then_some(commands::cp::SizeFilter {
        min: min_size,
        max: max_size,
    });

    // Get share for context-aware paths
    let share = extract_share_from_paths(pool_mgr, sources, destination);

    let config = commands::mv::MoveConfig {
        sources,
        destination,
        file_over_file: &file_over_file,
        file_over_folder: &file_over_folder,
        folder_over_file: &folder_over_file,
        simulate: dry_run,
        workers,
        verbose: cli.verbose,
        extensions: ext,
        exclude,
        include,
        limit,
        size_limit: parsed_size_limit,
        size: parsed_size,
        share,
    };

    let stats = commands::mv::execute(&config)?;

    if stats.errors.load(std::sync::atomic::Ordering::Relaxed) > 0 {
        return Err(NofsError::Command("Some move operations failed".to_string()));
    }
    Ok(())
}

/// Run remove command
fn run_remove_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Rm {
        rm_paths,
        recursive,
        verbose,
    } = cli.command.clone()
    else {
        return Ok(());
    };
    let mut any_failed = false;
    for path in &rm_paths {
        let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
        if let Err(e) = commands::rm::execute(pool, pool_path, recursive, verbose || cli.verbose) {
            eprintln!("nofs: {e}");
            any_failed = true;
        }
    }
    if any_failed {
        return Err(NofsError::Command("Some removal operations failed".to_string()));
    }
    Ok(())
}

/// Run mkdir command
fn run_mkdir_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Mkdir {
        mkdir_paths,
        parents,
        verbose,
    } = cli.command.clone()
    else {
        return Ok(());
    };
    for path in &mkdir_paths {
        let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
        commands::mkdir::execute(pool, pool_path, parents, verbose || cli.verbose)?;
    }
    Ok(())
}

/// Run rmdir command
fn run_rmdir_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Rmdir { rmdir_paths, verbose } = cli.command.clone() else { return Ok(()) };
    for path in &rmdir_paths {
        let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
        commands::rmdir::execute(pool, pool_path, verbose || cli.verbose)?;
    }
    Ok(())
}

/// Run touch command
fn run_touch_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Touch { touch_paths, verbose } = cli.command.clone() else { return Ok(()) };
    for path in &touch_paths {
        let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
        commands::touch::execute(pool, pool_path, verbose || cli.verbose)?;
    }
    Ok(())
}

/// Run du command
fn run_du_command(cli: &Cli, pool_mgr: &pool::PoolManager) -> Result<()> {
    let Commands::Du {
        du_paths,
        human,
        all,
        maxdepth,
    } = cli.command.clone()
    else {
        return Ok(());
    };
    for path in &du_paths {
        let (pool, pool_path) = pool_mgr.resolve_context_path(path)?;
        commands::du::execute(pool, pool_path, commands::du::DuOptions { human, all, json: cli.json, verbose: cli.verbose }, maxdepth)?;
    }
    Ok(())
}

/// Parse a size string (e.g., "1K", "1.5M", "2G") into bytes
///
/// # Errors
///
/// Returns an error if the size string cannot be parsed.
fn parse_size(s: &str) -> Result<u64> {
    use crate::policy::parse_size as policy_parse_size;
    policy_parse_size(s)
}

/// Try to extract a share from paths that contain context prefixes
fn extract_share_from_paths<'a>(
    pool_mgr: &'a pool::PoolManager,
    sources: &[String],
    destination: &str,
) -> Option<&'a pool::Pool> {
    // Check if any path has a context prefix
    for path in sources.iter().chain(std::iter::once(&destination.to_string())) {
        let parsed = utils::parse_path_with_context(path);

        // Skip if no colon or UNC path
        if parsed.has_no_colon || parsed.is_unc {
            continue;
        }

        // Try to find a pool that matches the prefix
        for pool in pool_mgr.pools().values() {
            if parsed.matches_share(&pool.name) {
                return Some(pool);
            }
        }
    }

    None
}
