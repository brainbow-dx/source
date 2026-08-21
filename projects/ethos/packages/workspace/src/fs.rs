//! Abstracts reading a workspace's directory structure so `Workspace::scan` works identically
//! whether the caller has real filesystem access (a native process) or none of its own (a
//! browser web worker handed files some other way) — the "embed anywhere" requirement `Workspace`
//! exists for in the first place. A trait, not a concrete `Path`-based API, so a non-native
//! embedder can supply its own backing (an in-memory map, a `FileSystemDirectoryHandle` bridge,
//! whatever it actually has) without this crate needing to know about it.

/// One entry from listing a directory — just enough for `Workspace::scan` to tell a
/// subdirectory apart from a file and recurse or not. Deliberately not `std::fs::DirEntry`
/// (whose own methods assume a real filesystem underneath) or a full `Path` (this crate never
/// needs to do path manipulation beyond joining plain string segments, and a non-native embedder
/// may not have `std::path::Path` semantics to offer at all).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
}

/// What `Workspace::scan` reads a workspace through. Paths are plain, forward-slash-joined
/// strings relative to whatever root the implementor considers `""` — not `std::path::Path`,
/// again so a non-native embedder isn't forced to model OS path semantics it may not have.
pub trait WorkspaceFs {
    /// Every direct child of `path` — `Vec::new()` (not an error) for a path that doesn't exist
    /// or isn't a directory, matching `Workspace::scan`'s own "missing means nothing to find
    /// here," not "fail the whole scan," treatment of an absent `projects/` directory.
    fn read_dir(&self, path: &str) -> Vec<DirEntry>;

    /// Whether `path` exists and is a regular file — all `ProjectKind::detect` needs to check
    /// for a marker file's presence; nothing here ever needs a marker file's actual contents.
    fn is_file(&self, path: &str) -> bool;
}

/// A real, `std::fs`-backed `WorkspaceFs` for a native embedder (Anvil's CLI, an LSP) that
/// actually has a filesystem to read — gated behind the `native-fs` feature so a wasm build of
/// this crate never pulls in code that assumes one exists. `root` is joined with each relative
/// path `WorkspaceFs`'s own methods receive; callers construct one per real filesystem root they
/// want to scan.
#[cfg(feature = "native-fs")]
pub struct NativeFs {
    root: std::path::PathBuf,
}

#[cfg(feature = "native-fs")]
impl NativeFs {
    pub fn new(root: impl Into<std::path::PathBuf>) -> Self {
        NativeFs { root: root.into() }
    }
}

#[cfg(feature = "native-fs")]
impl WorkspaceFs for NativeFs {
    fn read_dir(&self, path: &str) -> Vec<DirEntry> {
        let Ok(entries) = std::fs::read_dir(self.root.join(path)) else {
            return Vec::new();
        };

        entries
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                let name = entry.file_name().to_str()?.to_owned();
                let is_dir = entry.file_type().ok()?.is_dir();
                Some(DirEntry { name, is_dir })
            })
            .collect()
    }

    fn is_file(&self, path: &str) -> bool {
        self.root.join(path).is_file()
    }
}
