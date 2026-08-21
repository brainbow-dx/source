//! Guards against `escher/patches/libsqlite3-sys`'s fix silently stopping to apply. That patch
//! only takes effect for dependency requests it satisfies (see its own `Cargo.toml`'s pinned
//! version). If a future update ever needs a newer `libsqlite3-sys` than it declares, Cargo
//! quietly falls back to the real, upstream-vanilla crate instead of erroring, which reintroduces
//! the segfault the patch exists to prevent. That means two different vendored SQLite forks in one
//! binary; see the patch crate's own `src/lib.rs` doc comment. This only runs via `cargo test`, a
//! build/CI-time gate. It is never compiled into a shipped release binary.

#[test]
fn embedded_sqlite_is_libsqls_own_fork() {
    let version = libsql::version_number();
    assert_eq!(
        version, 3_045_001,
        "linked SQLite reports version {version} ({}), not libsql's own fork (3045001, i.e. \
         3.45.1) — escher/patches/libsqlite3-sys has stopped applying, which means Anvil's \
         persistence will segfault (see that patch crate's own doc comment for the full story)",
        libsql::version(),
    );
}
