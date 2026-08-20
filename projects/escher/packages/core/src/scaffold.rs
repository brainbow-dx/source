#![allow(unused)]

use eyre::Error;
use foldhash::fast::RandomState;

use core::any::Any;
use core::any::TypeId;
use core::fmt::Debug;
use core::hash::Hash;
use core::hash::Hasher;
use core::hash::BuildHasher;
// TODO: Replace with foldhash (or ahash?) ..
use core::marker::PhantomData;

use derive_more::*;

use hashbrown::HashMap;
use hashbrown::DefaultHashBuilder;

// TODO: Remove the atlas-core dependency in favor of a specialized
// implementation for collections of Slot types.
use atlas_core::collections::OrderedMap;

use crate::event::EventStack;
use crate::style::Style;
use crate::style::StyleSheet;
use crate::style::Property;
use crate::event::Event;
use crate::content::Content;
use crate::element::Element;
use crate::draw::DrawContext;
use crate::draw::Bump;
use crate::draw::boxed::Box as BBox;
use crate::draw::collections::String as BString;
use crate::draw::collections::Vec as BVec;

pub mod prelude {
    pub use super::Scaffold;
    pub use super::ScaffoldError;
    pub use super::NodePath;
    pub use super::Overlay;
}

/// A stable path to one node in a `Scaffold` tree, as the sequence of `(TypeId, usize)` slot
/// keys from the root — the same keys `slot::<S>()` already assigns (marker type + insertion
/// index). Exists so a surface with genuinely long-lived native objects (an `NSButton`, an ECS
/// `Entity`) can identify "the same logical node" across separate draw calls, each of which builds
/// an entirely new, differently-arena-allocated `Scaffold` tree — a plain `&Scaffold` reference
/// can't survive past the draw call that produced it, but this can, since it holds no borrow into
/// the arena at all (`TypeId` is `'static`, `usize` is `Copy`).
///
/// Only as stable as the caller's own composition: conditionally *omitting* a `slot` call
/// between draws shifts every later sibling's index and silently breaks identity for them (the
/// classic missing-key bug any keyed-reconciliation scheme has) — prefer `Scaffold::condition
/// (false)` to keep a node present-but-disabled instead of skipping its `slot` call.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct NodePath(Vec<(TypeId, usize)>);

impl NodePath {
    pub fn root() -> Self {
        NodePath(Vec::new())
    }

    pub fn child(&self, key: (TypeId, usize)) -> Self {
        let mut path = self.0.clone();
        path.push(key);
        NodePath(path)
    }
}

/// Reserved marker for `Scaffold::get_overlay()`'s detached child, for use in `NodePath`-based
/// identity schemes — the overlay isn't stored in `slots` (see the `overlay` field), so it has no
/// real slot key of its own; this is a stable stand-in every surface can key it the same way with,
/// via `NodePath::root().child((TypeId::of::<Overlay>(), 0))`.
pub struct Overlay;

#[derive(Debug)]
pub struct Scaffold<'ctx> {
    enabled: bool,
    debug: bool,
    content: Option<BString<'ctx>>,
    element: Option<&'ctx dyn Any>,
    styles: StyleSheet<'ctx>,
    events: EventStack<'ctx>,
    state: HashMap<(TypeId, usize), &'ctx dyn Any, DefaultHashBuilder, &'ctx Bump>,
    slots: OrderedMap<(TypeId, usize), Scaffold<'ctx>, DefaultHashBuilder, &'ctx Bump>,
    // A single detached child, rendered by the runtime outside the normal slot layout flow —
    // e.g. positioned at a fixed corner and layered on top, instead of partitioning space
    // with its siblings the way `slots` does.
    overlay: Option<&'ctx Scaffold<'ctx>>,
    context: DrawContext<'ctx>,
}

impl<'ctx> Scaffold<'ctx> {
    pub fn new_in(arena: &'ctx Bump) -> Self {
        Scaffold {
            enabled: true,
            debug: false,
            element: None,
            content: None,
            state: HashMap::new_in(arena),
            styles: StyleSheet::new_in(arena),
            events: EventStack::new_in(arena),
            slots: OrderedMap::<_, _, DefaultHashBuilder, _>::new_in(arena),
            overlay: None,
            context: DrawContext::new_in(arena),
        }
    }

    pub fn alloc<V>(&self, value: V) -> &'ctx V {
        self.context.arena().alloc(value)
    }
}

impl<'ctx> Scaffold<'ctx> {
    pub fn condition(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self // etc..
    }
    
    pub fn debug(mut self, xray: bool) -> Self {
        self.debug = xray;
        self // etc..
    }

    pub fn element<E: Element + Any + 'ctx>(mut self, element: E) -> Self {
        // TODO: Wait to draw until we have all slots ..
        // let draw_fn = self.context.arena().alloc(draw_fn);
        let draw_fn = element.draw(self.context);

