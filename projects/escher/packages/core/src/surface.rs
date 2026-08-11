#![allow(unused)]

use core::fmt::Debug;

use color_eyre::Result;

use crate::draw::DrawUpdate;
use crate::scaffold::Scaffold;

//---
pub trait Surface {
    type Event;
    
    fn draw<F>(&mut self, draw_fn: F) -> Result<Self::Event>
    where
        F: for<'a> FnOnce(Scaffold<'a>) -> Scaffold<'a>;
}

pub struct SurfaceUpdate {
    //..
}

impl DrawUpdate for SurfaceUpdate {
    //..
}
