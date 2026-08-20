use std::path::Path;
use std::process::Command;
use std::process::Stdio;

use crate::OsError;

/// Writes `command` (prefixed with a `cd` into `working_dir`) to a small `.command` file and
/// hands it to `open`, rather than automating one specific terminal app via AppleScript. An
/// earlier version of this used `osascript -e 'tell application "Terminal" to do script ...'`,
/// which has two real problems: it always targets Apple's own Terminal.app specifically,
/// regardless of what the user is actually running (iTerm2, Ghostty, a plain tmux
/// session — anything else, and the new window opens somewhere the user isn't looking, reading as
/// "nothing happened"); and `Command::status()` inherits stdout by default, so `osascript`'s own
/// return value (`"tab N of window id M of application \"Terminal\""`) got written straight into
/// Anvil's own raw-mode/alternate-screen terminal, corrupting it — ratatui's diffing has no way to
/// know a cell it never touched just changed under it, so the stray text persisted on screen
/// until something else happened to overwrite those exact cells. `open` instead launches the
/// `.command` file through whatever the user's actual default handler for that file type is
/// (normally Terminal.app, but respects a real LaunchServices override) and prints nothing on
/// success. The `.command` extension (not `.sh`) is what makes `open` treat it as a
/// double-click-to-run script rather than a plain text file.
pub fn open_running(command: &str, working_dir: &Path) -> Result<(), OsError> {
    let script_path = std::env::temp_dir().join(format!("escher-os-run-{}-{}.command", std::process::id(), command.len()));
    let script = format!("#!/bin/sh\ncd {} || exit 1\n{command}\n", shell_quote(working_dir));
    std::fs::write(&script_path, script).map_err(|error| OsError::Failed(error.to_string()))?;

    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(&script_path, std::fs::Permissions::from_mode(0o755)).map_err(|error| OsError::Failed(error.to_string()))?;

    // `open` doesn't print anything on success, but nulling stdio anyway rather than trusting
    // that — an inherited fd is exactly what corrupted Anvil's display before, see this
    // function's own doc comment.
    let status = Command::new("open")
        .arg(&script_path)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|error| OsError::Failed(error.to_string()))?;

    if !status.success() {
        return Err(OsError::Failed(format!("open exited with {status}")));
    }

    Ok(())
}

/// A minimal single-quote shell escape — wraps `path` in single quotes, closing and reopening
/// around any literal single quote it contains. Good enough for real filesystem paths; not a
/// general-purpose shell-escaping utility.
fn shell_quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}
