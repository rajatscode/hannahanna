use crate::errors::Result;

/// Output shell wrapper function for hn switch command
///
/// This prints the shell function that enables `hn switch` to actually
/// change directories. Users should add this to their ~/.bashrc or ~/.zshrc:
///
/// ```bash
/// eval "$(hn init-shell)"
/// ```
pub fn run() -> Result<()> {
    let shell_function = r#"# hannahanna shell integration
# This function wraps the 'hn' command to enable directory switching
hn() {
    if [ "$1" = "switch" ]; then
        # Capture the worktree path from stdout
        local path=$(command hn switch "$2" 2>/dev/null)
        if [ $? -eq 0 ]; then
            # Switch succeeded, change directory
            cd "$path"
            # Show info messages (they were suppressed above)
            command hn switch "$2" >/dev/null
        else
            # Switch failed, show error message
            command hn switch "$2"
        fi
    elif [ "$1" = "return" ]; then
        # Capture the parent worktree path from stdout
        local path=$(command hn return "${@:2}" 2>/dev/null)
        if [ $? -eq 0 ]; then
            # Return succeeded, change directory to parent
            cd "$path"
        else
            # Return failed, show error message
            command hn return "${@:2}"
        fi
    else
        # Pass through all other commands
        command hn "$@"
    fi
}
"#;

    print!("{}", shell_function);
    Ok(())
}
