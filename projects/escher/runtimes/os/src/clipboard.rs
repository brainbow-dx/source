//! System clipboard, plain text only.

use crate::OsError;

pub fn read_text() -> Result<Option<String>, OsError> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::clipboard::read_text()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(OsError::Unsupported)
    }
}

pub fn write_text(text: &str) -> Result<(), OsError> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::clipboard::write_text(text)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = text;
        Err(OsError::Unsupported)
    }
}
