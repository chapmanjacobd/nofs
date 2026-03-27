# nofs

A lightweight [shared filesystem](https://en.wikipedia.org/wiki/Clustered_file_system#Shared-disk_file_system) tool

## Overview

**nofs** provides mergerfs-like functionality for combining multiple filesystems/directories into a unified view, but operates entirely in userspace via subcommands rather than as a FUSE filesystem. This makes it simpler, faster to get started, and easier to integrate into scripts.

## Features

- No FUSE: pure userspace tool
- share-path syntax: `nofs ls media:/movies` lists files in the `media` share
- Ad-hoc: Use directly from the command line without any config
- Policy-based branch selection: Choose branches based on free space, randomness, or path preservation
- POSIX-like commands: Familiar interface (`ls`, `find`, `which`, `cp`, `mv`, `rm`, etc.)
- Parallel operations: `cp` and `mv` support multi-threaded transfers
- Conflict resolution: Flexible strategies for file/folder conflicts during copy/move

## Installation

```bash
cargo install nofs
```

(or manually)

```bash
git clone https://github.com/chapmanjacobd/nofs
cd nofs
cargo build --release
sudo cp target/release/nofs /usr/local/bin/
```

## Examples

### SSD Cache Setup

```toml
# ~/.config/nofs/config.toml
[share.fast]
paths = ["/nvme/cache"]
nc_paths = ["/hdd/storage"]  # HDD can read/modify but not create
create_policy = "lfs"  # Fill SSD first (least free space)
```

### Media Server Setup

```toml
# /etc/nofs/config.toml
[share.movies]
paths = ["/hdd1/movies", "/hdd2/movies"]
ro_paths = ["/hdd3/movies"]  # Read-only backup
create_policy = "pfrd"

[share.tv]
paths = ["/hdd1/tv", "/hdd2/tv"]
create_policy = "mfs"
```

## Policies

### Create Policies

| Policy | Description |
|--------|-------------|
| **pfrd** | Percentage free random distribution (default) - weighted by available space |
| **mfs** | Most free space |
| **ff** | First found (first in list) |
| **rand** | Random selection |
| **lfs** | Least free space |
| **lus** | Least used space |
| **lup** | Least used percentage |
| **epmfs** | Existing path, most free space (path-preserving) |
| **epff** | Existing path, first found (path-preserving) |
| **eprand** | Existing path, random selection (path-preserving) |
| **epall** | Existing path, all branches (path-preserving) |
| **all** | All branches |

### Search Policies

| Policy | Description |
|--------|-------------|
| **ff** | First found (default) |
| **all** | All branches |

## Configuration Options

### Share Settings

```toml
# .nofs.toml
[share.name]
paths = ["/path1", "/path2"]       # Required: RW branch paths
ro_paths = ["/path3"]              # Optional: read-only branches
nc_paths = ["/path4"]              # Optional: no-create branches
create_policy = "pfrd"             # Policy for create operations
search_policy = "ff"               # Policy for search operations
action_policy = "epall"            # Policy for action operations
minfreespace = "4G"                # Minimum free space threshold
```

### Branch Modes

- **RW** (Read/Write) - Full participation in all operations (default)
- **RO** (Read-Only) - Excluded from create and action operations
- **NC** (No-Create) - Can read and modify, but not create new files

#### Ad-hoc Branch Mode Syntax

In ad-hoc mode, specify modes with `=` syntax:

```bash
nofs --paths /mnt/hdd1=RW,/mnt/hdd2=RW,/mnt/backup=RO ls /
```

### Quick Examples

```bash
# List files in a share
nofs ls media:/movies

# Find which branch contains a file
nofs -v which media:/movies/big_buck_bunny.mkv
# Output (stderr):
#   selected:
#     /mnt/hdd1/media/movies/big_buck_bunny.mkv (first-found policy)

# Get best branch for creating a new file
nofs -v create media:/new_movie.mkv
# Output (stderr):
#   selected:
#     /mnt/hdd2/media/new_movie.mkv (pfrd policy)

# Find files matching a pattern
nofs find media:/ --name "*.mkv"

# Show filesystem statistics
nofs stat -H

# Show all shares
nofs info

# Show specific share
nofs info media

# Copy files with parallel workers
nofs cp -j 8 media:/movies/ /backup/movies/

# Move files with extension filter
nofs mv -e .mkv media:/old/ media:/new/

# Create directory with parents
nofs mkdir -p media:/new/scifi/2024/
```

### Ad-hoc Mode (No Config)

```bash
# Quick share of directories
nofs --paths /mnt/hdd1,/mnt/hdd2,/mnt/hdd3 ls /media

# With branch modes
nofs --paths /mnt/hdd1=RW,/mnt/hdd2=RW,/mnt/backup=RO ls /

# With custom policy
nofs --paths /mnt/ssd,/mnt/hdd --policy mfs create /data/newfile.txt
```

## Commands

### `ls` - List Directory Contents

```bash
nofs [OPTIONS] ls [context:]path

OPTIONS:
    -l, --long     Show detailed information
    -a, --all      Show hidden files
    -v, --verbose  Show which branches contain the directory
```

### `find` - Find Files

```bash
nofs [OPTIONS] find [context:]path

OPTIONS:
    --name <PATTERN>     Filename pattern (glob)
    --type <TYPE>        File type: f=file, d=directory
    --maxdepth <N>       Maximum depth
    -v, --verbose        Show which branches are searched
```

### `which` - Find File Location

```bash
nofs [OPTIONS] which [context:]path

OPTIONS:
    -a, --all      Show all branches containing the file
    -v, --verbose  Show selection decision
```

### `create` - Get Create Path

```bash
nofs [OPTIONS] create [context:]path

Returns the full path on the best branch for creating a new file.
Use -v to see which policy was used.
```

### `stat` - Filesystem Statistics

```bash
nofs [OPTIONS] stat [context:]path

OPTIONS:
    -H, --human    Show human-readable sizes
```

### `info` - Share Information

```bash
nofs info [context]

Shows all shares, or specific share if named.
```

### `exists` - Check File Existence

```bash
nofs exists [context:]path

Returns exit code 0 if file exists, 1 otherwise.
Prints location to stdout.
```

### `cat` - Read File Content

```bash
nofs cat [context:]path

Reads file content from first found branch.
```

### `cp` - Copy Files/Directories

```bash
nofs [OPTIONS] cp [SOURCE...] DEST

SOURCES and DEST can be regular paths or nofs context paths (e.g., media:/movies).

OPTIONS:
    --file-over-file <STRATEGY>    File-over-file conflict strategy (default: "delete-src-hash rename-dest")
    --file-over-folder <STRATEGY>  File-over-folder conflict strategy (default: "merge")
    --folder-over-file <STRATEGY>  Folder-over-file conflict strategy (default: "merge")
    -n, --dry-run                  Simulate without making changes
    -j, --workers <N>              Number of parallel workers (default: 4)
    -e, --ext <EXT>                Filter by file extensions (e.g., .mkv, .jpg)
    -E, --exclude <PATTERN>        Exclude patterns (glob)
    -I, --include <PATTERN>        Include patterns (glob)
    -S, --size <SIZE>              Filter by file size (e.g., +5M, -10M)
    -l, --limit <N>                Limit number of files transferred
    --size-limit <SIZE>            Limit total size transferred (e.g., 100M, 1G)
    -v, --verbose                  Verbose output

Conflict strategies:
    skip, skip-hash, rename-src, rename-dest, delete-src, delete-dest, delete-src-hash
```

### `mv` - Move Files/Directories

```bash
nofs [OPTIONS] mv [SOURCE...] DEST

Same options as `cp`, but moves files instead of copying.
```

### `rm` - Remove Files/Directories

```bash
nofs [OPTIONS] rm [PATH...]

OPTIONS:
    -r, --recursive    Remove directories and their contents recursively
    -v, --verbose      Verbose output
```

### `mkdir` - Create Directories

```bash
nofs [OPTIONS] mkdir [context:]path

OPTIONS:
    -p, --parents    Create parent directories as needed
    -v, --verbose    Verbose output
```

### `rmdir` - Remove Empty Directories

```bash
nofs [OPTIONS] rmdir [context:]path

OPTIONS:
    -v, --verbose    Verbose output
```

### `touch` - Create or Update Files

```bash
nofs [OPTIONS] touch [context:]path

OPTIONS:
    -v, --verbose    Verbose output
```

### `du` - Show Disk Usage

```bash
nofs [OPTIONS] du [context:]path

OPTIONS:
    -H, --human          Show human-readable sizes (KB, MB, GB)
    -a, --all            Show all subdirectory sizes
    --maxdepth <N>       Maximum directory traversal depth
    -v, --verbose        Verbose output

EXAMPLES:
    nofs du media:/                  # Show disk usage for entire share
    nofs du -H media:/photos         # Human-readable sizes
    nofs du -a media:/docs           # Show all subdirectory sizes
    nofs du --maxdepth 1 media:/     # Only show top-level directories
```

## Comparison with mergerfs

| Feature | mergerfs | nofs |
|---------|----------|------|
| Requires root | Yes | No |
| Mount required | Yes | No |
| Mountpoint access | Yes | No |
| FUSE-based | Yes | No |
| /etc/fstab support | Yes | No |
| Config changes require remount | Yes | No |
| Works in containers | Difficult | Yes |
| File creation | Transparent | Via subcommands |
| File access | Direct | Via subcommands |
| Performance | Near-native | Subprocess overhead |

### When to Use nofs

**Good fit:**
- Scripting and automation
- Batch operations across branches
- A bit simpler and easier to understand

**Consider mergerfs instead:**
- Need transparent filesystem access
- Require POSIX filesystem semantics
- Want applications to see unified shares automatically
- Many more features!
