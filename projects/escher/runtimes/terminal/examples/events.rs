#![feature(allocator_api)]

extern crate alloc;

use core::fmt::Display;

use color_eyre::owo_colors::OwoColorize;
use derive_more::Display;

use bumpalo::Bump;

use crate::escher_event::EventStack;

#[derive(Debug, Display)]
#[display("{:}({:})", self.as_role, self.user_id)]
pub struct UserAuthSuccess<R: Display> {
    pub user_id: u32,
    pub as_role: R,
}

#[derive(Debug, Display)]
pub struct Admin;

#[derive(Debug, Display)]
pub struct User;

#[derive(Debug, Display)]
pub struct Guest;

#[derive(Debug, Display)]
#[display("Downloaded {:} bytes to {:}", self.size_bytes, self.file_path)]
pub struct FileDownloadSuccess<P: Display> {
    size_bytes: u64,
    file_path: P,
}

fn main() {
    println!("Setting up event system for a single scope");
    
    let arena = Bump::new();
    
    let mut event_stack = EventStack::new_in(&arena);
    
    event_stack.push(|event: &UserAuthSuccess<Admin>| {
        println!(" --> {0}", event.magenta());
    });
    
    event_stack.push(|event: &UserAuthSuccess<User>| {
        println!(" --> {0}", event.cyan());
    });
    
    event_stack.push(|event: &UserAuthSuccess<Guest>| {
        println!(" --> {0}: Audit", event.red());
    });
    
    event_stack.push(|event: &FileDownloadSuccess<&str>| {
        println!(" --> {0}", event.blue());
    });
    
    //--
    println!("Dispatching UserLoggedIn event");
    
    event_stack.call(&UserAuthSuccess {
        user_id: 1031,
        as_role: Admin,
    });
    
    event_stack.call(&UserAuthSuccess {
        user_id: 4693,
        as_role: User,
    });
    
    event_stack.call(&UserAuthSuccess {
        user_id: 3102,
        as_role: Guest,
    });
    
    //--
    println!("Dispatching FileDownloaded events ..");
    
    event_stack.call(&FileDownloadSuccess {
        file_path: "/data/report.pdf",
        size_bytes: 4096,
    });
    
    event_stack.call(&FileDownloadSuccess {
        file_path: "/data/f2-asdfsdf.pdf",
        size_bytes: 391,
    });
    
    event_stack.call(&FileDownloadSuccess {
        file_path: "/data/final_final_report_v3.3.1.pdf",
        size_bytes: 824040,
    });

    println!("End of scope");
}

pub mod escher_event {
    use core::any::Any;
    use core::any::TypeId;
    use core::marker::PhantomData;
    use core::alloc::Allocator;
    
    use alloc::alloc::Global;
    
    use derive_more::Deref;
    use derive_more::DerefMut;
    use hashbrown::DefaultHashBuilder;
    use hashbrown::HashMap;
    
    #[derive(Deref, DerefMut)]
    pub struct Event<'event, E> {
        pub propagation_stopped: bool,
        
        #[deref]
        #[deref_mut]
        event: &'event E,
    }
    
    pub trait EventHandler<'ctx> {
        fn call(&self, event: &dyn Any);
    }
    
    #[derive(Deref, DerefMut)]
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
        
        pub fn call<E: Any>(&self, event: &E) {
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
}
