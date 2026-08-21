use core::any::Any;
use core::any::TypeId;
use core::fmt::Debug;
use core::marker::PhantomData;

use bumpalo::Bump;
use bumpalo::collections::Vec as BVec;

use derive_more::Deref;
use derive_more::DerefMut;

use hashbrown::HashMap;
use hashbrown::DefaultHashBuilder;

pub use ui_events::*;

pub mod prelude {
    pub use super::Event;
    pub use super::EventHandler;
    pub use super::EventStack;
    pub use super::EventHandlerRef;
    pub use super::ClickEvent;
    pub use super::SubmitEvent;
}

/// A generic "this control was activated" signal — deliberately zero-field. Not
/// `ui_events::pointer::PointerButtonEvent` (built for continuous, sub-pixel pointer devices —
/// pressure, tilt, contact geometry — none of which a native button's target-action model, or a
/// terminal-cell click, can meaningfully supply). Surfaces with real spatial/pointer semantics
/// (`runtimes/terminal`'s own `ClickEvent`, column/row/button/modifiers) keep their own richer,
/// surface-local type for hit-testing — this one is for the common case of "a `Button`-carrying
/// node was activated," dispatchable uniformly by any surface regardless of what triggered it.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClickEvent;

/// Fired when a node's value is committed — Return on a native text field, or an equivalent from
/// any other surface — carrying the committed text. Distinct from `keyboard::KeyboardEvent`,
/// which only ever describes a single keypress, never the accumulated/committed value.
#[derive(Debug, Clone)]
pub struct SubmitEvent(pub String);

#[derive(Deref, DerefMut)]
pub struct Event<'event, E> {
    pub propagation_stopped: bool,
    
    #[deref]
    #[deref_mut]
    event: &'event E,
}

pub trait EventHandler<'ctx>: Debug {
    fn call(&self, event: &dyn Any);
}

#[derive(Debug, Deref, DerefMut)]
pub struct EventStack<'ctx> {
    #[deref]
    #[deref_mut]
    handlers: HashMap<TypeId, BVec<'ctx, &'ctx dyn EventHandler<'ctx>>, DefaultHashBuilder, &'ctx Bump>,
}

impl<'ctx> EventStack<'ctx> {
    pub fn new_in(arena: &'ctx Bump) -> Self {
        EventStack {
            handlers: HashMap::new_in(arena),
        }
    }

    pub fn push<E, F>(&mut self, closure: F)
    where
        E: Any,
        F: Fn(&E) + 'ctx,
    {
        let arena = self.arena();
        let handler: &'ctx dyn EventHandler<'ctx> = arena.alloc(EventHandlerRef::new(closure));
        self.handlers.entry(TypeId::of::<E>())
            .or_insert_with(|| BVec::new_in(arena))
            .push(handler);
    }

    pub fn exec<E: Any>(&self, event: &E) {
        if let Some(handlers) = self.handlers.get(&TypeId::of::<E>()) {
            for handler in handlers.iter() {
                handler.call(event);
            }
        }
    }

    pub fn arena(&self) -> &'ctx Bump {
        *self.handlers.allocator()
    }
}

//---
pub struct EventHandlerRef<E, F> {
    event: PhantomData<E>,
    handler: F,
}

impl<E, F> EventHandlerRef<E, F> {
    pub fn new(handler: F) -> Self {
        EventHandlerRef {
            handler,
            event: PhantomData,
        }
    }
}

impl<'ctx, E, F> Debug for EventHandlerRef<E, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EventHandlerRef")
            .field("event", &self.event)
            .field("handler", &"<fn>")
            .finish()
    }
}

impl<'ctx, E, F> EventHandler<'ctx> for EventHandlerRef<E, F>
where
    E: Any,
    F: Fn(&E) + 'ctx,
{
    fn call(&self, event: &dyn Any) {
        if let Some(event) = event.downcast_ref::<E>() {
            (self.handler)(event);
        }
    }
}
