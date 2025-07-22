use core::alloc::AllocError;
use core::alloc::Allocator;
use core::alloc::Layout;
use core::ptr::NonNull;

pub use alloc::alloc::alloc;

use once_cell::sync::OnceCell;

pub use bumpalo::Bump;
pub use bumpalo::boxed::*;
pub use bumpalo::collections::*;

pub use bumpalo_herd::Herd;
pub use bumpalo_herd::Member;

//---
static HERD: OnceCell<Herd> = OnceCell::new();

pub fn get_arena<'ctx>() -> Member<'ctx> {
    HERD.get_or_init(|| Herd::new()).get()
}

#[derive(Debug, Clone, Copy)]
pub struct Context<'ctx> {
    arena: &'ctx Bump,
}

impl<'ctx> Context<'ctx> {
    pub fn new_in(arena: &'ctx Bump) -> Self {
        Context { arena }
    }

    pub fn arena(&self) -> &Bump {
        self.arena
    }
}

impl<'ctx> Context<'ctx> {
    pub fn alloc<T>(&self, value: T) -> &mut T {
        self.arena.alloc(value)
    }
}

unsafe impl Allocator for Context<'_> {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
        self.arena.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
        unsafe { self.arena.deallocate(ptr, layout) }
    }
}
