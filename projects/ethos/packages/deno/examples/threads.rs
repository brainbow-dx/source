//! Nesting a single-threaded Tokio runtime inside a spawned OS thread, communicating back to the
//! spawning thread via `std::sync::mpsc` — the same shape `EcmaRuntimeManager` itself uses
//! internally (a dedicated OS thread driving its own `tokio::runtime::Runtime`), useful as a
//! minimal reference when embedding the runtime in a host that already has its own threading
//! model and doesn't want the JS runtime's async work sharing a runtime with anything else.

use tokio::runtime::Builder as TokioRuntimeBuilder;

const TRACING_FILTER: &str = "threads=trace,warn";

fn main() {
    ethos_deno::tracing::mount(TRACING_FILTER);

    let (tx, rx) = std::sync::mpsc::channel::<i32>();

    let child = std::thread::spawn(move || {
        let async_executor = TokioRuntimeBuilder::new_current_thread().enable_all().build().unwrap();

        async_executor.block_on(async {
            tracing::debug!("Running inside the single-threaded runtime");
        });

        tracing::debug!("Back to the multi-threaded runtime");

        tx.send(1).unwrap();
    });

    tracing::info!("Got message: {:}", rx.recv().unwrap());

    child.join().unwrap();
}
