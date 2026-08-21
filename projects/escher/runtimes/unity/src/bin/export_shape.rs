//! Runs the Ethos-side UXML/USS codegen tool (`ethos/tools/codegen/uxml/shape-demo.ts`, invoked
//! via `ethos-cli run-command`, the same way `apps/anvil` already invokes any other Ethos script)
//! and writes its output into Aby's Unity project as new asset files. See
//! `projects/ethos/spec/agents/proposals/uxml-uss-codegen.md` for why the actual codegen logic
//! lives in Ethos, not here. This binary is a thin consumer: run the script, parse its JSON
//! stdout, write two files. It has no `ethos-ecma`/`ethos-deno` Rust dependency (see this crate's
//! `Cargo.toml` doc comment for why that matters), just a subprocess call, like `codegen.rs`'s
//! `csbindgen` use is a separate, unrelated kind of "generate output" step.
//!
//! Run directly: `cargo run -p escher-unity --bin export_shape`.
//!
//! Deliberately writes only new files under `Assets/UI/Generated/`. It makes no `.cs` change and
//! no `.unity` scene edit, since Aby's Unity Editor may be open interactively when this runs (new
//! non-script assets don't trigger a domain reload the way a `.cs` change would).

use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

const ETHOS_SCRIPT: &str = "tools/codegen/uxml/shape-demo.ts";

fn main() {
    let escher_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let ethos_root = escher_root.join("../../../ethos").canonicalize().expect("ethos checkout must exist as a sibling of escher");
    let unity_project = escher_root.join("../../../aby/runtimes/unity").canonicalize().expect("aby's unity project must exist as a sibling of escher");

    let ethos_cli = ensure_ethos_cli_built(&ethos_root).expect("failed to build ethos-cli");

    let output = Command::new(&ethos_cli)
        .args(["run-command", ETHOS_SCRIPT])
        .current_dir(&ethos_root)
        .output()
        .expect("failed to run ethos-cli run-command");

    if !output.status.success() {
        panic!("ethos-cli run-command failed:\n{}", String::from_utf8_lossy(&output.stderr));
    }

    let stdout = String::from_utf8(output.stdout).expect("ethos-cli stdout must be valid UTF-8");
    let json: serde_json::Value = serde_json::from_str(stdout.trim()).expect("script output must be valid JSON");

    let uxml = json.get("uxml").and_then(serde_json::Value::as_str).expect("missing `uxml` field in script output");
    let uss = json.get("uss").and_then(serde_json::Value::as_str).expect("missing `uss` field in script output");

    let output_dir = unity_project.join("Assets/UI/Generated");
    std::fs::create_dir_all(&output_dir).expect("failed to create Assets/UI/Generated");

    let uxml_path = output_dir.join("Shape.uxml");
    let uss_path = output_dir.join("Shape.uss");
    std::fs::write(&uxml_path, uxml).expect("failed to write Shape.uxml");
    std::fs::write(&uss_path, uss).expect("failed to write Shape.uss");

    println!("Wrote {} and {}", uxml_path.display(), uss_path.display());
}

/// Same build+locate pattern `apps/anvil/src/main.rs`'s `ensure_ethos_cli_built` uses. Kept as
/// its own copy here rather than shared, since this is a standalone one-shot binary, not a
/// library `apps/anvil` could depend on without pulling in that whole crate.
fn ensure_ethos_cli_built(ethos_root: &Path) -> Result<PathBuf, String> {
    let output = Command::new("cargo")
        .args(["build", "-p", "ethos-cli", "--bin", "ethos", "--quiet"])
        .current_dir(ethos_root)
        .output()
        .map_err(|error| format!("failed to build ethos-cli: {error}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(ethos_root.join(".cargo/target/debug/ethos"))
}
