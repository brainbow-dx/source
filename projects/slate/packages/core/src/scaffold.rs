#![allow(unused)]

use alloc::alloc::Global;
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
use core::alloc::Allocator;

use derive_more::*;

use hashbrown::HashMap;
use hashbrown::DefaultHashBuilder;

// TODO: Remove the atlas dependency in favor of a specialized
// implementation for collections of Slot types.
use atlas::collections::OrderedMap;

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

#[derive(Debug)]
pub struct Scaffold<'ctx> {
    enabled: bool,
    debug: bool,
    content: Option<BString<'ctx>>,
    element: Option<&'ctx dyn Any>,
    styles: StyleSheet<&'ctx Bump>,
    events: EventStack<'ctx, &'ctx Bump>,
    state: HashMap<(TypeId, usize), &'ctx dyn Any, DefaultHashBuilder, &'ctx Bump>,
    slots: OrderedMap<(TypeId, usize), Scaffold<'ctx>, DefaultHashBuilder, &'ctx Bump>,
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
            slots: OrderedMap::new_in(arena),
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
    
    pub fn with_debug(mut self, xray: bool) -> Self {
        self.debug = xray;
        self // etc..
    }
    
    pub fn with_element<E: Element + Any + 'ctx>(mut self, element: E) -> Self {
        // TODO: Wait to draw until we have all slots ..
        // let draw_fn = self.context.arena().alloc(draw_fn);
        let draw_fn = element.draw(self.context);

        self = self.with_slot::<E>(draw_fn);
        self.element = Some(self.alloc(element));
        self
    }

    pub fn with_content<C: AsRef<str> + 'ctx>(mut self, content: Option<C>) -> Self {
        let arena = self.context.arena();
        self.content = content.map(|content| BString::from_str_in(content.as_ref(), arena));
        self // etc..
    }
    // TODO: Move the "Any" requirement to a method on a "State" type.
    pub fn with_state<K: Hash + Any, V: Any>(mut self, key: K, value: V) -> Self {
        let mut hasher = RandomState::default().build_hasher();
        let hash = key.hash(&mut hasher);
        self.state.insert((TypeId::of::<K>(), hasher.finish() as usize), self.alloc(value));
        self // etc..
    }
    
    pub fn with_style<S: Into<Property> + Any>(mut self, style: S) -> Self {
        self.styles.insert(style);
        self // etc..
    }

    pub fn with_handler<E: Any>(mut self, handler: impl Fn(&E) + 'static) -> Self {
        // TODO: Do this inside EventStack ..
        self.events.push(handler);
        self // etc..
    }

    pub fn with_slot<S: Any + 'ctx>(mut self, draw_fn: impl FnOnce(Scaffold<'ctx>) -> Scaffold<'ctx>) -> Self {
        let next_idx = self.slots.len();
        let slot = Scaffold::new_in(self.context.arena());
        self.slots.insert((TypeId::of::<S>(), next_idx), draw_fn(slot));
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
    
    pub fn get_styles(&self) -> &StyleSheet<&Bump> {
        &self.styles
    }
    
    pub fn get_handlers(&self) -> &EventStack<'ctx, &Bump> {
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
