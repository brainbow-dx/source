//! Running a single JS/TS module's exported `run(args)` and getting back its (stringified)
//! result — the actual embedding contract every host of this crate shares (`ethos-cli`'s
//! `run-command`, `escher-anvil`'s JS-backed slash commands). Factored out here, instead of each
//! host reimplementing the same `MainWorker`/module-evaluate/V8-scope dance against `deno_core`/
//! `deno_runtime` directly, so a new host only ever needs this crate's own API surface.

use std::path::Path;

use deno_core::v8;
use deno_core::Extension;
use deno_core::PollEventLoopOptions;
use deno_runtime::deno_core::resolve_url_or_path;
use deno_runtime::deno_io::Stdio;

use crate::worker::bootstrap_main_worker;

/// Loads `script` as an ES module (resolved against `current_dir`), evaluates it, calls its
/// exported `<export_name>(args)`, and returns the (stringified) result — anything the script
/// itself `console.log`s along the way goes wherever `stdio.stdout`/`stdio.stderr` point, not
/// into this return value.
///
/// `export_name` is almost always `"run"` (the normal per-invocation entry point every command
/// script has), but is a parameter rather than hardcoded so a host can call a *different* exported
/// function against the same script for a distinct lifecycle moment — see `escher-anvil`'s own
/// `onLoad` convention (run once at startup, when a command is discovered, rather than per
/// invocation) for the reason this exists.
///
/// `extensions` are registered on the worker before evaluation, same as `bootstrap_main_worker`'s
/// own parameter of the same name — this is how a host gives a script real callable functions
/// (see `ethos_sdk`'s own `op_send_host_log` for the pattern), rather than the script signaling
/// intent back through its return value for the host to interpret. Pass an empty `Vec` for a
/// script that only needs stdout/stderr and a return value, no host actions.
///
/// Builds its own dedicated single-threaded Tokio runtime — `deno_unsync` (used internally by
/// `deno_fetch`) asserts the runtime driving it is `CurrentThread` — so callers don't need a
/// Tokio context of their own at all; this blocks the calling thread until the script's
/// `<export_name>` returns.
pub fn run_module_command(script: &Path, current_dir: &Path, args: &str, stdio: Stdio, extensions: Vec<Extension>, export_name: &str) -> Result<String, String> {
    let tokio_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|error| format!("failed to build a runtime for the embedded JS engine: {error}"))?;

    tokio_runtime.block_on(async move {
        let main_module = resolve_url_or_path(&script.to_string_lossy(), current_dir)
            .map_err(|error| format!("{script:?} is not a valid module specifier: {error}"))?;

        let mut worker = bootstrap_main_worker(&main_module, stdio, None, extensions, false, None);

        let module_id = worker.preload_main_module(&main_module).await.map_err(|error| format!("failed to preload {script:?}: {error}"))?;
        worker.evaluate_module(module_id).await.map_err(|error| format!("failed to evaluate {script:?}: {error}"))?;
        worker
            .js_runtime
            .run_event_loop(PollEventLoopOptions::default())
            .await
            .map_err(|error| format!("{script:?}'s event loop failed: {error}"))?;

        let namespace = worker.js_runtime.get_module_namespace(module_id).map_err(|error| format!("{script:?}: {error}"))?;

        let (run_function, arg_value) = {
            deno_core::scope!(scope, worker.js_runtime);
            let namespace_local = v8::Local::new(scope, &namespace);

            let key = v8::String::new(scope, export_name).ok_or_else(|| format!("failed to allocate {export_name:?} key"))?;
            let run_value = namespace_local.get(scope, key.into()).ok_or_else(|| format!("{script:?} does not export {export_name:?}"))?;
            let run_function =
                v8::Local::<v8::Function>::try_from(run_value).map_err(|_| format!("{script:?}'s exported {export_name:?} is not a function"))?;

            let arg_string = v8::String::new(scope, args).ok_or_else(|| "failed to allocate arg string".to_string())?;
            let arg_value: v8::Local<v8::Value> = arg_string.into();

            (v8::Global::new(scope, run_function), v8::Global::new(scope, arg_value))
        };

        #[allow(deprecated, reason = "call_with_args (the replacement) needs the caller to drive the event loop manually; this one already does it, and that's exactly what's needed here")]
        let result = worker
            .js_runtime
            .call_with_args_and_await(&run_function, &[arg_value])
            .await
            .map_err(|error| format!("{script:?}'s {export_name:?} threw: {error}"))?;

        let result_string = {
            deno_core::scope!(scope, worker.js_runtime);
            let result_local = v8::Local::new(scope, &result);
            if result_local.is_undefined() || result_local.is_null() {
                String::new()
            } else {
                result_local.to_rust_string_lossy(scope)
            }
        };

        Ok(result_string)
    })
}
