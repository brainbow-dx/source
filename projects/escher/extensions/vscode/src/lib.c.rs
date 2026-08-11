// use 

#[derive(Default)]
// #[cwrap(c, wasm, ts)]
pub struct Config<'cfg> {
    env_filter: Option<&'cfg str>,
}

// #[cwrap(c, wasm, ts)]
pub unsafe fn init(config: Config) {
    let filter = config.env_filter.unwrap_or("trace");
    
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_thread_names(false)
        .with_line_number(false)
        .with_target(false)
        .with_file(false)
        .with_ansi(true)
        .without_time()
        .init();
}