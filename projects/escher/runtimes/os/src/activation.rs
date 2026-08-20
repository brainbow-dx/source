use crate::OsError;

/// Brings this whole app to the front, above whatever other app currently has focus — see
/// `macos::activation::activate`'s own doc comment for why this is a distinct concern from
/// focusing one of this app's own windows.
pub fn activate() -> Result<(), OsError> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::activation::activate()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Err(OsError::Unsupported)
    }
}
