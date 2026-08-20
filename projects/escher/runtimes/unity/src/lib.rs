//! No Rust dependency on `ethos-ecma` here — see this crate's `Cargo.toml` doc comment. The
//! actual native library Unity loads (`ethos-ecma`'s cdylib) is built directly from ethos's own
//! workspace by `scripts/sync-plugin.sh`, independent of anything this crate compiles or links.
//! This crate is the home for the Unity `Assets/` integration and its build/sync tooling.
