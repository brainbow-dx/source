//! Native alert/confirm dialogs.

use crate::OsError;

/// Shows a blocking alert dialog with a single "OK" button.
pub fn alert(title: &str, message: &str) -> Result<(), OsError> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::dialog::alert(title, message)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (title, message);
        Err(OsError::Unsupported)
    }
}

/// Shows a blocking confirm dialog with "OK"/"Cancel" buttons. Returns `true` if OK was chosen.
pub fn confirm(title: &str, message: &str) -> Result<bool, OsError> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::dialog::confirm(title, message)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (title, message);
        Err(OsError::Unsupported)
    }
}
