//! One project within a `Workspace` — e.g. `projects/escher`, `projects/atlas` in this monorepo.

/// Which toolchain marker files a project directory has — a project can genuinely have more
/// than one (Escher's own `runtimes/web` is both a Rust crate and a Deno-scripted dev server),
/// so this is a set of flags, not a single enum variant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectKind {
    pub rust: bool,
    pub deno: bool,
    pub node: bool,
}

impl ProjectKind {
    /// Detects `kind` from marker files directly inside `path` — deliberately shallow (this
    /// doesn't look inside `Cargo.toml`/`package.json`, or recurse into subdirectories); see
    /// `Workspace`'s own doc comment for why day one stays this narrow.
    pub(crate) fn detect(fs: &dyn crate::WorkspaceFs, path: &str) -> Self {
        ProjectKind {
            rust: fs.is_file(&join(path, "Cargo.toml")),
            deno: fs.is_file(&join(path, "deno.json")) || fs.is_file(&join(path, "deno.jsonc")),
            node: fs.is_file(&join(path, "package.json")),
        }
    }

    /// Whether none of the known markers were found — `Workspace::scan` still records a project
    /// like this (a directory under `projects/` is a project by convention, regardless of
    /// toolchain), but callers wanting to filter out non-code directories can check this.
    pub fn is_unrecognized(&self) -> bool {
        !self.rust && !self.deno && !self.node
    }
}

fn join(path: &str, file_name: &str) -> String {
    format!("{path}/{file_name}")
}

/// One project directory within a `Workspace` — `name` is its directory name (`"escher"`,
/// `"atlas"`), `path` is that directory's full path relative to the same root the owning
/// `Workspace` was scanned from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub name: String,
    pub path: String,
    pub kind: ProjectKind,
}
