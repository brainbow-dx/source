//! The seam between "a runtime lets a user edit a `Scaffold` tree visually" and "something turns
//! that into a real change" — one trait (`EditBackend`), with (at least) two implementations
//! expected over time: [`InMemoryEditBackend`] here, which only mutates a runtime-side tree (what
//! an editor-UI track builds and demos against first, with nowhere for edits to actually
//! persist), and a later Ethos-backed implementation that turns the same calls into precise
//! source-code updates on disk (see `spec/.agents/proposals/scaffold-edit-mode.md`).
//!
//! Exists as its own crate, not folded into `escher-core` or wherever the first UI consumer
//! lives, specifically so it can be the shared contract between two independently-developed
//! tracks: an editor-UI build-out that only needs *a* working `EditBackend` to build against, and
//! an Ethos-codegen build-out that only needs to satisfy this same trait once it's ready — neither
//! side blocks on the other, and neither has to guess at the other's shape. Swapping
//! `InMemoryEditBackend` for the real one should be a one-line change at whatever call site
//! constructs it, not a rewrite of the UI code that calls it.
//!
//! Deliberately minimal: no undo/redo, no batching/transactions, no conflict resolution for
//! concurrent edits. Add those once a real consumer needs them, not speculatively.

use std::collections::HashMap;
use std::fmt;

use escher_core::scaffold::NodePath;
use escher_core::style::Property;

/// A structural change an `EditBackend` can apply. Selection/hit-testing (turning a click into a
/// `NodePath`) is a runtime-UI concern, not this trait's — everything here starts from a
/// `NodePath` the caller already resolved.
pub trait EditBackend {
    type Error: std::error::Error;

    /// Inserts `node` as a new child of `parent`, at `index` among its existing children.
    /// Returns the new node's own `NodePath`.
    fn insert(&mut self, parent: NodePath, index: usize, node: NodeEdit) -> Result<NodePath, Self::Error>;

    /// Removes the node at `path` (and everything under it).
    fn delete(&mut self, path: NodePath) -> Result<(), Self::Error>;

    /// Moves the node at `path` to be a child of `new_parent`, at `index` among its children.
    fn move_node(&mut self, path: NodePath, new_parent: NodePath, index: usize) -> Result<(), Self::Error>;

    /// Sets (or overwrites) one style property on the node at `path`.
    fn set_style(&mut self, path: NodePath, property: Property) -> Result<(), Self::Error>;

    /// Sets (or clears, via `None`) the node at `path`'s text content.
    fn set_content(&mut self, path: NodePath, content: Option<String>) -> Result<(), Self::Error>;

    /// Flushes any in-flight/uncommitted edits — a real Ethos-backed implementation is expected
    /// to buffer edits and apply them as one precise source-code patch here, rather than
    /// rewriting a file on every single call above; `InMemoryEditBackend` applies immediately, so
    /// this is a no-op for it.
    fn commit(&mut self) -> Result<(), Self::Error>;
}

/// Minimal description of a node to insert — just enough for `InMemoryEditBackend` and an
/// eventual Ethos backend to both have something concrete to act on. Deliberately not the full
/// `Scaffold`/element type system (that's arena-bound, `'ctx`-tied, and not meant to cross this
/// boundary) — extend with whatever a real editor UI turns out to need (an element *kind* name at
/// minimum; more fields as insertion actually gets built).
#[derive(Debug, Clone, Default)]
pub struct NodeEdit {
    pub element: String,
    pub content: Option<String>,
}

/// A pure in-memory `EditBackend` — no source-code writes, no persistence at all. Good enough to
/// build and demo an editor UI's insert/select/delete/restyle interactions against before Ethos's
/// codegen pipeline exists; not good enough to ship, since nothing here ever reaches disk.
#[derive(Debug, Default)]
pub struct InMemoryEditBackend {
    children: HashMap<NodePath, Vec<NodePath>>,
    styles: HashMap<NodePath, Vec<Property>>,
    content: HashMap<NodePath, Option<String>>,
    next_index: HashMap<NodePath, usize>,
}

#[derive(Debug)]
pub enum InMemoryEditError {
    /// `delete`/`move_node`/`set_style`/`set_content` referenced a `NodePath` this backend never
    /// saw an `insert` for.
    UnknownNode(NodePath),
}

impl fmt::Display for InMemoryEditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InMemoryEditError::UnknownNode(path) => write!(f, "unknown node: {path:?}"),
        }
    }
}

impl std::error::Error for InMemoryEditError {}

impl EditBackend for InMemoryEditBackend {
    type Error = InMemoryEditError;

    fn insert(&mut self, parent: NodePath, _index: usize, node: NodeEdit) -> Result<NodePath, Self::Error> {
        let slot_index = self.next_index.entry(parent.clone()).or_insert(0);
        let type_id = std::any::TypeId::of::<NodeEdit>();
        let child_path = parent.child((type_id, *slot_index));
        *slot_index += 1;

        self.children.entry(parent).or_default().push(child_path.clone());
        self.content.insert(child_path.clone(), node.content);

        Ok(child_path)
    }

    fn delete(&mut self, path: NodePath) -> Result<(), Self::Error> {
        for children in self.children.values_mut() {
            children.retain(|child| child != &path);
        }
        self.children.remove(&path);
        self.styles.remove(&path);
        self.content.remove(&path);
        Ok(())
    }

    fn move_node(&mut self, path: NodePath, new_parent: NodePath, _index: usize) -> Result<(), Self::Error> {
        for children in self.children.values_mut() {
            children.retain(|child| child != &path);
        }
        self.children.entry(new_parent).or_default().push(path);
        Ok(())
    }

    fn set_style(&mut self, path: NodePath, property: Property) -> Result<(), Self::Error> {
        self.styles.entry(path).or_default().push(property);
        Ok(())
    }

    fn set_content(&mut self, path: NodePath, content: Option<String>) -> Result<(), Self::Error> {
        self.content.insert(path, content);
        Ok(())
    }

    fn commit(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_then_delete_round_trips() {
        let mut backend = InMemoryEditBackend::default();
        let root = NodePath::root();

        let child = backend.insert(root.clone(), 0, NodeEdit { element: "Label".into(), content: Some("hi".into()) }).unwrap();
        assert_eq!(backend.children.get(&root).map(Vec::len), Some(1));

        backend.set_style(child.clone(), Property::Padding(Default::default())).unwrap();
        assert_eq!(backend.styles.get(&child).map(Vec::len), Some(1));

        backend.delete(child.clone()).unwrap();
        assert_eq!(backend.children.get(&root).map(Vec::len), Some(0));
        assert!(!backend.styles.contains_key(&child));
    }
}
