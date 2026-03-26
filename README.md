# nofs

A lightweight union filesystem tool - a mergerfs alternative without FUSE.

## Overview

**nofs** provides mergerfs-like functionality for pooling multiple filesystems/directories into a unified view, but operates entirely in userspace via subcommands rather than as a FUSE filesystem. This makes it simpler, faster to query, and easier to integrate into scripts.

## Features

- **No FUSE required** - Pure userspace tool, no kernel module needed
- **TOML configuration** - Optional config file for persistent pool definitions
- **Ad-hoc mode** - Use directly from command line without config
- **Policy-based branch selection** - Choose branches based on free space, randomness, or path preservation
- **Fast lookups** - Query which branch contains a file instantly
- **POSIX-like commands** - Familiar interface (`ls`, `find`, `stat`, etc.)

## Installation

```bash
cargo build --release
sudo cp target/release/nofs /usr/local/bin/
```

## Quick Start

### Ad-hoc Usage (No Config)

```bash
# List directory contents from pooled branches
nofs --paths /mnt/hdd1,/mnt/hdd2,/mnt/hdd3 ls /media

# Find which branch contains a file
nofs --paths /mnt/hdd1,/mnt/hdd2 where /media/movie.mkv

# Get best branch for creating a new file
nofs --paths /mnt/hdd1,/mnt/hdd2 --policy mfs create /media/newfile.txt

# Find files matching a pattern
nofs --paths /mnt/hdd1,/mnt/hdd2 find /media --name "*.mkv"

# Show filesystem statistics
nofs --paths /mnt/hdd1,/mnt/hdd2 stat --human
```

### With Configuration File

Create `/etc/nofs/config.toml`:

```toml
[[pools]]
name = "media"
mountpoint = "/mnt/pool"
create_policy = "pfrd"
search_policy = "ff"
minfreespace = "4G"

[[pools.branches]]
path = "/mnt/hdd1"
mode = "RW"

[[pools.branches]]
path = "/mnt/hdd2"
mode = "RW"

[[pools.branches]]
path = "/mnt/backup"
mode = "RO"  # Read-only
```

Then use:

```bash
nofs --config /etc/nofs/config.toml ls /media
nofs where /media/movie.mkv
nofs stat --human
```

## Commands

### `ls` - List Directory Contents

```bash
nofs ls [OPTIONS] <PATH>

OPTIONS:
    -l, --long     Show detailed information
    -a, --all      Show hidden files
```

### `find` - Find Files

```bash
nofs find [OPTIONS] <PATH>

OPTIONS:
    --name <PATTERN>     Filename pattern (glob)
    --type <TYPE>        File type: f=file, d=directory
    --maxdepth <N>       Maximum depth
```

### `where` - Find File Location

```bash
nofs where [OPTIONS] <PATH>

OPTIONS:
    -a, --all    Show all branches containing the file
```

### `create` - Get Create Path

```bash
nofs create [OPTIONS] <PATH>

Returns the full path on the best branch for creating a new file.
```

### `stat` - Filesystem Statistics

```bash
nofs stat [OPTIONS] [PATH]

OPTIONS:
    -h, --human    Show human-readable sizes
```

### `info` - Pool Information

```bash
nofs info

Shows pool configuration and status.
```

### `exists` - Check File Existence

```bash
nofs exists <PATH>

Returns exit code 0 if file exists, 1 otherwise.
```

### `cat` - Read File Content

```bash
nofs cat <PATH>

Reads file content from first found branch.
```

## Policies

### Create Policies

- **pfrd** - Percentage free random distribution (default)
  - Selects branches weighted by available space
- **mfs** - Most free space
  - Selects branch with most available space
- **ff** - First found
  - Selects first eligible branch
- **rand** - Random
  - Selects random eligible branch
- **lfs** - Least free space
  - Selects branch with least available space
- **lus** - Least used space
  - Selects branch with least used space
- **lup** - Least used percentage
  - Selects branch with lowest usage percentage
- **epmfs** - Existing path, most free space
  - Path-preserving variant of mfs
- **epff** - Existing path, first found
  - Path-preserving variant of ff

### Search Policies

- **ff** - First found (default)
- **all** - All branches

### Action Policies

- **epall** - Existing path, all (default)
- **all** - All branches

## Branch Modes

- **RW** (Read/Write) - Full participation in all operations (default)
- **RO** (Read-Only) - Excluded from create and action operations
- **NC** (No-Create) - Can read and modify, but not create new files

## Configuration Options

### Pool Settings

- `name` - Pool identifier
- `mountpoint` - Virtual mount point path
- `create_policy` - Default policy for create operations
- `search_policy` - Default policy for search operations
- `action_policy` - Default policy for action operations
- `minfreespace` - Minimum free space threshold (e.g., "4G", "100M")

### Branch Settings

- `path` - Branch filesystem path
- `mode` - Branch mode (RW, RO, NC)
- `minfreespace` - Per-branch minimum free space override

## Examples

### Basic Media Pool

```bash
# Create pool with 3 HDDs
nofs --paths /mnt/hdd1,/mnt/hdd2,/mnt/hdd3 ls /

# Find movies across all drives
nofs --paths /mnt/hdd1,/mnt/hdd2,/mnt/hdd3 find / --name "*.mkv"

# Check where a file is stored
nofs --paths /mnt/hdd1,/mnt/hdd2,/mnt/hdd3 where /movies/blade_runner.mkv
```

### With Read-Only Backup Branch

```bash
# Include read-only backup drive
nofs --paths /mnt/hdd1=RW,/mnt/hdd2=RW,/mnt/backup=RO stat

# Create will only use RW branches
nofs --paths /mnt/hdd1=RW,/mnt/hdd2=RW,/mnt/backup=RO create /newfile.txt
```

### SSD Cache Setup

```bash
# Prefer SSD for new files
nofs --paths /mnt/ssd,/mnt/hdd1,/mnt/hdd2 --policy mfs create /data/newfile.txt

# But search all branches
nofs --paths /mnt/ssd,/mnt/hdd1,/mnt/hdd2 find /data --name "*.log"
```

## Comparison with mergerfs

| Feature | mergerfs | nofs |
|---------|----------|------|
| FUSE-based | Yes | No |
| Mount point | Yes | No |
| Config file | Optional | Optional |
| Ad-hoc usage | Limited | Full |
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

**Consider mergerfs instead:**
- Need transparent filesystem access
- Require POSIX filesystem semantics
- Want applications to see unified pool automatically

## License

MIT License

## Contributing

Contributions welcome! Please feel free to submit issues and pull requests.
