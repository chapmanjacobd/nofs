
using namespace System.Management.Automation
using namespace System.Management.Automation.Language

Register-ArgumentCompleter -Native -CommandName 'nofs' -ScriptBlock {
    param($wordToComplete, $commandAst, $cursorPosition)

    $commandElements = $commandAst.CommandElements
    $command = @(
        'nofs'
        for ($i = 1; $i -lt $commandElements.Count; $i++) {
            $element = $commandElements[$i]
            if ($element -isnot [StringConstantExpressionAst] -or
                $element.StringConstantType -ne [StringConstantType]::BareWord -or
                $element.Value.StartsWith('-') -or
                $element.Value -eq $wordToComplete) {
                break
        }
        $element.Value
    }) -join ';'

    $completions = @(switch ($command) {
        'nofs' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List directory contents (like ls)')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'Find files matching a pattern')
            [CompletionResult]::new('which', 'which', [CompletionResultType]::ParameterValue, 'Find which branch contains a file')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Get the best branch path for creating a new file')
            [CompletionResult]::new('stat', 'stat', [CompletionResultType]::ParameterValue, 'Show filesystem statistics')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show share configuration and status')
            [CompletionResult]::new('exists', 'exists', [CompletionResultType]::ParameterValue, 'Check if a file exists and return its location')
            [CompletionResult]::new('cat', 'cat', [CompletionResultType]::ParameterValue, 'Read file content (from first found branch)')
            [CompletionResult]::new('cp', 'cp', [CompletionResultType]::ParameterValue, 'Copy files/directories (supports nofs context paths)')
            [CompletionResult]::new('mv', 'mv', [CompletionResultType]::ParameterValue, 'Move files/directories (supports nofs context paths)')
            [CompletionResult]::new('rm', 'rm', [CompletionResultType]::ParameterValue, 'Remove files or directories')
            [CompletionResult]::new('mkdir', 'mkdir', [CompletionResultType]::ParameterValue, 'Create directories')
            [CompletionResult]::new('rmdir', 'rmdir', [CompletionResultType]::ParameterValue, 'Remove empty directories')
            [CompletionResult]::new('touch', 'touch', [CompletionResultType]::ParameterValue, 'Create or update files')
            [CompletionResult]::new('du', 'du', [CompletionResultType]::ParameterValue, 'Show disk usage (recursive directory size calculation)')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts')
            [CompletionResult]::new('manpage', 'manpage', [CompletionResultType]::ParameterValue, 'Generate man page')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'nofs;ls' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Show detailed information (permissions, size, modification time)')
            [CompletionResult]::new('--long', '--long', [CompletionResultType]::ParameterName, 'Show detailed information (permissions, size, modification time)')
            [CompletionResult]::new('-a', '-a', [CompletionResultType]::ParameterName, 'Show hidden files (files starting with .)')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'Show hidden files (files starting with .)')
            [CompletionResult]::new('--conflicts', '--conflicts', [CompletionResultType]::ParameterName, 'Detect and report conflicts (files with same name but different content)')
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'Use hash comparison for conflict detection (slower but more accurate)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;find' {
            [CompletionResult]::new('--name', '--name', [CompletionResultType]::ParameterName, 'Filename pattern (glob syntax: *.txt, **/logs/*)')
            [CompletionResult]::new('--type', '--type', [CompletionResultType]::ParameterName, 'File type: ''f'' for files, ''d'' for directories')
            [CompletionResult]::new('--maxdepth', '--maxdepth', [CompletionResultType]::ParameterName, 'Maximum directory traversal depth (0 = starting directory only)')
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;which' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-a', '-a', [CompletionResultType]::ParameterName, 'Show all branches containing the file (not just the first)')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'Show all branches containing the file (not just the first)')
            [CompletionResult]::new('--conflicts', '--conflicts', [CompletionResultType]::ParameterName, 'Detect and report conflicts (files with same name but different content)')
            [CompletionResult]::new('--hash', '--hash', [CompletionResultType]::ParameterName, 'Use hash comparison for conflict detection (slower but more accurate)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;create' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;stat' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-H', '-H ', [CompletionResultType]::ParameterName, 'Show human-readable sizes (KB, MB, GB instead of bytes)')
            [CompletionResult]::new('--human', '--human', [CompletionResultType]::ParameterName, 'Show human-readable sizes (KB, MB, GB instead of bytes)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;info' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;exists' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;cat' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;cp' {
            [CompletionResult]::new('--file-over-file', '--file-over-file', [CompletionResultType]::ParameterName, 'File-over-file conflict strategy')
            [CompletionResult]::new('--file-over-folder', '--file-over-folder', [CompletionResultType]::ParameterName, 'File-over-folder conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge')
            [CompletionResult]::new('--folder-over-file', '--folder-over-file', [CompletionResultType]::ParameterName, 'Folder-over-file conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge')
            [CompletionResult]::new('-j', '-j', [CompletionResultType]::ParameterName, 'Number of parallel workers')
            [CompletionResult]::new('--workers', '--workers', [CompletionResultType]::ParameterName, 'Number of parallel workers')
            [CompletionResult]::new('-e', '-e', [CompletionResultType]::ParameterName, 'Filter by file extensions (e.g., .mkv, .jpg)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'Filter by file extensions (e.g., .mkv, .jpg)')
            [CompletionResult]::new('-E', '-E ', [CompletionResultType]::ParameterName, 'Exclude files matching glob pattern')
            [CompletionResult]::new('--exclude', '--exclude', [CompletionResultType]::ParameterName, 'Exclude files matching glob pattern')
            [CompletionResult]::new('-I', '-I ', [CompletionResultType]::ParameterName, 'Include only files matching glob pattern')
            [CompletionResult]::new('--include', '--include', [CompletionResultType]::ParameterName, 'Include only files matching glob pattern')
            [CompletionResult]::new('-S', '-S ', [CompletionResultType]::ParameterName, 'Filter by file size (e.g., +5M, -10M)')
            [CompletionResult]::new('--size', '--size', [CompletionResultType]::ParameterName, 'Filter by file size (e.g., +5M, -10M)')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Limit number of files transferred')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Limit number of files transferred')
            [CompletionResult]::new('--size-limit', '--size-limit', [CompletionResultType]::ParameterName, 'Limit total size transferred (e.g., 100M, 1G)')
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'Simulate without making changes (dry-run)')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Simulate without making changes (dry-run)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;mv' {
            [CompletionResult]::new('--file-over-file', '--file-over-file', [CompletionResultType]::ParameterName, 'File-over-file conflict strategy')
            [CompletionResult]::new('--file-over-folder', '--file-over-folder', [CompletionResultType]::ParameterName, 'File-over-folder conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge')
            [CompletionResult]::new('--folder-over-file', '--folder-over-file', [CompletionResultType]::ParameterName, 'Folder-over-file conflict strategy: skip, rename-src, rename-dest, delete-src, delete-dest, merge')
            [CompletionResult]::new('-j', '-j', [CompletionResultType]::ParameterName, 'Number of parallel workers')
            [CompletionResult]::new('--workers', '--workers', [CompletionResultType]::ParameterName, 'Number of parallel workers')
            [CompletionResult]::new('-e', '-e', [CompletionResultType]::ParameterName, 'Filter by file extensions (e.g., .mkv, .jpg)')
            [CompletionResult]::new('--ext', '--ext', [CompletionResultType]::ParameterName, 'Filter by file extensions (e.g., .mkv, .jpg)')
            [CompletionResult]::new('-E', '-E ', [CompletionResultType]::ParameterName, 'Exclude files matching glob pattern')
            [CompletionResult]::new('--exclude', '--exclude', [CompletionResultType]::ParameterName, 'Exclude files matching glob pattern')
            [CompletionResult]::new('-I', '-I ', [CompletionResultType]::ParameterName, 'Include only files matching glob pattern')
            [CompletionResult]::new('--include', '--include', [CompletionResultType]::ParameterName, 'Include only files matching glob pattern')
            [CompletionResult]::new('-S', '-S ', [CompletionResultType]::ParameterName, 'Filter by file size (e.g., +5M, -10M)')
            [CompletionResult]::new('--size', '--size', [CompletionResultType]::ParameterName, 'Filter by file size (e.g., +5M, -10M)')
            [CompletionResult]::new('-l', '-l', [CompletionResultType]::ParameterName, 'Limit number of files moved')
            [CompletionResult]::new('--limit', '--limit', [CompletionResultType]::ParameterName, 'Limit number of files moved')
            [CompletionResult]::new('--size-limit', '--size-limit', [CompletionResultType]::ParameterName, 'Limit total size moved (e.g., 100M, 1G)')
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-n', '-n', [CompletionResultType]::ParameterName, 'Simulate without making changes (dry-run)')
            [CompletionResult]::new('--dry-run', '--dry-run', [CompletionResultType]::ParameterName, 'Simulate without making changes (dry-run)')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;rm' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-r', '-r', [CompletionResultType]::ParameterName, 'Remove directories and their contents recursively')
            [CompletionResult]::new('--recursive', '--recursive', [CompletionResultType]::ParameterName, 'Remove directories and their contents recursively')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Print each file/directory as it is removed')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Print each file/directory as it is removed')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;mkdir' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-p', '-p', [CompletionResultType]::ParameterName, 'Create parent directories as needed')
            [CompletionResult]::new('--parents', '--parents', [CompletionResultType]::ParameterName, 'Create parent directories as needed')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Print each directory as it is created')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Print each directory as it is created')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;rmdir' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Print the directory as it is removed')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Print the directory as it is removed')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;touch' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Print the file path after creation/update')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Print the file path after creation/update')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;du' {
            [CompletionResult]::new('--maxdepth', '--maxdepth', [CompletionResultType]::ParameterName, 'Maximum directory traversal depth (0 = starting directory only)')
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-H', '-H ', [CompletionResultType]::ParameterName, 'Show human-readable sizes (KB, MB, GB instead of bytes)')
            [CompletionResult]::new('--human', '--human', [CompletionResultType]::ParameterName, 'Show human-readable sizes (KB, MB, GB instead of bytes)')
            [CompletionResult]::new('-a', '-a', [CompletionResultType]::ParameterName, 'Show all subdirectory sizes')
            [CompletionResult]::new('--all', '--all', [CompletionResultType]::ParameterName, 'Show all subdirectory sizes')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;completions' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;manpage' {
            [CompletionResult]::new('-c', '-c', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--config', '--config', [CompletionResultType]::ParameterName, 'Path to configuration file')
            [CompletionResult]::new('--paths', '--paths', [CompletionResultType]::ParameterName, 'Comma-separated list of branch paths (ad-hoc mode) Format: /path1,/path2 or /path1=RW,/path2=RO')
            [CompletionResult]::new('--policy', '--policy', [CompletionResultType]::ParameterName, 'Policy to use for branch selection')
            [CompletionResult]::new('--minfreespace', '--minfreespace', [CompletionResultType]::ParameterName, 'Minimum free space required on branch (e.g., "4G", "100M")')
            [CompletionResult]::new('-v', '-v', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--verbose', '--verbose', [CompletionResultType]::ParameterName, 'Verbose output (print decision steps to stderr)')
            [CompletionResult]::new('--json', '--json', [CompletionResultType]::ParameterName, 'Output in JSON format (for scripting/automation)')
            [CompletionResult]::new('-h', '-h', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('--help', '--help', [CompletionResultType]::ParameterName, 'Print help (see more with ''--help'')')
            [CompletionResult]::new('-V', '-V ', [CompletionResultType]::ParameterName, 'Print version')
            [CompletionResult]::new('--version', '--version', [CompletionResultType]::ParameterName, 'Print version')
            break
        }
        'nofs;help' {
            [CompletionResult]::new('ls', 'ls', [CompletionResultType]::ParameterValue, 'List directory contents (like ls)')
            [CompletionResult]::new('find', 'find', [CompletionResultType]::ParameterValue, 'Find files matching a pattern')
            [CompletionResult]::new('which', 'which', [CompletionResultType]::ParameterValue, 'Find which branch contains a file')
            [CompletionResult]::new('create', 'create', [CompletionResultType]::ParameterValue, 'Get the best branch path for creating a new file')
            [CompletionResult]::new('stat', 'stat', [CompletionResultType]::ParameterValue, 'Show filesystem statistics')
            [CompletionResult]::new('info', 'info', [CompletionResultType]::ParameterValue, 'Show share configuration and status')
            [CompletionResult]::new('exists', 'exists', [CompletionResultType]::ParameterValue, 'Check if a file exists and return its location')
            [CompletionResult]::new('cat', 'cat', [CompletionResultType]::ParameterValue, 'Read file content (from first found branch)')
            [CompletionResult]::new('cp', 'cp', [CompletionResultType]::ParameterValue, 'Copy files/directories (supports nofs context paths)')
            [CompletionResult]::new('mv', 'mv', [CompletionResultType]::ParameterValue, 'Move files/directories (supports nofs context paths)')
            [CompletionResult]::new('rm', 'rm', [CompletionResultType]::ParameterValue, 'Remove files or directories')
            [CompletionResult]::new('mkdir', 'mkdir', [CompletionResultType]::ParameterValue, 'Create directories')
            [CompletionResult]::new('rmdir', 'rmdir', [CompletionResultType]::ParameterValue, 'Remove empty directories')
            [CompletionResult]::new('touch', 'touch', [CompletionResultType]::ParameterValue, 'Create or update files')
            [CompletionResult]::new('du', 'du', [CompletionResultType]::ParameterValue, 'Show disk usage (recursive directory size calculation)')
            [CompletionResult]::new('completions', 'completions', [CompletionResultType]::ParameterValue, 'Generate shell completion scripts')
            [CompletionResult]::new('manpage', 'manpage', [CompletionResultType]::ParameterValue, 'Generate man page')
            [CompletionResult]::new('help', 'help', [CompletionResultType]::ParameterValue, 'Print this message or the help of the given subcommand(s)')
            break
        }
        'nofs;help;ls' {
            break
        }
        'nofs;help;find' {
            break
        }
        'nofs;help;which' {
            break
        }
        'nofs;help;create' {
            break
        }
        'nofs;help;stat' {
            break
        }
        'nofs;help;info' {
            break
        }
        'nofs;help;exists' {
            break
        }
        'nofs;help;cat' {
            break
        }
        'nofs;help;cp' {
            break
        }
        'nofs;help;mv' {
            break
        }
        'nofs;help;rm' {
            break
        }
        'nofs;help;mkdir' {
            break
        }
        'nofs;help;rmdir' {
            break
        }
        'nofs;help;touch' {
            break
        }
        'nofs;help;du' {
            break
        }
        'nofs;help;completions' {
            break
        }
        'nofs;help;manpage' {
            break
        }
        'nofs;help;help' {
            break
        }
    })

    $completions.Where{ $_.CompletionText -like "$wordToComplete*" } |
        Sort-Object -Property ListItemText
}
