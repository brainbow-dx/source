use std::path::PathBuf;

/// Builds a V8 startup snapshot embedding deno's own base extensions (`deno_telemetry`, etc.).
/// Without one, those extensions fall back to loading their raw, unbundled TypeScript at
/// runtime — V8 has no TypeScript support, so that fails to parse. Dialect-specific extensions
/// (e.g. `dialects/ecma`'s `aby_sdk`) are layered on top at `WorkerOptions` construction time,
/// not baked into this snapshot — it only needs to match this crate's own `deno_core`/
/// `deno_runtime` versions, not any particular dialect's.
fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let snapshot_path = out_dir.join("DENO_SNAPSHOT.bin");
    deno_runtime::snapshot::create_runtime_snapshot(snapshot_path, Default::default(), Vec::new());
}
