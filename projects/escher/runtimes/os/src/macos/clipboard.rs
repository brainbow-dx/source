use objc2_app_kit::NSPasteboard;
use objc2_app_kit::NSPasteboardTypeString;

use objc2_foundation::NSString;

use crate::OsError;

pub fn read_text() -> Result<Option<String>, OsError> {
    let pasteboard = NSPasteboard::generalPasteboard();
    // SAFETY: `NSPasteboardTypeString` is a valid, always-initialized system constant.
    let text = pasteboard.stringForType(unsafe { NSPasteboardTypeString });
    Ok(text.map(|s| s.to_string()))
}

pub fn write_text(text: &str) -> Result<(), OsError> {
    let pasteboard = NSPasteboard::generalPasteboard();
    pasteboard.clearContents();
    // SAFETY: `NSPasteboardTypeString` is a valid, always-initialized system constant.
    pasteboard.setString_forType(&NSString::from_str(text), unsafe { NSPasteboardTypeString });
    Ok(())
}
