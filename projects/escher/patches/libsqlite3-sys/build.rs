//! Compiles `bundled/sqlite3.c` — a verbatim copy of `libsql-ffi` 0.9.30's own vendored SQLite
//! fork source (see `src/lib.rs`'s doc comment for why this crate exists at all). Deliberately
//! *not* "rely on `libsql-ffi` being linked into the same binary elsewhere": that assumption
//! holds for `escher-anvil`'s own final binary (which also depends on `libsql`), but breaks for
//! any build unit that pulls `rusqlite` in without `libsql`/`libsql-ffi` anywhere in its own
//! graph — `ethos-deno`'s build script is one such case (its V8 snapshot-generation step links
//! `deno_runtime`'s full extension set, including `rusqlite`, on its own, producing undefined-
//! symbol errors with no vendored SQLite anywhere else to satisfy them). Compiling this crate's
//! own copy makes it self-sufficient in every link unit, not just the ones that happen to already
//! have `libsql-ffi` present.
//!
//! This does mean `escher-anvil`'s own final binary ends up with *two* independently-compiled
//! objects exporting the same `sqlite3_*` symbol names again (this crate's, and `libsql-ffi`'s
//! own) — but since both are compiled from byte-identical source with the same flags, whichever
//! one the linker keeps behaves identically to the other. The original crash was from two
//! *different* SQLite versions/forks colliding, not from duplicate-but-identical ones; this
//! sidesteps that distinction rather than fighting it.
//!
//! Flags copied verbatim from `libsql-ffi` 0.9.30's own `build.rs` (`build_bundled`, the plain
//! non-sqlcipher/non-sqlean path) — always on, not feature-conditional like the real crate's,
//! since this shim has no need to build more than one configuration.

fn main() {
    println!("cargo:rerun-if-changed=bundled/sqlite3.c");
    println!("cargo:rerun-if-changed=bundled/sqlite3.h");

    cc::Build::new()
        .file("bundled/sqlite3.c")
        .flag_if_supported("-std=c11")
        .define("SQLITE_CORE", None)
        .define("SQLITE_DEFAULT_FOREIGN_KEYS", "1")
        .define("SQLITE_ENABLE_API_ARMOR", None)
        .define("SQLITE_ENABLE_COLUMN_METADATA", None)
        .define("SQLITE_ENABLE_DBSTAT_VTAB", None)
        .define("SQLITE_ENABLE_FTS3", None)
        .define("SQLITE_ENABLE_FTS3_PARENTHESIS", None)
        .define("SQLITE_ENABLE_FTS5", None)
        .define("SQLITE_ENABLE_JSON1", None)
        .define("SQLITE_ENABLE_LOAD_EXTENSION", "1")
        .define("SQLITE_ENABLE_MEMORY_MANAGEMENT", None)
        .define("SQLITE_ENABLE_RTREE", None)
        .define("SQLITE_ENABLE_STAT2", None)
        .define("SQLITE_ENABLE_STAT4", None)
        .define("SQLITE_SOUNDEX", None)
        .define("SQLITE_THREADSAFE", "1")
        .define("SQLITE_USE_URI", None)
        .define("HAVE_USLEEP", "1")
        .define("SQLITE_ENABLE_UNLOCK_NOTIFY", None)
        .define("SQLITE_ENABLE_PREUPDATE_HOOK", None)
        .define("SQLITE_ENABLE_SESSION", None)
        .warnings(false)
        .compile("sqlite3_shim");
}
