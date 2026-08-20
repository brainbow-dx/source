//! A drop-in replacement for the real `libsqlite3-sys` crate (see this workspace's root
//! `Cargo.toml`'s `[patch.crates-io]`), reusing its exact FFI declarations (copied verbatim from
//! its own checked-in `bindgen_bundled_version.rs`, minus a handful of functions libsql's fork
//! doesn't implement — see `bindings.rs`'s own `// omitted:` comments) and, per `build.rs`,
//! compiling `libsql-ffi`'s own vendored SQLite C source verbatim, rather than building the real
//! `libsqlite3-sys`'s own vanilla-upstream vendored copy.
//!
//! `rusqlite` (pulled in transitively by `deno_runtime`'s `deno_kv`/`deno_cache`/
//! `deno_node_sqlite`/`deno_webstorage`, all of which hard-require `rusqlite`'s own `bundled`
//! feature in their own published `Cargo.toml`s, so this can't be avoided by feature selection
//! alone) calls these exact symbol names. Building the real `libsqlite3-sys` would vendor a
//! *second*, separately-compiled copy of SQLite exporting the exact same public symbol names as
//! `libsql-ffi` (via the `libsql` crate, Anvil's own sqld/persistence layer) already does. A
//! linker can only resolve that by silently keeping one implementation and discarding the other
//! for *every* caller — and there's no guarantee it keeps the right one: vanilla SQLite's
//! `sqlite3_open` etc. can silently win over libsql's own, and libsql's replication code jumps
//! through an internal hook vanilla SQLite never defines, causing a real segfault rather than a
//! cosmetic linker warning.
//!
//! This crate exists so the *other* copy — wherever `rusqlite` needs one — is always libsql's
//! own fork instead of vanilla upstream SQLite, by compiling that exact same fork's source itself
//! rather than assuming `libsql-ffi` happens to already be linked into the same binary (that
//! assumption breaks for build units that pull `rusqlite` in without `libsql` anywhere in their
//! own graph — `ethos-deno`'s V8-snapshot-generating build script is one such case). Duplicate
//! symbols still exist in `escher-anvil`'s own final binary (this crate's compiled object and
//! `libsql-ffi`'s own), but since both come from byte-identical source built with the same flags,
//! it no longer matters which one the linker keeps.
//!
//! Remove this crate and its `[patch.crates-io]` entry once `deno_kv`/`deno_cache`/
//! `deno_node_sqlite`/`deno_webstorage` stop hard-requiring `rusqlite`'s `bundled` feature
//! upstream (see `escher/spec/ROADMAP.md` for tracking).
#![expect(non_snake_case, non_camel_case_types)]
#![cfg_attr(not(test), no_std)]

pub use self::error::*;

use core::mem;

mod error;

#[must_use]
pub fn SQLITE_STATIC() -> sqlite3_destructor_type {
    None
}

#[must_use]
pub fn SQLITE_TRANSIENT() -> sqlite3_destructor_type {
    Some(unsafe { mem::transmute::<isize, unsafe extern "C" fn(*mut core::ffi::c_void)>(-1_isize) })
}

#[allow(dead_code, clippy::all)]
mod bindings {
    include!("bindings.rs");
}
pub use bindings::*;

impl Default for sqlite3_vtab {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}

impl Default for sqlite3_vtab_cursor {
    fn default() -> Self {
        unsafe { mem::zeroed() }
    }
}
