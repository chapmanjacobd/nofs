# nofs

A lightweight union filesystem tool

## Overview

**nofs** provides mergerfs-like functionality for pooling multiple filesystems/directories into a unified view, but operates entirely in userspace via subcommands rather than as a FUSE filesystem. This makes it simpler, faster to query, and easier to integrate into scripts.

## Features

- No FUSE required - Pure userspace tool, no kernel module needed
- Named union contexts - Simple TOML config with `[union.name]` sections
- Context:path syntax - `nofs ls media:/movies` selects the `media` context
- Ad-hoc mode - Use directly from command line without config
- Policy-based branch selection - Choose branches based on free space, randomness, or path preservation
- Verbose mode - See decision steps with `-v` flag
- POSIX-like commands - Familiar interface (`ls`, `find`, `where`, etc.)

## Installation

```bash
cargo build --release
sudo cp target/release/nofs /usr/local/bin/
```

## Quick Start

### Configuration File

Create `/etc/nofs/config.toml`:

```toml
[union.media]
paths = ["/mnt/hdd1/media", "/mnt/hdd2/media", "/mnt/hdd3/media"]
modes = ["RW", "RW", "RO"]  # Optional: last branch is read-only
create_policy = "pfrd"       # percentage free random distribution
search_policy = "ff"         # first found
minfreespace = "4G"

[union.backup]
paths = ["/mnt/backup1", "/mnt/backup2"]
create_policy = "mfs"        # most free space
minfreespace = "10G"

[union.scratch]
paths = ["/tmp/a", "/tmp/b"]
create_policy = "rand"       # random selection
```

### Usage with Contexts

```bash
# List directory from specific union context
nofs ls media:/movies

# Find which branch contains a file
nofs -v where media:/movies/blade_runner.mkv
# Output (stderr):
#   selected:
#     /mnt/hdd1/media/movies/blade_runner.mkv (first-found policy)

# Get best branch for creating a new file
nofs -v create media:/new_movie.mkv
# Output (stderr):
#   selected:
#     /mnt/hdd2/media/new_movie.mkv (pfrd policy)

# Find files matching a pattern
nofs find media:/ --name "*.mkv"

# Show filesystem statistics
nofs stat -H

# Show all union contexts
nofs info

# Show specific context
nofs info media
```

### Ad-hoc Mode (No Config)

```bash
# Quick union of directories
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

### `where` - Find File Location

```bash
nofs [OPTIONS] where [context:]path

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

### `info` - Pool Information

```bash
nofs info [context]

Shows all union contexts, or specific context if named.
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

### Search Policies

| Policy | Description |
|--------|-------------|
| **ff** | First found (default) |
| **all** | All branches |

## Configuration Options

### Union Context Settings

```toml
[union.name]
paths = ["/path1", "/path2"]      # Required: branch paths
modes = ["RW", "RO"]               # Optional: branch modes (parallel to paths)
create_policy = "pfrd"             # Policy for create operations
search_policy = "ff"               # Policy for search operations  
action_policy = "epall"            # Policy for action operations
minfreespace = "4G"                # Minimum free space threshold
```

### Branch Modes

- **RW** (Read/Write) - Full participation in all operations (default)
- **RO** (Read-Only) - Excluded from create and action operations
- **NC** (No-Create) - Can read and modify, but not create new files

## Examples

### Media Server Setup

```toml
[union.movies]
paths = ["/hdd1/movies", "/hdd2/movies", "/hdd3/movies"]
modes = ["RW", "RW", "RO"]
create_policy = "pfrd"

[union.tv]
paths = ["/hdd1/tv", "/hdd2/tv"]
create_policy = "mfs"
```

```bash
# List movies across all drives
nofs ls movies:/

# Find specific movie
nofs where movies:/scifi/blade_runner.mkv

# Add new movie (automatically selects best branch)
nofs create movies:/new_release.mkv
```

### SSD Cache Setup

```toml
[union.fast]
paths = ["/nvme/cache", "/hdd/storage"]
modes = ["RW", "NC"]  # HDD can read/modify but not create
create_policy = "lfs"  # Fill SSD first (least free space)
```

### Verbose Debugging

```bash
$ nofs -v where media:/movie.mkv
selected:
  /mnt/hdd1/media/movie.mkv (first-found policy)
/mnt/hdd1/media/movie.mkv

$ nofs -v ls media:/movies
found in:
  /mnt/hdd1/media/movies
  /mnt/hdd2/media/movies
file1.mkv
file2.mkv
```

## Comparison with mergerfs

| Feature | mergerfs | nofs |
|---------|----------|------|
| FUSE-based | Yes | No |
| Mount point | Yes | No |
| Config file | Optional | Optional |
| Named contexts | No | Yes |
| Ad-hoc usage | Limited | Full |
| Verbose mode | No | Yes |
| File creation | Transparent | Via `create` command |
| File access | Direct | Via subcommands |
| Performance | Near-native | Subprocess overhead |
| Complexity | Higher | Lower |

## When to Use nofs

**Good fit:**
- Scripting and automation
- Querying file locations
- Batch operations across branches
- Simple pooling without FUSE complexity
- Integration with existing tools
- Multiple independent unions (contexts)

**Consider mergerfs instead:**
- Need transparent filesystem access
- Require POSIX filesystem semantics
- Want applications to see unified pool automatically

## License

MIT License

## Contributing

Contributions welcome! Please feel free to submit issues and pull requests.