        self = self.slot::<E>(draw_fn);
        self.element = Some(self.alloc(element));
        self
    }

    pub fn content<C: AsRef<str> + 'ctx>(mut self, content: Option<C>) -> Self {
        let arena = self.context.arena();
        self.content = content.map(|content| BString::from_str_in(content.as_ref(), arena));
        self // etc..
    }
    // TODO: Move the "Any" requirement to a method on a "State" type.
    pub fn state<K: Hash + Any, V: Any>(mut self, key: K, value: V) -> Self {
        let mut hasher = RandomState::default().build_hasher();
        let hash = key.hash(&mut hasher);
        self.state.insert((TypeId::of::<K>(), hasher.finish() as usize), self.alloc(value));
        self // etc..
    }

    pub fn style<S: Into<Property> + Any>(mut self, style: S) -> Self {
        self.styles.insert(style);
        self // etc..
    }

    pub fn handle<E: Any>(mut self, handler: impl Fn(&E) + 'static) -> Self {
        // TODO: Do this inside EventStack ..
        self.events.push(handler);
        self // etc..
    }

    pub fn slot<S: Any + 'ctx>(mut self, draw_fn: impl FnOnce(Scaffold<'ctx>) -> Scaffold<'ctx>) -> Self {
        // Per-`S` insertion index, not a global count of every slot on this node regardless of
        // type — a slot's key is "the Nth child of marker type `S`," matching what `NodePath`'s
        // own doc comment already promises ("marker type + insertion index"). Using a global
        // counter here instead would mean any slot inserted *after* a variable-length loop of some
        // other type (e.g. a "new tab" button after N tab rows) gets a key that silently shifts
        // every time the loop's length changes, breaking its identity across draws for no reason
        // tied to that slot itself.
        let next_idx = self.slots.iter().filter(|((type_id, _), _)| *type_id == TypeId::of::<S>()).count();
        let slot = Scaffold::new_in(self.context.arena());
        self.slots.insert((TypeId::of::<S>(), next_idx), draw_fn(slot));
        self // etc..
    }

    /// A detached child rendered outside the normal slot layout — see the `overlay` field.
    pub fn overlay(mut self, draw_fn: impl FnOnce(Scaffold<'ctx>) -> Scaffold<'ctx>) -> Self {
        let overlay = Scaffold::new_in(self.context.arena());
        self.overlay = Some(self.alloc(draw_fn(overlay)));
        self // etc..
    }
}

impl<'ctx> Scaffold<'ctx> {
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_debug(&self) -> bool {
        self.debug
    }
}

impl<'ctx> Scaffold<'ctx> {
    pub fn set_debug(&mut self, debug: bool) -> &Self {
        self.debug = debug;
        self // etc..
    }
}

impl<'ctx> Scaffold<'ctx> {
    pub fn get_element<E: Element + Any>(&self) -> Option<&E> {
        self.element.and_then(|element| element.downcast_ref::<E>())
    }
    
    pub fn get_content(&self) -> Option<&BString<'ctx>> {
        self.content.as_ref()
    }
    
    pub fn get_styles(&self) -> &StyleSheet<'ctx> {
        &self.styles
    }

    pub fn get_handlers(&self) -> &EventStack<'ctx> {
        &self.events
    }
    
    pub fn get_slots(&self) -> &OrderedMap<(TypeId, usize), Scaffold<'ctx>, DefaultHashBuilder, &Bump> {
        &self.slots
    }
    
    pub fn get_slot<S: Any>(&self) -> impl IntoIterator<Item = (&usize, &Scaffold<'ctx>)> {
        self.slots.iter().filter_map(|((type_id, i), scaffold)| {
            type_id.eq(&TypeId::of::<S>()).then(|| (i, scaffold))
        })
    }
    
    pub fn get_slot_at<S: Any>(&self, i: usize) -> Option<&Scaffold<'ctx>> {
        self.slots.get(&(TypeId::of::<S>(), i))
    }

    pub fn get_overlay(&self) -> Option<&Scaffold<'ctx>> {
        self.overlay
    }

    /// Looks up a direct child by its raw `(TypeId, usize)` slot key — the same key `NodePath`
    /// accumulates one segment of per level. Plain slot lookup only; use `get_at_path` to resolve
    /// a full multi-level `NodePath` against this tree.
    pub fn get_slot_by_key(&self, key: &(TypeId, usize)) -> Option<&Scaffold<'ctx>> {
        self.slots.get(key)
    }

    /// Resolves a `NodePath` (captured against some *earlier* draw's tree) against *this* tree —
    /// the one legitimate way to turn "the node a stale native callback remembers" back into a
    /// live, current `&Scaffold` with real, callable handlers. Returns `None` if the path no
    /// longer resolves (the node was removed, or an ancestor's slot count changed under it) —
    /// callers should treat that as "silently drop this event," not an error, since it just means
    /// the tree changed shape between when the native callback fired and this draw.
    pub fn get_at_path(&self, path: &NodePath) -> Option<&Scaffold<'ctx>> {
        path.0.iter().try_fold(self, |node, key| {
            if *key == (TypeId::of::<Overlay>(), 0) { node.get_overlay() } else { node.get_slot_by_key(key) }
        })
    }
}

impl<'ctx> Scaffold<'ctx> {
    pub fn dispatch<E: Any>(&self, event: &E) -> Result<(), Error> {
        for (slot_type_id, slot_scaffold) in self.get_slots().iter() {
            slot_scaffold.dispatch::<E>(event);
        }
        
        self.events.exec::<E>(event);
        Ok(())
    }
    
    pub fn build(mut self) -> Self {
        self // etc..
    }
}

#[derive(oops::Error, From)]
pub enum ScaffoldError {
    #[msg("unknown error: {0}")]
    Unknown(eyre::Error),
}
