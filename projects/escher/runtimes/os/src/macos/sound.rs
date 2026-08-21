use objc2_app_kit::NSSound;

use objc2_foundation::NSString;

use crate::OsError;

pub fn play(name: &str) -> Result<(), OsError> {
    let Some(sound) = NSSound::soundNamed(&NSString::from_str(name)) else {
        return Err(OsError::NotFound(name.to_string()));
    };

    // Fire-and-forget, same as the system's own `afplay`/Sound Effects preview — whether it's
    // actually audible (system volume, muted output) isn't this call's concern to report.
    sound.play();

    Ok(())
}
