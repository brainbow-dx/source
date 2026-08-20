use tracing_subscriber::FmtSubscriber;

/// Init a basic global logger with a few configurable bells-n-whistles.
pub fn init(filter: &str) {
    let subscriber = FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_timer(tracing_subscriber::fmt::time::uptime())
        .with_ansi(true)
        .with_level(true)
        .with_thread_names(false)
        .with_thread_ids(false)
        .with_target(true)
        .with_file(false)
        .with_line_number(false)
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("setting default subscriber failed");
}
