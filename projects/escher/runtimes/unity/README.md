# Escher Unity integration

Runs JavaScript inside Unity (Editor and Player) via `ethos-ecma`'s Deno-based runtime, over the
same C ABI already used by `ethos-ecma`'s own examples (`counter`, etc.) and by `runtimes/unreal`
eventually. This crate is a normal member of escher's main workspace — it has **no Rust
dependency on `ethos-ecma`**, direct or otherwise. The native library Unity actually loads is
`ethos-ecma`'s own cdylib, built independently from ethos's own workspace by
`scripts/sync-plugin.sh` (below), not by anything this crate compiles or links. An earlier version
of this crate carried a `pub use ethos_ecma::runtime::ffi as ffi;` convenience re-export with zero
real consumers — that pulled `deno_runtime`'s build-time `swc_allocator` (needs bumpalo's
`allocator-api2` feature) into escher's main workspace `Cargo.lock`, conflicting with escher-core's
own nightly `allocator_api` bumpalo feature (the two are mutually exclusive in bumpalo's own
source — confirmed in `bumpalo`'s `src/lib.rs`). Removing the unused dependency, rather than
isolating this crate into its own workspace, keeps it consistent with every other `runtimes/*`
crate instead of secretly not belonging to escher's workspace despite living inside its directory
tree. Rebuilt from scratch 2026-08-13 — the old RecLab/Aby project's Unity integration
(researched from `/Volumes/Bob`'s backup) never actually worked: every native call site across its
hand-written C# (`Runtime.cs`, `Executor.cs`, the Editor windows) was commented out, and the
project's own roadmap doc listed Unity as an unfinished "secondary support target." This isn't a
port of that code — it's new code, using the old project's *design* (the FFI struct shapes, the
Editor lifecycle hook points it correctly identified but never wired up) as a reference, not its
implementation.

## Layout

- `scripts/sync-plugin.sh` — builds `ethos-ecma`'s cdylib (`cargo build -p ethos-ecma --features
  ffi`, run from `dialects/ecma` itself) and copies it, plus its `csbindgen`-generated C#
  bindings, into `Assets/Plugins/Escher/`. Fixes real bugs in the old project's equivalent script
  (wrong crate directory, wrong output filename, Windows-only despite listing the other two
  platforms in a comment) — this version handles macOS/Linux/Windows naming for real.
- `Assets/Plugins/Escher/EcmaRuntime.g.cs` — **generated, not committed** — produced by
  `sync-plugin.sh`. Don't hand-edit it.
- `Assets/Plugins/Escher/Runtime.cs` — `EscherRuntime`, a static class owning one global runtime
  instance: construct, execute a module, free. Includes the log-callback delegate correctly kept
  alive as a static field (GC-pinning the old code left as a commented-out TODO) and the Player
  bootstrap hook (`RuntimeInitializeOnLoadMethod(BeforeSceneLoad)`).
- `Assets/Plugins/Escher/Editor/EditorLifecycle.cs` — Editor-only: tears the runtime down before a
  domain reload / on quit (a stale native pointer after reload is a guaranteed crash), and
  bootstraps/tears down around entering/exiting Play Mode.

## What's actually verified, and what isn't

**Verified, for real**, in this sandbox (no Unity Editor installed here — only Unity Hub, which
manages Editor installs but doesn't include one by default; installing a multi-GB Editor version
non-interactively wasn't attempted):

- `ethos-ecma`'s cdylib builds successfully (`cargo build -p ethos-ecma --features ffi --lib`).
- The `csbindgen`-generated `EcmaRuntime.g.cs` is syntactically correct, real C# — confirmed by
  actually compiling it (via `dotnet build`, no Unity APIs involved, just the raw P/Invoke
  declarations and structs).
- **The full FFI round-trip works end to end from C#**: a standalone (non-Unity) C# console app
  P/Invoked into the real built `libethos_ecma.dylib`, called `c_construct_runtime` →
  `c_exec_module` → `c_free_runtime`, and a real JS module actually executed — its `console.log`
  output appeared in the C# process's stdout, and the native log callback (with the delegate
  correctly GC-pinned via a static field, the exact thing RecLab's code never finished) fired
  correctly back into C#. This is the identical P/Invoke pattern `Runtime.cs` uses.

**Not verified** — genuinely can't be, without installing a Unity Editor:

- `Runtime.cs`/`EditorLifecycle.cs` compiling against real `UnityEngine`/`UnityEditor` assemblies.
- The `RuntimeInitializeOnLoadMethod`/`InitializeOnLoad`/`AssemblyReloadEvents`/
  `playModeStateChanged` attributes and events actually firing as expected inside a real Editor or
  Player.
- Any behavior specific to Unity's own execution environment (its own AppDomain/assembly loading,
  IL2CPP vs. Mono scripting backends, etc.).

If you have Unity installed: `bash scripts/sync-plugin.sh`, then open this directory as (or copy
`Assets/Plugins/Escher/` into) a Unity project and press Play — that's the real test this
environment couldn't run.
