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
'-l[Show detailed information]' \
'--long[Show detailed information]' \
'-a[Show hidden files]' \
'--all[Show hidden files]' \
'--conflicts[Detect and report conflicts (files with same name but different content)]' \
'--hash[Use hash comparison for conflict detection (slower but more accurate)]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
':path -- Path within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(find)
_arguments "${_arguments_options[@]}" : \
'--name=[Filename pattern (glob)]:NAME:_default' \
'--type=[File type\: f=file, d=directory]:TYPE:_default' \
'--maxdepth=[Maximum depth]:MAXDEPTH:_default' \
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
':path -- Starting path within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(which)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-a[Show all branches containing the file]' \
'--all[Show all branches containing the file]' \
'--conflicts[Detect and report conflicts (files with same name but different content)]' \
'--hash[Use hash comparison for conflict detection (slower but more accurate)]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
':path -- Path within the share (format\: \[context\:\]path):_default' \
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
':path -- Path within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(stat)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-H[Show human-readable sizes]' \
'--human[Show human-readable sizes]' \
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
'::context -- Context name (optional, shows all if not specified):_default' \
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
':path -- Path within the share (format\: \[context\:\]path):_default' \
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
':path -- Path within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(cp)
_arguments "${_arguments_options[@]}" : \
'--file-over-file=[File-over-file conflict strategy (e.g., "skip-hash rename-dest")]:FILE_OVER_FILE:_default' \
'--file-over-folder=[File-over-folder conflict strategy]:FILE_OVER_FOLDER:_default' \
'--folder-over-file=[Folder-over-file conflict strategy]:FOLDER_OVER_FILE:_default' \
'-j+[Number of parallel workers]:WORKERS:_default' \
'--workers=[Number of parallel workers]:WORKERS:_default' \
'*-e+[Filter by file extensions (e.g., .mkv, .jpg)]:EXT:_default' \
'*--ext=[Filter by file extensions (e.g., .mkv, .jpg)]:EXT:_default' \
'*-E+[Exclude patterns (glob)]:EXCLUDE:_default' \
'*--exclude=[Exclude patterns (glob)]:EXCLUDE:_default' \
'*-I+[Include patterns (glob)]:INCLUDE:_default' \
'*--include=[Include patterns (glob)]:INCLUDE:_default' \
'*-S+[Filter by file size (e.g., +5M, -10M)]:SIZE:_default' \
'*--size=[Filter by file size (e.g., +5M, -10M)]:SIZE:_default' \
'-l+[Limit number of files transferred]:LIMIT:_default' \
'--limit=[Limit number of files transferred]:LIMIT:_default' \
'--size-limit=[Limit total size transferred (e.g., 100M, 1G)]:SIZE_LIMIT:_default' \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-n[Simulate without making changes (dry-run)]' \
'--dry-run[Simulate without making changes (dry-run)]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::paths -- Source paths \[...\] and destination (last argument). Format\: \[context\:\]path or regular path:_default' \
&& ret=0
;;
(mv)
_arguments "${_arguments_options[@]}" : \
'--file-over-file=[File-over-file conflict strategy (e.g., "skip-hash rename-dest")]:FILE_OVER_FILE:_default' \
'--file-over-folder=[File-over-folder conflict strategy]:FILE_OVER_FOLDER:_default' \
'--folder-over-file=[Folder-over-file conflict strategy]:FOLDER_OVER_FILE:_default' \
'-j+[Number of parallel workers]:WORKERS:_default' \
'--workers=[Number of parallel workers]:WORKERS:_default' \
'*-e+[Filter by file extensions (e.g., .mkv, .jpg)]:EXT:_default' \
'*--ext=[Filter by file extensions (e.g., .mkv, .jpg)]:EXT:_default' \
'*-E+[Exclude patterns (glob)]:EXCLUDE:_default' \
'*--exclude=[Exclude patterns (glob)]:EXCLUDE:_default' \
'*-I+[Include patterns (glob)]:INCLUDE:_default' \
'*--include=[Include patterns (glob)]:INCLUDE:_default' \
'*-S+[Filter by file size (e.g., +5M, -10M)]:SIZE:_default' \
'*--size=[Filter by file size (e.g., +5M, -10M)]:SIZE:_default' \
'-l+[Limit number of files transferred]:LIMIT:_default' \
'--limit=[Limit number of files transferred]:LIMIT:_default' \
'--size-limit=[Limit total size transferred (e.g., 100M, 1G)]:SIZE_LIMIT:_default' \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-n[Simulate without making changes (dry-run)]' \
'--dry-run[Simulate without making changes (dry-run)]' \
'-v[Verbose output (print decision steps to stderr)]' \
'--verbose[Verbose output (print decision steps to stderr)]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::paths -- Source paths \[...\] and destination (last argument). Format\: \[context\:\]path or regular path:_default' \
&& ret=0
;;
(rm)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-r[Remove directories and their contents recursively]' \
'--recursive[Remove directories and their contents recursively]' \
'-v[Verbose output]' \
'--verbose[Verbose output]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
'*::paths -- Path(s) within the share (format\: \[context\:\]path):_default' \
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
'-v[Verbose output]' \
'--verbose[Verbose output]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
':path -- Path within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(rmdir)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Verbose output]' \
'--verbose[Verbose output]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
':path -- Path within the share (format\: \[context\:\]path):_default' \
&& ret=0
;;
(touch)
_arguments "${_arguments_options[@]}" : \
'-c+[Path to configuration file]:CONFIG:_default' \
'--config=[Path to configuration file]:CONFIG:_default' \
'--paths=[Comma-separated list of branch paths (ad-hoc mode) Format\: /path1,/path2 or /path1=RW,/path2=RO]:PATHS:_default' \
'--policy=[Policy to use for branch selection]:POLICY:_default' \
'--minfreespace=[Minimum free space required on branch (e.g., "4G", "100M")]:MINFREESPACE:_default' \
'-v[Verbose output]' \
'--verbose[Verbose output]' \
'--json[Output in JSON format (for scripting/automation)]' \
'-h[Print help]' \
'--help[Print help]' \
'-V[Print version]' \
'--version[Print version]' \
':path -- Path within the share (format\: \[context\:\]path):_default' \
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
'cp:Copy files/directories (supports nofs context paths)' \
'mv:Move files/directories (supports nofs context paths)' \
'rm:Remove files or directories' \
'mkdir:Create directories' \
'rmdir:Remove empty directories' \
'touch:Create or update files' \
'completions:Generate shell completion scripts' \
'manpage:Generate man page' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'nofs commands' commands "$@"
}
(( $+functions[_nofs__cat_commands] )) ||
_nofs__cat_commands() {
    local commands; commands=()
    _describe -t commands 'nofs cat commands' commands "$@"
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
'cp:Copy files/directories (supports nofs context paths)' \
'mv:Move files/directories (supports nofs context paths)' \
'rm:Remove files or directories' \
'mkdir:Create directories' \
'rmdir:Remove empty directories' \
'touch:Create or update files' \
'completions:Generate shell completion scripts' \
'manpage:Generate man page' \
'help:Print this message or the help of the given subcommand(s)' \
    )
    _describe -t commands 'nofs help commands' commands "$@"
}
(( $+functions[_nofs__help__cat_commands] )) ||
_nofs__help__cat_commands() {
    local commands; commands=()
    _describe -t commands 'nofs help cat commands' commands "$@"
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
