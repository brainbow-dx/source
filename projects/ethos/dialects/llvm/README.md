# LLVM dialect (not a workspace member yet)

Not listed in the root `Cargo.toml`'s `[workspace] members` — `main.rs` depends on `inkwell`, which needs a version-matched LLVM install (`llvm-config` on `PATH`); this machine has `clang` but no `llvm-config`, so this hasn't been gotten building. Ported here (2026-08-14, originally `legacy/examples/franken`, a pre-refactor snapshot) as reference material for whoever picks this up, not as working code today.

See `../../spec/Dialects.md` for what this crate is meant to become and how it relates to the dialects that already exist.

`main.rs` hardcodes a Windows path (`C:/LLVM/16.0.5/bin/clang.exe`) from whatever machine it was last run on; that'll need fixing for whatever machine eventually builds this for real.
