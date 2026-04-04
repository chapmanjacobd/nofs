#compdef nofs

autoload -U is-at-least

_nofs() {
    typeset -A opt_args
    typeset -a _arguments_options
    local ret=1

    if is-at-least 5.2; then
        _arguments_options=(-s -S -C)
    else
        _arguments_options=(-s -C)
    fi

    local context curcontext="$curcontext" state line
    _arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
":: :_nofs_commands" \
"*::: :->nofs" \
&& ret=0
    case $state in
    (nofs)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:nofs-command-$line[1]:"
        case $line[1] in
            (ls)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-l[Show detailed information (permissions, size, modification time)]' \
'--long[Show detailed information (permissions, size, modification time)]' \
'-a[Show hidden files (files starting with .)]' \
'--all[Show hidden files (files starting with .)]' \
'--conflicts[Detect and report conflicts (files with same name but different content)]' \
'--hash[Use hash comparison for conflict detection (slower but more accurate)]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::ls_paths -- Path(s) within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(find)
_arguments "${_arguments_options[@]}" : \
'--name=[Filename pattern (glob syntax\: *.txt, **/logs/*)]:PATTERN:_default' \
'--type=[File type\: '\''f'\'' for files, '\''d'\'' for directories]:TYPE:_default' \
'--maxdepth=[Maximum directory traversal depth (0 = starting directory only)]:N:_default' \
'--min-siblings=[Minimum number of siblings (entries in the same directory) to include]:N:_default' \
'--max-siblings=[Maximum number of siblings (entries in the same directory) to include. Folders with more entries than this will be skipped during traversal]:N:_default' \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::find_paths -- Starting path(s) within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(which)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-a[Show all branches containing the file (not just the first)]' \
'--all[Show all branches containing the file (not just the first)]' \
'--conflicts[Detect and report conflicts (files with same name but different content)]' \
'--hash[Use hash comparison for conflict detection (slower but more accurate)]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::which_paths -- Path(s) within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(create)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::create_paths -- Path(s) within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(stat)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-H[Show human-readable sizes (KB, MB, GB instead of bytes)]' \
'--human[Show human-readable sizes (KB, MB, GB instead of bytes)]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'::path -- Path within the share (defaults to root):_default' \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'::context -- Context name (optional, shows all shares if not specified):_default' \
&& ret=0
;;
(exists)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::exists_paths -- Path(s) within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(cat)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::cat_paths -- Path(s) within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(diff)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-H[Use hash comparison for conflict detection (slower but more accurate)]' \
'--hash[Use hash comparison for conflict detection (slower but more accurate)]' \
'-v[Verbose output (show timestamps and hashes)]' \
'--verbose[Verbose output (show timestamps and hashes)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
':diff_path -- Path within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(cmp)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Verbose output (print message if files are identical)]' \
'--verbose[Verbose output (print message if files are identical)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
':cmp_path -- Path within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(df)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-H[Show human-readable sizes (K, M, G)]' \
'--human[Show human-readable sizes (K, M, G)]' \
'-T[Show total for all branches]' \
'--total[Show total for all branches]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'::context -- Context/share name (optional, shows all if not specified):_default' \
&& ret=0
;;
(grep)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-i[Case-insensitive search]' \
'--ignore-case[Case-insensitive search]' \
'--invert-match[Invert match (show non-matching lines)]' \
'-n[Show line numbers]' \
'--line-number[Show line numbers]' \
'-l[Show only filenames with matches]' \
'--files-with-matches[Show only filenames with matches]' \
'-r[Recursive search (for directories)]' \
'--recursive[Recursive search (for directories)]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
':pattern -- Pattern to search for:_default' \
'*::grep_paths -- Path(s) to search within (format\: \[context\:\]path):_default' \
&& ret=0
;;
(tree)
_arguments "${_arguments_options[@]}" : \
'--max-depth=[Maximum depth to display]:N:_default' \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-a[Show all branches for each file]' \
'--all-branches[Show all branches for each file]' \
'-d[Show directories only]' \
'--directories[Show directories only]' \
'-f[Show files only]' \
'--files[Show files only]' \
'-H[Human-readable file sizes]' \
'--human[Human-readable file sizes]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
':tree_path -- Path within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(cp)
_arguments "${_arguments_options[@]}" : \
'--file-over-file=[File-over-file conflict strategy]:STRATEGY:_default' \
'--file-over-folder=[File-over-folder conflict strategy\: skip, rename-src, rename-dest, delete-src, delete-dest, merge]:MODE:_default' \
'--folder-over-file=[Folder-over-file conflict strategy\: skip, rename-src, rename-dest, delete-src, delete-dest, merge]:MODE:_default' \
'-j+[Number of parallel workers]:N:_default' \
'--workers=[Number of parallel workers]:N:_default' \
'*-e+[Filter by file extensions (e.g., .mkv, .jpg)]:EXT:_default' \
'*--ext=[Filter by file extensions (e.g., .mkv, .jpg)]:EXT:_default' \
'*-E+[Exclude files matching glob pattern]:PATTERN:_default' \
'*--exclude=[Exclude files matching glob pattern]:PATTERN:_default' \
'*-I+[Include only files matching glob pattern]:PATTERN:_default' \
'*--include=[Include only files matching glob pattern]:PATTERN:_default' \
'--min-size=[Minimum file size to include (e.g., 5M, 1G)]:SIZE:_default' \
'--max-size=[Maximum file size to include (e.g., 10M, 2G)]:SIZE:_default' \
'-l+[Limit number of files transferred]:N:_default' \
'--limit=[Limit number of files transferred]:N:_default' \
'--size-limit=[Limit total size transferred (e.g., 100M, 1G)]:SIZE:_default' \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-n[Simulate without making changes (dry-run)]' \
'--dry-run[Simulate without making changes (dry-run)]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
'-V[Print version]' \
'--version[Print version]' \
'*::cp_paths -- Source paths \[...\] and destination (last argument). Format\: \[context\:\]path or regular path:_default' \
&& ret=0
;;
(mv)
_arguments "${_arguments_options[@]}" : \
'--file-over-file=[File-over-file conflict strategy]:STRATEGY:_default' \
'--file-over-folder=[File-over-folder conflict strategy\: skip, rename-src, rename-dest, delete-src, delete-dest, merge]:MODE:_default' \
'--folder-over-file=[Folder-over-file conflict strategy\: skip, rename-src, rename-dest, delete-src, delete-dest, merge]:MODE:_default' \
'-j+[Number of parallel workers]:N:_default' \
'--workers=[Number of parallel workers]:N:_default' \
'*-e+[Filter by file extensions (e.g., .mkv, .jpg)]:EXT:_default' \
'*--ext=[Filter by file extensions (e.g., .mkv, .jpg)]:EXT:_default' \
'*-E+[Exclude files matching glob pattern]:PATTERN:_default' \
'*--exclude=[Exclude files matching glob pattern]:PATTERN:_default' \
'*-I+[Include only files matching glob pattern]:PATTERN:_default' \
'*--include=[Include only files matching glob pattern]:PATTERN:_default' \
'--min-size=[Minimum file size to include (e.g., 5M, 1G)]:SIZE:_default' \
'--max-size=[Maximum file size to include (e.g., 10M, 2G)]:SIZE:_default' \
'-l+[Limit number of files moved]:N:_default' \
'--limit=[Limit number of files moved]:N:_default' \
'--size-limit=[Limit total size moved (e.g., 100M, 1G)]:SIZE:_default' \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-n[Simulate without making changes (dry-run)]' \
'--dry-run[Simulate without making changes (dry-run)]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
'-V[Print version]' \
'--version[Print version]' \
'*::mv_paths -- Source paths \[...\] and destination (last argument). Format\: \[context\:\]path or regular path:_default' \
&& ret=0
;;
(rm)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-r[Remove directories and their contents recursively]' \
'--recursive[Remove directories and their contents recursively]' \
'-v[Print each file/directory as it is removed]' \
'--verbose[Print each file/directory as it is removed]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::rm_paths -- Path(s) within the share (format\: \[context\:\]path). Supports glob patterns:_default' \
&& ret=0
;;
(mkdir)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-p[Create parent directories as needed]' \
'--parents[Create parent directories as needed]' \
'-v[Print each directory as it is created]' \
'--verbose[Print each directory as it is created]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::mkdir_paths -- Path(s) within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(rmdir)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Print the directory as it is removed]' \
'--verbose[Print the directory as it is removed]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::rmdir_paths -- Path(s) within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(touch)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Print the file path after creation/update]' \
'--verbose[Print the file path after creation/update]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::touch_paths -- Path(s) within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(du)
_arguments "${_arguments_options[@]}" : \
'--maxdepth=[Maximum directory traversal depth (0 = starting directory only)]:N:_default' \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-H[Show human-readable sizes (KB, MB, GB instead of bytes)]' \
'--human[Show human-readable sizes (KB, MB, GB instead of bytes)]' \
'-a[Show all subdirectory sizes]' \
'--all[Show all subdirectory sizes]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::du_paths -- Path(s) within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(completions)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
'-V[Print version]' \
'--version[Print version]' \
':shell -- Shell type (bash, zsh, fish, elvish, powershell):(bash elvish fish powershell zsh)' \
&& ret=0
;;
(manpage)
_arguments "${_arguments_options[@]}" : \
'--subcommand=[Generate man page for a specific subcommand]:SUBCOMMAND:_default' \
'-o+[Output directory for man pages (default\: ./man/)]:DIR:_default' \
'--outdir=[Output directory for man pages (default\: ./man/)]:DIR:_default' \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help (see more with '\''--help'\'')]' \
'--help[Print help (see more with '\''--help'\'')]' \
'-V[Print version]' \
'--version[Print version]' \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
":: :_nofs__help_commands" \
"*::: :->help" \
&& ret=0

    case $state in
    (help)
        words=($line[1] "${words[@]}")
        (( CURRENT += 1 ))
        curcontext="${curcontext%:*:*}:nofs-help-command-$line[1]:"
        case $line[1] in
            (ls)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(find)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(which)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(create)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(stat)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(info)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(exists)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(cat)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(diff)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(cmp)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(df)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(grep)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(tree)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(cp)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(mv)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(rm)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(mkdir)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(rmdir)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(touch)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(du)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(completions)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(manpage)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
