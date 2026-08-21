//! Runs plain JS source via `DenoRuntime`, the simple `ethos_core::Runtime` implementation —
//! bare script execution, no module resolution, no extensions. This is the ergonomic path;
//! `examples/counter` demonstrates the lower-level FFI surface hosts like Unity call into
//! instead.

use ethos_core::Runtime;
use ethos_deno::worker::DenoRuntime;

fn main() {
    let mut runtime = DenoRuntime;

    let output = runtime.execute("[1, 2, 3, 4, 5].filter(n => n % 2 === 0).join(', ')").expect("execute");

    println!("{output}");
}
