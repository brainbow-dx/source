// use tracing::Level;

//---
/// Default filter for a normal run: quiet third-party crates, informational for `ethos` itself.
#[cfg(all(not(feature = "debug"), not(feature = "verbose")))]
pub const DEFAULT_LOG_FILTER: &str = "error,ethos_core=info,ethos=info";

/// Default filter under the `debug` feature.
#[cfg(all(feature = "debug", not(feature = "verbose")))]
pub const DEFAULT_LOG_FILTER: &str = "warn,ethos_core=debug,ethos=debug";

/// Default filter under the `verbose` feature.
#[cfg(all(not(feature = "debug"), feature = "verbose"))]
pub const DEFAULT_LOG_FILTER: &str = "info,ethos_core=trace,ethos=trace";

//---
/// Init a basic global logger with a few configurable bells-n-whistles.
pub fn init(filter: &str) {
    #[cfg(feature = "profiling")]
    {
        use tracy_client::Client;
        let _client = Client::start();
        // Ensure client started & (optionally) start a gui ..
    }

    #[cfg(feature = "std")]
    {
        // TODO: Make the tracing subscriber configurable via tracing builder.

        use tracing_subscriber::FmtSubscriber;
        use tracing_subscriber::fmt::time;

        let subscriber = FmtSubscriber::builder()
            .with_env_filter(filter)
            .with_timer(time::uptime())
            .with_ansi(true)
            .with_level(true)
            .with_thread_names(false)
            .with_thread_ids(false)
            .with_target(true)
            .with_file(false)
            .with_line_number(false)
            .finish();

        tracing::subscriber::set_global_default(subscriber)
            .expect("setting default subscriber failed");
    }
}
