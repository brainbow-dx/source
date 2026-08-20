use objc2::MainThreadMarker;

use objc2_app_kit::NSAlert;
use objc2_app_kit::NSAlertFirstButtonReturn;

use objc2_foundation::NSString;

use crate::OsError;

pub fn alert(title: &str, message: &str) -> Result<(), OsError> {
    let mtm = MainThreadMarker::new().ok_or(OsError::NotOnMainThread)?;

    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(message));
    alert.addButtonWithTitle(&NSString::from_str("OK"));
    alert.runModal();

    Ok(())
}

pub fn confirm(title: &str, message: &str) -> Result<bool, OsError> {
    let mtm = MainThreadMarker::new().ok_or(OsError::NotOnMainThread)?;

    let alert = NSAlert::new(mtm);
    alert.setMessageText(&NSString::from_str(title));
    alert.setInformativeText(&NSString::from_str(message));
    alert.addButtonWithTitle(&NSString::from_str("OK"));
    alert.addButtonWithTitle(&NSString::from_str("Cancel"));

    Ok(alert.runModal() == NSAlertFirstButtonReturn)
}
