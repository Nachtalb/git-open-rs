complete --no-files -c git -n '__fish_git_needs_command' -a open -d 'Open the current git repo in the browser'  # git open
complete --no-files -c git -n '__fish_git_using_command open' -s c -l commit    -d "Commit hash"          -ka '(__fish_git_recent_commits --all)'  # git open COMMIT
complete --no-files -c git -n '__fish_git_using_command open' -s p -l path      -d "Path of the git repo" -ka '(__fish_complete_directories)'      # git open --path/-p /some/path
complete --no-files -c git -n '__fish_git_using_command open' -s h -l help      -d "Print usage information"             # git open --help/-h
complete --no-files -c git -n '__fish_git_using_command open' -s n -l no-branch -d "Don't open current branch"           # git open --no-branch/-n
complete --no-files -c git -n '__fish_git_using_command open' -s q -l quiet     -d "Less verbose output per occurrence"  # git open --verbose/-v
complete --no-files -c git -n '__fish_git_using_command open' -s v -l verbose   -d "More verbose output per occurrence"  # git open --quiet/-q
