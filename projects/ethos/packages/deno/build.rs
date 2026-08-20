use std::fs::File;
use std::io::Write as _;
use std::path::PathBuf;

use deno_runtime::deno_core::ModuleCodeString;
use deno_runtime::deno_core::ModuleName;
use deno_runtime::snapshot::LazyExtensionFile;
use deno_runtime::snapshot::LazyExtensionFileKind;
use deno_runtime::transpile::maybe_transpile_source;

/// Builds a V8 startup snapshot embedding deno's own base extensions (`deno_telemetry`, etc.).
/// Without one, those extensions fall back to loading their raw, unbundled TypeScript at
/// runtime, which V8 can't parse. Dialect-specific extensions (e.g. `dialects/ecma`'s
/// `aby_sdk`) are layered on top at `WorkerOptions` construction time, not baked into this
/// snapshot — it only needs to match this crate's own `deno_core`/`deno_runtime` versions, not
/// any particular dialect's.
fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let snapshot_path = out_dir.join("DENO_SNAPSHOT.bin");
    let output = deno_runtime::snapshot::create_runtime_snapshot(snapshot_path, Default::default(), Vec::new());

    // Not every `lazy_loaded_*` file an extension declares actually ends up with its source
    // embedded in the snapshot blob itself (`output.consumed_lazy_specifiers`) — the rest
    // (`lazy_extension_files` minus that set — deno_runtime's own doc comment calls this the
    // "residual" set) still needs its raw source available *somewhere* in the final binary, or
    // `op_lazy_load_esm`/`op_load_ext_script` fails at runtime with "cannot be lazy-loaded as it
    // was not included in the binary" the moment anything actually exercises that lazy module —
    // confirmed live: `deno_fetch`'s own `ext:deno_fetch/26_fetch.js` is one of these, so any
    // embedded JS command calling `fetch()` failed outright before this fix.
    // `WorkerOptions::residual_lazy_esm_sources`/`residual_lazy_js_sources` is exactly the
    // embedder-supplied slot `deno_core`/`deno_runtime` expect this in — generated here as a
    // build-time `include!`-able file, since which files fall into the residual set is an
    // implementation detail of `deno_runtime`'s own extension list that this crate has no reason
    // to track by hand.
    let consumed: std::collections::HashSet<&str> = output.consumed_lazy_specifiers.iter().map(String::as_str).collect();
    let is_residual = |file: &&LazyExtensionFile| !consumed.contains(file.specifier.as_str());

    let mut esm_files: Vec<&LazyExtensionFile> = output.lazy_extension_files.iter().filter(|file| file.kind == LazyExtensionFileKind::Esm).filter(is_residual).collect();
    let mut js_files: Vec<&LazyExtensionFile> = output.lazy_extension_files.iter().filter(|file| file.kind == LazyExtensionFileKind::Js).filter(is_residual).collect();
    // `LazyEsmModuleLoader`/its `Js`-kind equivalent resolve a residual specifier via
    // `binary_search_by` — the generated arrays below have to be sorted for that to work, not
    // just for a stable diff.
    esm_files.sort_by(|a, b| a.specifier.cmp(&b.specifier));
    js_files.sort_by(|a, b| a.specifier.cmp(&b.specifier));

    let generated_path = out_dir.join("residual_lazy_sources.rs");
    let mut generated = File::create(&generated_path).expect("failed to create residual_lazy_sources.rs");
    write_residual_array(&mut generated, "RESIDUAL_LAZY_ESM_SOURCES", &esm_files);
    write_residual_array(&mut generated, "RESIDUAL_LAZY_JS_SOURCES", &js_files);
}

fn write_residual_array(file: &mut File, name: &str, entries: &[&LazyExtensionFile]) {
    writeln!(file, "pub static {name}: &[(&str, &str)] = &[").unwrap();
    for entry in entries {
        let path = entry.path.display().to_string();
        let raw_source = std::fs::read_to_string(&entry.path).unwrap_or_else(|error| panic!("failed to read {path}: {error}"));
        // Real transpilation, not a plain `include_str!` of the file on disk — the exact same
        // step `create_runtime_snapshot`'s own `extension_transpiler` callback applies to every
        // *other* source it embeds. `.js`/`.mjs` residuals pass through unchanged (confirmed:
        // `maybe_transpile_source` is a no-op for those), but a residual declared against a
        // `.ts` file (`ext:deno_telemetry/telemetry.ts`, confirmed live as a real one) is genuine
        // TypeScript — embedding its raw source verbatim fed V8 real type-annotation syntax
        // (`Unexpected token ':'`) the moment anything actually lazy-loaded it.
        println!("cargo:rerun-if-changed={path}");
        let (transpiled, _source_map) = maybe_transpile_source(ModuleName::from(entry.specifier.clone()), ModuleCodeString::from(raw_source))
            .unwrap_or_else(|error| panic!("failed to transpile residual lazy source {path}: {error}"));
        writeln!(file, "    ({:?}, {:?}),", entry.specifier, transpiled.as_str()).unwrap();
    }
    writeln!(file, "];").unwrap();
}
