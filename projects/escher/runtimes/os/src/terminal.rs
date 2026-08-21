//! Launching a command in a real, standalone terminal window — for anything that needs a genuine
//! TTY of its own (raw mode, an alternate screen), which a piped subprocess can't give it.

use std::path::Path;

use crate::OsError;

/// Opens a new terminal window running `command` from `working_dir`. Fire-and-forget: the new
/// window is independent of this process from the moment it opens, so there's nothing further to
/// wait on or manage.
pub fn open_running(command: &str, working_dir: &Path) -> Result<(), OsError> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::terminal::open_running(command, working_dir)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (command, working_dir);
        Err(OsError::Unsupported)
    }
}
