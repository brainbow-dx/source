use core::alloc::Layout;
// use core::any::TypeId;
// use core::borrow::Borrow;
use core::hash::Hash;
use core::hash::Hasher;

use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::context::Bump;
use crate::element::Element;
use crate::element::ElementError;
use crate::element::ElementHasher;
use crate::element::ElementNode;
use crate::element::UUID;
use crate::element::content::Content;
use crate::style::Style;
use crate::style::StyleSheet;

//---
/// A lightweight single-pass builder for a tree of Element nodes.
///
/// Represents the intended state of a `Surface` for a.
///
/// ## Guide: Direct Scaffolding
///
/// In some cases you might want to build a scaffold directly (e.g., at runtime
/// on an embedded system, or in cases where reactivity isn't a primary goal).
///
/// In these cases you're encouraged to build a `Scaffold` directly and use it
/// to render the `Surface` in whatever way you need.
///
/// ```rust
/// let mut scaffold = Scaffold::new();
/// // TODO: etc..
/// ```
#[derive(Debug)]
pub struct Scaffold<'arena> {
    /// The node currently being built.
    element: Option<(&'arena dyn Element, Layout)>,

    /// The built stylesheet for this node.
    stylesheet: StyleSheet<'arena>,

    /// TODO
    slots: Vec<(), &'arena Bump>,

    /// TODO
    children: Vec<Scaffold<'arena>, &'arena Bump>,

    /// Hashing for the Scaffold's internal state. Includes the internal
    /// Element, ~~styles, events, etc~~.
    ///
    /// ### TODO:
    ///  - Update hasher for styles, events, etc.
    hasher: ElementHasher,

    /// TODO
    arena: &'arena Bump,
}

impl<'arena> Scaffold<'arena> {
    /// Create a new Scaffold from a given Element.
    pub fn new_in(arena: &'arena Bump) -> Self {
        Scaffold {
            element: None,
            stylesheet: StyleSheet::new_in(arena),
            slots: Vec::new_in(arena),
            children: Vec::new_in(arena),
            hasher: ElementHasher::default(),
            arena,
        }
    }

    /// TODO
    pub fn with_element<E: Element + Hash + 'arena>(mut self, element: E) -> Self {
        let element = self.arena.alloc(element);
        element.hash(&mut self.hasher);
        self.element = Some((element, Layout::new::<E>()));
        self // etc..
    }

    pub fn with_slot(mut self) -> Self {
        self.slots.push(());
        self // etc..
    }

    /// TODO
    pub fn is_empty(&self) -> bool {
        self.element.is_none() && self.children.is_empty()
    }
}

impl<'arena> Scaffold<'arena> {
    /// Provides immutable access to the element node of this Scaffold.
    pub fn get_element(&self) -> Option<&dyn Element> {
        self.element
            .map(|element| element.0)
    }

    /// TODO: See if we can do this without unsafe (maybe with `dyn_clone`?).
    pub fn take_element_boxed(&mut self) -> Option<Box<dyn Element>> {
        self.element
            .take()
            .and_then(|(element, layout)| unsafe {
                type DynPtr = (*mut u8, *const ());
                let (src_data_ptr, vtable_ptr): DynPtr = core::mem::transmute(element);

                let box_data_ptr = crate::context::alloc(layout) as *mut u8;
                core::ptr::copy_nonoverlapping(src_data_ptr, box_data_ptr, layout.size());

                let element_ptr: *mut dyn Element = core::mem::transmute((box_data_ptr, vtable_ptr));
                let element_box = Box::from_raw(element_ptr);

                Some(element_box)
            })
    }

    /// TODO
    pub fn stylesheet(&self) -> &StyleSheet<'arena> {
        &self.stylesheet
    }

    /// TODO
    pub fn stylesheet_mut(&mut self) -> &mut StyleSheet<'arena> {
        &mut self.stylesheet
    }

    /// TODO
    pub fn content(&self) -> Option<Content<'_>> {
        self.get_element()
            .as_ref()
            .and_then(|e| e.content())
    }

    //--
    /// Provides immutable access to the children of this Scaffold.
    pub fn children(&self) -> &Vec<Scaffold<'arena>, &'arena Bump> {
        self.children.as_ref()
    }

    /// Provides mutable access to the children of this Scaffold.
    pub fn children_mut(&mut self) -> &mut Vec<Scaffold<'arena>, &'arena Bump> {
        self.children.as_mut()
    }
}

impl<'arena> Scaffold<'arena> {
    /// TODO
    pub fn add<E>(&mut self, element: E) -> Result<&mut Self, ScaffoldError>
    where
        E: Element + Hash + 'arena,
    {
        let child_idx = self.children.len() + 1;
        let child = Scaffold::new_in(self.arena).with_element(element);

        self.children.push(child);
        self.children
            .last_mut()
            .ok_or(ScaffoldError::IndexOutOfBounds(child_idx))
    }

    /// TODO
    pub fn with_style_attr<V: Style + 'static>(&mut self, style_value: V) -> Result<&mut Self, ScaffoldError> {
        self.stylesheet
            .push(style_value);
        Ok(self) // etc..
    }

    /// TODO
    pub fn with_class_attr<F: Fn(&mut StyleSheet<'arena>)>(&mut self, class_fn: F) -> Result<&mut Self, ScaffoldError> {
        class_fn(&mut self.stylesheet);
        Ok(self) // etc..
    }

    /// TODO
    pub fn with_children<F>(&mut self, child_builder_fn: F) -> Result<&mut Self, ScaffoldError>
    where
        F: FnOnce(&mut Scaffold) -> Result<(), ScaffoldError>,
    {
        child_builder_fn(self)?;
        Ok(self)
    }

    /// TODO
    pub fn build(&mut self) -> Result<&mut Self, ScaffoldError> {
        if let Some(ref element) = self.get_element() {
            element.draw()(self)?;
        }

        #[cfg(feature = "verbose")]
        tracing::debug!("Built Scaffold with Hash({:?})", self.hash);

        Ok(self)
    }
}

impl<'arena> Scaffold<'arena> {
    /// TODO
    pub fn hash(&self) -> u64 {
        self.hasher.finish()
    }

    /// TODO
    pub fn has_changes(&self, element_node: &ElementNode) -> bool {
        self.hash() != element_node.hash()
    }
}

impl<'arena> Scaffold<'arena> {
    /// TODO
    pub fn try_from_draw_fn<F>(arena: &'arena Bump, draw_fn: F) -> Result<Self, ScaffoldError>
    where
        F: FnOnce(&mut Scaffold<'arena>) -> Result<(), ScaffoldError>,
    {
        let mut scaffold = Scaffold::new_in(arena);
        draw_fn(&mut scaffold)?;
        Ok(scaffold)
    }
}

/// TODO
#[derive(oops::Error)]
#[derive(derive_more::From)]
pub enum ScaffoldError {
    /// An item wasn't at the expected index. This is almost always a logical
    /// error and is likely a bug in the Scaffold behavior iteself.
    #[msg("index '{0}' out of bounds")]
    IndexOutOfBounds(usize),

    /// The hash was expected but is not available. This is almost always a
    /// programming error. It means you're trying to use a hash that hasn't
    /// been built yet (probably during some phase of change detection).
    #[msg("hash missing")]
    HashMissing,

    /// TODO
    #[msg("node '{0}' missing")]
    NodeMissing(UUID),

    /// TODO
    #[msg("element error: {0}")]
    ElementError(#[from] ElementError),

    /// TODO
    #[msg("unknown error: {0}")]
    Unknown(&'static str),
}
