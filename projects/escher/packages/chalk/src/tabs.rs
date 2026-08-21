//! Neutral tab data — no dependency on any surface's own rendering of it. Kept separate from
//! `escher_appkit::tabs::tab_strip` (which *does* stay AppKit-specific, see this crate's own doc
//! comment) so an app can hold and pass around tab state without pulling in a concrete rendering
//! backend just to name the data shape.

/// One open scene, as far as any tab-strip-shaped composition is concerned. `id` is a stable
/// identity that survives reordering (a tab's *position* in whatever list holds it doesn't — see
/// `escher_core::scaffold::NodePath`'s own doc comment on why identity has to be handled this way
/// for anything reorderable).
#[derive(Clone)]
pub struct TabInfo {
    pub id: u64,
    pub title: String,
    pub host: String,
}
