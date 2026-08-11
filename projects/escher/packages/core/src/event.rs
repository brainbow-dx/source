use core::any::Any;
use core::any::TypeId;
use core::fmt::Debug;
use core::marker::PhantomData;
use core::alloc::Allocator;

use alloc::alloc::Global;

use derive_more::Deref;
use derive_more::DerefMut;

use hashbrown::HashMap;
use hashbrown::DefaultHashBuilder;

pub use ui_events::*;

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
pub struct EventStack<'ctx, A: Allocator> {
    #[deref]
    #[deref_mut]
    handlers: HashMap<TypeId, Vec<Box<dyn EventHandler<'ctx> + 'ctx, A>, A>, DefaultHashBuilder, A>,
}

impl<'ctx> EventStack<'ctx, Global> {
    pub fn new() -> Self {
        EventStack {
            handlers: HashMap::new(),
        }
    }
}

impl<'ctx, A: Allocator + Clone> EventStack<'ctx, A> {
    pub fn new_in(arena: A) -> Self {
        EventStack {
            handlers: HashMap::new_in(arena.clone()),
        }
    }
}

impl<'ctx, A: Allocator + Clone> EventStack<'ctx, A> {
    pub fn push<E, F>(&mut self, closure: F)
    where
        E: Any,
        F: Fn(&E) + 'ctx,
    {
        let arena = self.arena();
        self.handlers.entry(TypeId::of::<E>())
            .or_insert_with(|| Vec::new_in(arena.clone()))
            .push(Box::new_in(EventHandlerRef::new(closure), arena.clone()));
    }
    
    pub fn exec<E: Any>(&self, event: &E) {
        if let Some(handlers) = self.handlers.get(&TypeId::of::<E>()) {
            for handler in handlers.iter() {
                handler.call(event);
            }
        }
    }
    
    pub fn arena(&self) -> A {
        self.handlers.allocator().clone()
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
            .field("handler", &"TODO")
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
