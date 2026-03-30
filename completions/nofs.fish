# Print an optspec for argparse to handle cmd's options that are independent of any subcommand.
function __fish_nofs_global_optspecs
	string join \n c/config= paths= policy= minfreespace= v/verbose json h/help V/version
end

function __fish_nofs_needs_command
	# Figure out if the current invocation already has a command.
	set -l cmd (commandline -opc)
	set -e cmd[1]
	argparse -s (__fish_nofs_global_optspecs) -- $cmd 2>/dev/null
	or return
	if set -q argv[1]
		# Also print the command, so this can be used to figure out what it is.
		echo $argv[1]
		return 1
	end
	return 0
end

function __fish_nofs_using_subcommand
	set -l cmd (__fish_nofs_needs_command)
	test -z "$cmd"
	and return 1
	contains -- $cmd[1] $argv
end

complete -c nofs -n "__fish_nofs_needs_command" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_needs_command" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_needs_command" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_needs_command" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_needs_command" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_needs_command" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_needs_command" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_needs_command" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "ls" -d 'List directory contents (like ls)'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "find" -d 'Find files matching a pattern'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "which" -d 'Find which branch contains a file'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "create" -d 'Get the best branch path for creating a new file'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "stat" -d 'Show filesystem statistics'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "info" -d 'Show share configuration and status'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "exists" -d 'Check if a file exists and return its location'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "cat" -d 'Read file content (from first found branch)'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "cp" -d 'Copy files/directories (supports nofs context paths)'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "mv" -d 'Move files/directories (supports nofs context paths)'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "rm" -d 'Remove files or directories'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "mkdir" -d 'Create directories'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "rmdir" -d 'Remove empty directories'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "touch" -d 'Create or update files'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "du" -d 'Show disk usage (recursive directory size calculation)'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "completions" -d 'Generate shell completion scripts'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "manpage" -d 'Generate man pages'
complete -c nofs -n "__fish_nofs_needs_command" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
complete -c nofs -n "__fish_nofs_using_subcommand ls" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand ls" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand ls" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand ls" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand ls" -s l -l long -d 'Show detailed information (permissions, size, modification time)'
complete -c nofs -n "__fish_nofs_using_subcommand ls" -s a -l all -d 'Show hidden files (files starting with .)'
complete -c nofs -n "__fish_nofs_using_subcommand ls" -l conflicts -d 'Detect and report conflicts (files with same name but different content)'
complete -c nofs -n "__fish_nofs_using_subcommand ls" -l hash -d 'Use hash comparison for conflict detection (slower but more accurate)'
complete -c nofs -n "__fish_nofs_using_subcommand ls" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand ls" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand ls" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand ls" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand find" -l name -d 'Filename pattern (glob syntax: *.txt, **/logs/*)' -r
complete -c nofs -n "__fish_nofs_using_subcommand find" -l type -d 'File type: \'f\' for files, \'d\' for directories' -r
complete -c nofs -n "__fish_nofs_using_subcommand find" -l maxdepth -d 'Maximum directory traversal depth (0 = starting directory only)' -r
complete -c nofs -n "__fish_nofs_using_subcommand find" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand find" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand find" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand find" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand find" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand find" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand find" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand find" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand which" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand which" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand which" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand which" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand which" -s a -l all -d 'Show all branches containing the file (not just the first)'
complete -c nofs -n "__fish_nofs_using_subcommand which" -l conflicts -d 'Detect and report conflicts (files with same name but different content)'
complete -c nofs -n "__fish_nofs_using_subcommand which" -l hash -d 'Use hash comparison for conflict detection (slower but more accurate)'
complete -c nofs -n "__fish_nofs_using_subcommand which" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand which" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand which" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand which" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand create" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand create" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand create" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand create" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand create" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand create" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand create" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand create" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand stat" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand stat" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand stat" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand stat" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand stat" -s H -l human -d 'Show human-readable sizes (KB, MB, GB instead of bytes)'
complete -c nofs -n "__fish_nofs_using_subcommand stat" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand stat" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand stat" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand stat" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand info" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand info" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand info" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand info" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand info" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand info" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand info" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand info" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand exists" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand exists" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand exists" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand exists" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand exists" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand exists" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand exists" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand exists" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand cat" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand cat" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand cat" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand cat" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand cat" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand cat" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand cat" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand cat" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand cp" -l file-over-file -d 'File-over-file conflict strategy' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -l file-over-folder -d 'File-over-folder conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -l folder-over-file -d 'Folder-over-file conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -s j -l workers -d 'Number of parallel workers' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -s e -l ext -d 'Filter by file extensions (e.g., .mkv, .jpg)' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -s E -l exclude -d 'Exclude files matching glob pattern' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -s I -l include -d 'Include only files matching glob pattern' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -s S -l size -d 'Filter by file size (e.g., +5M, -10M)' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -s l -l limit -d 'Limit number of files transferred' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -l size-limit -d 'Limit total size transferred (e.g., 100M, 1G)' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand cp" -s n -l dry-run -d 'Simulate without making changes (dry-run)'
complete -c nofs -n "__fish_nofs_using_subcommand cp" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand cp" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand cp" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c nofs -n "__fish_nofs_using_subcommand cp" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand mv" -l file-over-file -d 'File-over-file conflict strategy' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -l file-over-folder -d 'File-over-folder conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -l folder-over-file -d 'Folder-over-file conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -s j -l workers -d 'Number of parallel workers' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -s e -l ext -d 'Filter by file extensions (e.g., .mkv, .jpg)' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -s E -l exclude -d 'Exclude files matching glob pattern' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -s I -l include -d 'Include only files matching glob pattern' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -s S -l size -d 'Filter by file size (e.g., +5M, -10M)' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -s l -l limit -d 'Limit number of files moved' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -l size-limit -d 'Limit total size moved (e.g., 100M, 1G)' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand mv" -s n -l dry-run -d 'Simulate without making changes (dry-run)'
complete -c nofs -n "__fish_nofs_using_subcommand mv" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand mv" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand mv" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c nofs -n "__fish_nofs_using_subcommand mv" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand rm" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand rm" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand rm" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand rm" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand rm" -s r -l recursive -d 'Remove directories and their contents recursively'
complete -c nofs -n "__fish_nofs_using_subcommand rm" -s v -l verbose -d 'Print each file/directory as it is removed'
complete -c nofs -n "__fish_nofs_using_subcommand rm" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand rm" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand rm" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand mkdir" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand mkdir" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand mkdir" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand mkdir" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand mkdir" -s p -l parents -d 'Create parent directories as needed'
complete -c nofs -n "__fish_nofs_using_subcommand mkdir" -s v -l verbose -d 'Print each directory as it is created'
complete -c nofs -n "__fish_nofs_using_subcommand mkdir" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand mkdir" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand mkdir" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand rmdir" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand rmdir" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand rmdir" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand rmdir" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand rmdir" -s v -l verbose -d 'Print the directory as it is removed'
complete -c nofs -n "__fish_nofs_using_subcommand rmdir" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand rmdir" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand rmdir" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand touch" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand touch" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand touch" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand touch" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand touch" -s v -l verbose -d 'Print the file path after creation/update'
complete -c nofs -n "__fish_nofs_using_subcommand touch" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand touch" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand touch" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand du" -l maxdepth -d 'Maximum directory traversal depth (0 = starting directory only)' -r
complete -c nofs -n "__fish_nofs_using_subcommand du" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand du" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand du" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand du" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand du" -s H -l human -d 'Show human-readable sizes (KB, MB, GB instead of bytes)'
complete -c nofs -n "__fish_nofs_using_subcommand du" -s a -l all -d 'Show all subdirectory sizes'
complete -c nofs -n "__fish_nofs_using_subcommand du" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand du" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand du" -s h -l help -d 'Print help'
complete -c nofs -n "__fish_nofs_using_subcommand du" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand completions" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand completions" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand completions" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand completions" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand completions" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand completions" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand completions" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c nofs -n "__fish_nofs_using_subcommand completions" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand manpage" -l subcommand -d 'Generate man page for a specific subcommand' -r
complete -c nofs -n "__fish_nofs_using_subcommand manpage" -s o -l outdir -d 'Output directory for man pages (default: ./man/)' -r
complete -c nofs -n "__fish_nofs_using_subcommand manpage" -s c -l config -d 'Path to configuration file' -r
complete -c nofs -n "__fish_nofs_using_subcommand manpage" -l paths -d 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO' -r
complete -c nofs -n "__fish_nofs_using_subcommand manpage" -l policy -d 'Policy to use for branch selection' -r
complete -c nofs -n "__fish_nofs_using_subcommand manpage" -l minfreespace -d 'Minimum free space required on branch (e.g., "4G", "100M")' -r
complete -c nofs -n "__fish_nofs_using_subcommand manpage" -s v -l verbose -d 'Verbose output (print decision steps to stderr)'
complete -c nofs -n "__fish_nofs_using_subcommand manpage" -l json -d 'Output in JSON format (for scripting/automation)'
complete -c nofs -n "__fish_nofs_using_subcommand manpage" -s h -l help -d 'Print help (see more with \'--help\')'
complete -c nofs -n "__fish_nofs_using_subcommand manpage" -s V -l version -d 'Print version'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "ls" -d 'List directory contents (like ls)'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "find" -d 'Find files matching a pattern'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "which" -d 'Find which branch contains a file'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "create" -d 'Get the best branch path for creating a new file'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "stat" -d 'Show filesystem statistics'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "info" -d 'Show share configuration and status'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "exists" -d 'Check if a file exists and return its location'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "cat" -d 'Read file content (from first found branch)'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "cp" -d 'Copy files/directories (supports nofs context paths)'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "mv" -d 'Move files/directories (supports nofs context paths)'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "rm" -d 'Remove files or directories'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "mkdir" -d 'Create directories'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "rmdir" -d 'Remove empty directories'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "touch" -d 'Create or update files'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "du" -d 'Show disk usage (recursive directory size calculation)'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "completions" -d 'Generate shell completion scripts'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "manpage" -d 'Generate man pages'
complete -c nofs -n "__fish_nofs_using_subcommand help; and not __fish_seen_subcommand_from ls find which create stat info exists cat cp mv rm mkdir rmdir touch du completions manpage help" -f -a "help" -d 'Print this message or the help of the given subcommand(s)'
