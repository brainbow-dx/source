#[cfg(not(any(feature = "debug", feature = "verbose")))]
pub const DEFAULT_LOG_FILTER: &str = "warn,escher_bevy=info,escher=info";

#[cfg(all(feature = "debug", not(feature = "verbose")))]
pub const DEFAULT_LOG_FILTER: &str = "debug,escher_bevy=debug,escher=debug,wgpu_core=warn,wgpu_hal=warn";

#[cfg(all(feature = "debug", feature = "verbose"))]
pub const DEFAULT_LOG_FILTER: &str = "trace,escher_bevy=trace,escher=trace,wgpu_core=error,wgpu_hal=error";
