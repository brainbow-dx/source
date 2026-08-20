//! Interactive, human-watched verification of `escher-os::dialog` — deliberately separate from
//! `examples/demo.rs`, which stays headless-safe on purpose (see its own doc comment for why it
//! skips exactly this). Run with `cargo run -p escher-os --example dialogs` and actually look at
//! the screen; this blocks on real clicks.

fn main() {
    println!("Showing alert() — click OK to continue...");
    if let Err(error) = escher_os::dialog::alert("Escher OS", "This is a plain alert() dialog.") {
        eprintln!("alert() failed: {error}");
    }

    println!("Showing confirm() — click OK or Cancel...");
    match escher_os::dialog::confirm("Escher OS", "This is a confirm() dialog. Did you click OK?") {
        Ok(true) => println!("confirm() returned true (OK)"),
        Ok(false) => println!("confirm() returned false (Cancel)"),
        Err(error) => eprintln!("confirm() failed: {error}"),
    }
}
