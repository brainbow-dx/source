use crate::element::Element;

use alloc::string::String;

#[derive(Element, Default, Clone, Hash, Debug)]
pub struct Container;

pub type Div = Container;
pub type Section = Container;
pub type Main = Container;
pub type Header = Container;
pub type Footer = Container;
pub type Sidebar = Container;

impl Container {
    pub fn with_alt<S: Into<String>>(self, _value: S) -> Self {
        self // etc.
    }
}
