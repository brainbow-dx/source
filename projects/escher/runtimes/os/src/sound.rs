//! Named system sounds only (the same set visible in System Settings > Sound > Sound Effects,
//! e.g. `"Glass"`, `"Ping"`, `"Pop"`) — not arbitrary audio file playback, which belongs to
//! whichever runtime actually needs it (Bevy's own audio, say), not this OS-integration crate.

use crate::OsError;

pub fn play(name: &str) -> Result<(), OsError> {
    #[cfg(target_os = "macos")]
    {
        crate::macos::sound::play(name)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = name;
        Err(OsError::Unsupported)
    }
}
