//! Reusable UI components, shared across every app and every runtime — the counterpart to a
//! project's own styleguide (a per-app/per-project markdown+YAML document describing the values a
//! composition should draw on, eventually consumed here and by runtimes/compilers to apply them;
//! not implemented yet, this crate is the component half of that pair).
//!
//! What belongs here: a composition built purely from `escher_core` primitives (`Scaffold`,
//! `Element`, `Property`, core `event`s) with **no dependency on any specific runtime or surface
//! crate** (`escher-appkit`, `escher-bevy`, `escher-terminal`, `escher-web`, ...) — anything that
//! renders correctly on whichever surface eventually draws it, the same way `escher_core::element
//! ::Button` does. `toolbar::toolbar` is the first one: back/forward/refresh buttons plus an
//! address field, built entirely out of `Button`/`Input`, portable as-is.
//!
//! What doesn't belong here yet: a composition that leans on a *specific* surface's own rendering
//! optimizations to work at all. `escher_appkit::tabs::tab_strip` is the concrete example — its
//! favicon display and drag-to-reorder interaction are expressed through `escher_appkit::surface`'s
//! own marker elements (`FaviconImage`, `TabRowMarker`), which only `AppKitSurface` knows how to
//! turn into a real native view; a Bevy or Terminal surface has no idea what to do with them. It
//! stays in `escher-appkit` until `escher-core` grows the generic primitives (an image element with
//! a real content source, a generic drag gesture) that would let it be re-expressed without
//! reaching for anything AppKit-specific — moving it here *unchanged* would just be lying about
//! portability, not providing it.

pub mod toolbar;
pub mod tabs;
