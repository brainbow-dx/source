//! Ethos's `Workspace` — a small, embeddable, read-only model of "what Brainbow projects exist
//! and how they relate," per `spec/agents/proposals/workspace-core.md`. One `Workspace` instance
//! per environment (an LSP process, an Anvil CLI invocation, eventually a browser web worker or
//! a Rust core embedded in a Unity game), not a shared daemon — so this crate stays a plain data
//! structure plus a scan function, not a service with its own lifecycle.
//!
//! Day one is deliberately narrow: which projects exist under a workspace root, and what kind of
//! project each looks like (Rust/Deno/Node, by marker file) — not a full file tree, and not yet
//! the "route mutations through here" contract the proposal doc describes as this crate's
//! eventual job. Widen it once a real second consumer (beyond Anvil's CLI) needs more.
//!
//! Kept free of any direct filesystem dependency in the default build — see `fs::WorkspaceFs` —
//! so this compiles for a wasm target with no filesystem of its own; a native embedder opts into
//! `NativeFs` via the `native-fs` feature.

mod fs;
mod project;
mod workspace;

pub use fs::DirEntry;
pub use fs::WorkspaceFs;
#[cfg(feature = "native-fs")]
pub use fs::NativeFs;
pub use project::Project;
pub use project::ProjectKind;
pub use workspace::Workspace;
