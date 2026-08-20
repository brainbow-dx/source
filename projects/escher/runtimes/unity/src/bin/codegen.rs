//! Generates the C# bindings Unity loads (`EcmaRuntime.g.cs`) from `ethos-deno`'s FFI surface.
//!
//! Lives here, not in `ethos-deno` itself, because Unity-specific concerns (the C# namespace,
//! the `libecma`/`__Internal` DLL name split, the output shape) have nothing to do with what
//! `ethos-deno` is — this crate is where Unity integration belongs. `ethos-deno` only needs to
//! expose its `extern "C"` surface (see its `ffi` feature); it stays unaware that Unity exists.
//! `csbindgen` only parses the given source files with `syn` — it doesn't compile or link
//! `ethos-deno`, so this has no Cargo dependency on it, just a relative path to its sources.
//!
//! Run via `scripts/sync-plugin.sh`, not directly.

const ETHOS_DENO_SRC: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../ethos/packages/deno/src");
const OUTPUT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/.output/EcmaRuntime.g.cs");

fn main() {
    csbindgen::Builder::default()
        .input_extern_file(format!("{ETHOS_DENO_SRC}/lib.rs"))
        .input_extern_file(format!("{ETHOS_DENO_SRC}/bootstrap.rs"))
        .input_extern_file(format!("{ETHOS_DENO_SRC}/logging.rs"))
        .input_extern_file(format!("{ETHOS_DENO_SRC}/tracing.rs"))
        .input_extern_file(format!("{ETHOS_DENO_SRC}/event.rs"))
        .input_extern_file(format!("{ETHOS_DENO_SRC}/runtime.rs"))
        .input_extern_file(format!("{ETHOS_DENO_SRC}/start.rs"))
        .csharp_dll_name("libecma")
        .csharp_dll_name_if("UNITY_IOS && !UNITY_EDITOR", "__Internal")
        .csharp_namespace("Unity.Runtime")
        .csharp_class_accessibility("public")
        .csharp_class_name("EcmaRuntime")
        .csharp_use_function_pointer(false)
        .generate_csharp_file(OUTPUT_PATH)
        .expect("Failed to generate CSharp bindings.");

    println!("Generated {OUTPUT_PATH}");
}