(help)
_arguments "${_arguments_options[@]}" : \
&& ret=0
;;
        esac
    ;;
esac
;;
        esac
    ;;
esac
}

(( $+functions[_nofs_commands] )) ||
_nofs_commands() {
    local commands; commands=(
'ls:List directory contents (like ls)' \
'find:Find files matching a pattern' \
'which:Find which branch contains a file' \
'create:Get the best branch path for creating a new file' \
'stat:Show filesystem statistics' \
'info:Show share configuration and status' \
'exists:Check if a file exists and return its location' \
'cat:Read file content (from first found branch)' \
'diff:Show differences between branches' \
'cmp:Compare files byte-by-byte' \
'df:Show disk free space (df-like output)' \
'grep:Search file contents across all branches (grep)' \
'tree:Show directory tree structure' \
'cp:Copy files/directories (supports nofs context paths)' \
'mv:Move files/directories (supports nofs context paths)' \
'rm:Remove files or directories' \
'mkdir:Create directories' \
'rmdir:Remove empty directories' \
'touch:Create or update files' \
'du:Show disk usage (recursive directory size calculation)' \
'completions:Generate shell completion scripts' \
'manpage:Generate man pages' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'nofs commands' commands "$@"
}
(( $+functions[_nofs__cat_commands] )) ||
_nofs__cat_commands() {
    local commands; commands=()
    _describe -t commands 'nofs cat commands' commands "$@"
}
(( $+functions[_nofs__cmp_commands] )) ||
_nofs__cmp_commands() {
    local commands; commands=()
    _describe -t commands 'nofs cmp commands' commands "$@"
}
(( $+functions[_nofs__completions_commands] )) ||
_nofs__completions_commands() {
    local commands; commands=()
    _describe -t commands 'nofs completions commands' commands "$@"
}
(( $+functions[_nofs__cp_commands] )) ||
_nofs__cp_commands() {
    local commands; commands=()
    _describe -t commands 'nofs cp commands' commands "$@"
}
(( $+functions[_nofs__create_commands] )) ||
_nofs__create_commands() {
    local commands; commands=()
    _describe -t commands 'nofs create commands' commands "$@"
}
(( $+functions[_nofs__df_commands] )) ||
_nofs__df_commands() {
    local commands; commands=()
    _describe -t commands 'nofs df commands' commands "$@"
}
(( $+functions[_nofs__diff_commands] )) ||
_nofs__diff_commands() {
    local commands; commands=()
    _describe -t commands 'nofs diff commands' commands "$@"
}
(( $+functions[_nofs__du_commands] )) ||
_nofs__du_commands() {
    local commands; commands=()
    _describe -t commands 'nofs du commands' commands "$@"
}
(( $+functions[_nofs__exists_commands] )) ||
_nofs__exists_commands() {
    local commands; commands=()
    _describe -t commands 'nofs exists commands' commands "$@"
}
(( $+functions[_nofs__find_commands] )) ||
_nofs__find_commands() {
    local commands; commands=()
    _describe -t commands 'nofs find commands' commands "$@"
}
(( $+functions[_nofs__grep_commands] )) ||
_nofs__grep_commands() {
    local commands; commands=()
    _describe -t commands 'nofs grep commands' commands "$@"
}
(( $+functions[_nofs__help_commands] )) ||
_nofs__help_commands() {
    local commands; commands=(
'ls:List directory contents (like ls)' \
'find:Find files matching a pattern' \
'which:Find which branch contains a file' \
'create:Get the best branch path for creating a new file' \
'stat:Show filesystem statistics' \
'info:Show share configuration and status' \
'exists:Check if a file exists and return its location' \
'cat:Read file content (from first found branch)' \
'diff:Show differences between branches' \
'cmp:Compare files byte-by-byte' \
'df:Show disk free space (df-like output)' \
'grep:Search file contents across all branches (grep)' \
'tree:Show directory tree structure' \
'cp:Copy files/directories (supports nofs context paths)' \
'mv:Move files/directories (supports nofs context paths)' \
'rm:Remove files or directories' \
'mkdir:Create directories' \
'rmdir:Remove empty directories' \
'touch:Create or update files' \
'du:Show disk usage (recursive directory size calculation)' \
'completions:Generate shell completion scripts' \
'manpage:Generate man pages' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'nofs help commands' commands "$@"
}
(( $+functions[_nofs__help__cat_commands] )) ||
_nofs__help__cat_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help cat commands' commands "$@"
}
(( $+functions[_nofs__help__cmp_commands] )) ||
_nofs__help__cmp_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help cmp commands' commands "$@"
}
(( $+functions[_nofs__help__completions_commands] )) ||
_nofs__help__completions_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help completions commands' commands "$@"
}
(( $+functions[_nofs__help__cp_commands] )) ||
_nofs__help__cp_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help cp commands' commands "$@"
}
(( $+functions[_nofs__help__create_commands] )) ||
_nofs__help__create_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help create commands' commands "$@"
}
(( $+functions[_nofs__help__df_commands] )) ||
_nofs__help__df_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help df commands' commands "$@"
}
(( $+functions[_nofs__help__diff_commands] )) ||
_nofs__help__diff_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help diff commands' commands "$@"
}
(( $+functions[_nofs__help__du_commands] )) ||
_nofs__help__du_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help du commands' commands "$@"
}
(( $+functions[_nofs__help__exists_commands] )) ||
_nofs__help__exists_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help exists commands' commands "$@"
}
(( $+functions[_nofs__help__find_commands] )) ||
_nofs__help__find_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help find commands' commands "$@"
}
(( $+functions[_nofs__help__grep_commands] )) ||
_nofs__help__grep_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help grep commands' commands "$@"
}
(( $+functions[_nofs__help__help_commands] )) ||
_nofs__help__help_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help help commands' commands "$@"
}
(( $+functions[_nofs__help__info_commands] )) ||
_nofs__help__info_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help info commands' commands "$@"
}
(( $+functions[_nofs__help__ls_commands] )) ||
_nofs__help__ls_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help ls commands' commands "$@"
}
(( $+functions[_nofs__help__manpage_commands] )) ||
_nofs__help__manpage_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help manpage commands' commands "$@"
}
(( $+functions[_nofs__help__mkdir_commands] )) ||
_nofs__help__mkdir_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help mkdir commands' commands "$@"
}
(( $+functions[_nofs__help__mv_commands] )) ||
_nofs__help__mv_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help mv commands' commands "$@"
}
(( $+functions[_nofs__help__rm_commands] )) ||
_nofs__help__rm_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help rm commands' commands "$@"
}
(( $+functions[_nofs__help__rmdir_commands] )) ||
_nofs__help__rmdir_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help rmdir commands' commands "$@"
}
(( $+functions[_nofs__help__stat_commands] )) ||
_nofs__help__stat_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help stat commands' commands "$@"
}
(( $+functions[_nofs__help__touch_commands] )) ||
_nofs__help__touch_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help touch commands' commands "$@"
}
(( $+functions[_nofs__help__tree_commands] )) ||
_nofs__help__tree_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help tree commands' commands "$@"
}
(( $+functions[_nofs__help__which_commands] )) ||
_nofs__help__which_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help which commands' commands "$@"
}
(( $+functions[_nofs__info_commands] )) ||
_nofs__info_commands() {
    local commands; commands=()
    _describe -t commands 'nofs info commands' commands "$@"
}
(( $+functions[_nofs__ls_commands] )) ||
_nofs__ls_commands() {
    local commands; commands=()
    _describe -t commands 'nofs ls commands' commands "$@"
}
(( $+functions[_nofs__manpage_commands] )) ||
_nofs__manpage_commands() {
    local commands; commands=()
    _describe -t commands 'nofs manpage commands' commands "$@"
}
(( $+functions[_nofs__mkdir_commands] )) ||
_nofs__mkdir_commands() {
    local commands; commands=()
    _describe -t commands 'nofs mkdir commands' commands "$@"
}
(( $+functions[_nofs__mv_commands] )) ||
_nofs__mv_commands() {
    local commands; commands=()
    _describe -t commands 'nofs mv commands' commands "$@"
}
(( $+functions[_nofs__rm_commands] )) ||
_nofs__rm_commands() {
    local commands; commands=()
    _describe -t commands 'nofs rm commands' commands "$@"
}
(( $+functions[_nofs__rmdir_commands] )) ||
_nofs__rmdir_commands() {
    local commands; commands=()
    _describe -t commands 'nofs rmdir commands' commands "$@"
}
(( $+functions[_nofs__stat_commands] )) ||
_nofs__stat_commands() {
    local commands; commands=()
    _describe -t commands 'nofs stat commands' commands "$@"
}
(( $+functions[_nofs__touch_commands] )) ||
_nofs__touch_commands() {
    local commands; commands=()
    _describe -t commands 'nofs touch commands' commands "$@"
}
(( $+functions[_nofs__tree_commands] )) ||
_nofs__tree_commands() {
    local commands; commands=()
    _describe -t commands 'nofs tree commands' commands "$@"
}
(( $+functions[_nofs__which_commands] )) ||
_nofs__which_commands() {
    local commands; commands=()
    _describe -t commands 'nofs which commands' commands "$@"
}

if [ "$funcstack[1]" = "_nofs" ]; then
    _nofs "$@"
else
    compdef _nofs nofs
fi
