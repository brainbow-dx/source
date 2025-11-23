pub use bumpalo::*;

pub use bumpalo_herd::*;

//---
#[derive(Debug, Clone, Copy)]
pub struct DrawContext<'ctx> {
    arena: &'ctx Bump,
}

impl<'ctx> DrawContext<'ctx> {
    pub fn new_in(arena: &'ctx Bump) -> Self {
        DrawContext {
            arena,
        }
    }
}

impl<'ctx> DrawContext<'ctx> {
    pub fn arena(&self) -> &'ctx Bump {
        self.arena
    }
}

pub trait DrawUpdate {
    //..
}

pub type DrawReport = ();

// pub enum DrawReport<U: DrawUpdate> {
//     Updates(Vec<U>),
//     NoOp,
// }
