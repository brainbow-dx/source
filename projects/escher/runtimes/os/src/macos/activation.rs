use objc2::MainThreadMarker;
use objc2_app_kit::NSApplication;

use crate::OsError;

/// Brings this whole app (not just one of its own windows) to the front, above whatever other
/// app currently has focus — a raw binary launched from a terminal (not a real `.app` bundle)
/// isn't always treated as "the active app" by macOS the same way a Dock-launched app is, so a
/// window-level `winit::window::Window::focus_window()` alone can reorder a window among this
/// app's *own* windows without actually stealing focus from, say, the terminal emulator the user
/// launched it from. Confirmed live as the real gap behind exactly that symptom.
pub fn activate() -> Result<(), OsError> {
    let mtm = MainThreadMarker::new().ok_or(OsError::NotOnMainThread)?;
    NSApplication::sharedApplication(mtm).activate();
    Ok(())
}
