mod style;
pub use style::*;

pub mod primitive;

pub mod property;

pub mod prelude {
    pub use crate::style::primitive::*;
    pub use crate::style::property::*;
    pub use crate::style::style::*;
}
