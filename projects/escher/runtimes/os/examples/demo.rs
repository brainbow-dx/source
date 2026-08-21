//! Verification example for `escher-os`: writes to the system clipboard, reads it back, and sets
//! a standard application menu bar. Doesn't call `dialog::alert`/`confirm` — those block on real
//! user interaction (a modal dialog), which can't be driven from an automated/headless run; their
//! implementation is a handful of straightforward, standard `NSAlert` calls (see `src/macos/
//! dialog.rs`) with nothing dialog-specific left to prove out beyond what this example already
//! exercises (`MainThreadMarker` access, `NSString` round-tripping).

fn main() {
    match escher_os::clipboard::write_text("escher-os clipboard round-trip") {
        Ok(()) => println!("Wrote to clipboard"),
        Err(error) => eprintln!("Failed to write clipboard: {error}"),
    }

    match escher_os::clipboard::read_text() {
        Ok(Some(text)) => println!("Read from clipboard: {text:?}"),
        Ok(None) => println!("Clipboard has no text"),
        Err(error) => eprintln!("Failed to read clipboard: {error}"),
    }

    let menu = escher_os::menu::default_application_menu("Escher OS Demo");
    match escher_os::menu::set_application_menu(&menu) {
        Ok(()) => println!("Set application menu"),
        Err(error) => eprintln!("Failed to set application menu: {error}"),
    }
}
