//! Finding another binary this process wants to run — the cargo/git-subcommand convention
//! (`escher-cli` looking up `escher-anvil`, `apps/anvil` looking up `ethos-cli`, and presumably
//! more callers later) factored out to one place instead of copy-pasted per caller. Genuinely
//! cross-platform (no `objc2`/AppKit involved at all) — lives in this crate anyway since it's
//! still "OS-level integration that isn't specific to any one engine," this crate's own doc
//! comment's exact description, just via `std::env`/`std::fs` instead of a native API.

use std::env;
use std::path::PathBuf;

/// Looks for `name` right next to the *running* executable first (the real, self-contained
/// release-install case — two binaries shipped side by side), then anywhere on `PATH` (a `cargo
/// install`-style setup, one directory holding every relevant binary without also holding this
/// exact executable). `None` if neither exists — the caller decides what "not found" means for
/// it (a hard error, a dev-checkout build-from-source fallback, silently skipping the feature
/// that needed it, ...); this function only ever answers "is a real binary sitting there."
pub fn find_sibling_or_path(name: &str) -> Option<PathBuf> {
    if let Ok(exe) = env::current_exe()
        && let Some(dir) = exe.parent()
    {
        let sibling = dir.join(name);
        if sibling.is_file() {
            return Some(sibling);
        }
    }

    if let Some(path_var) = env::var_os("PATH") {
        for dir in env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}
