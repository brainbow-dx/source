use core::ops::Deref;
// use core::fmt::Display;
use core::fmt::Debug;

// use alloc::vec::Vec;
// use alloc::format;

use alloc::string::String;

use crate::style::Style;
use crate::style::StyleValue;
// use crate::style::primitive::Weight;
use crate::style::primitive::Color;
use crate::style::primitive::Rect;
use crate::style::primitive::Size2d;
use crate::style::primitive::Unit;

/// TODO
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Flex(bool);

#[automatically_derived]
impl Into<StyleValue> for Flex {
    /// TODO
    fn into(self) -> StyleValue {
        StyleValue::Flex(self)
    }
}

#[automatically_derived]
impl Style for Flex {
    //..
}

/// TODO
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub enum FlexDirection {
    /// TODO
    Column,

    /// TODO
    #[default]
    Row,
}

#[automatically_derived]
impl Into<StyleValue> for FlexDirection {
    /// TODO
    fn into(self) -> StyleValue {
        StyleValue::FlexDirection(self)
    }
}

#[automatically_derived]
impl Style for FlexDirection {
    //..
}

//--
/// TODO
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct FlexBasis(Unit<f32>);

#[automatically_derived]
impl FlexBasis {
    pub fn new(value: f32) -> Self {
        FlexBasis(Unit::Px(value))
    }
}

#[automatically_derived]
impl FlexBasis {
    pub fn unit(&self) -> &Unit {
        &self.0
    }
}

#[automatically_derived]
impl Into<Unit> for FlexBasis {
    fn into(self) -> Unit {
        self.0
    }
}

#[automatically_derived]
impl Deref for FlexBasis {
    type Target = Unit;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[automatically_derived]
impl Into<StyleValue> for FlexBasis {
    fn into(self) -> StyleValue {
        StyleValue::FlexBasis(self)
    }
}

#[automatically_derived]
impl Style for FlexBasis {
    //..
}

//--
/// TODO
// #[derive(StyleValue, Debug, PartialEq)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FlexGrow(f32);

#[automatically_derived]
impl FlexGrow {
    /// TODO
    pub fn new<V: Into<f32>>(value: V) -> Self {
        FlexGrow(value.into())
    }
}

#[automatically_derived]
impl Deref for FlexGrow {
    type Target = f32;

    /// TODO
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[automatically_derived]
impl Into<StyleValue> for FlexGrow {
    /// TODO
    fn into(self) -> StyleValue {
        StyleValue::FlexGrow(self)
    }
}

#[automatically_derived]
impl Style for FlexGrow {
    //..
}

/// TODO
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FlexShrink(f32);

#[automatically_derived]
impl FlexShrink {
    /// TODO
    pub fn new<V: Into<f32>>(value: V) -> Self {
        FlexShrink(value.into())
    }
}

#[automatically_derived]
impl Into<StyleValue> for FlexShrink {
    /// TODO
    fn into(self) -> StyleValue {
        StyleValue::FlexShrink(self)
    }
}

impl Deref for FlexShrink {
    type Target = f32;

    /// TODO
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[automatically_derived]
impl Style for FlexShrink {
    //..
}

/// TODO
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum AlignItems {
    Stretch,
    Center,
    // Start,
    // End,
}

#[automatically_derived]
impl Into<StyleValue> for AlignItems {
    /// TODO
    fn into(self) -> StyleValue {
        StyleValue::AlignItems(self)
    }
}

#[automatically_derived]
impl Style for AlignItems {
    //..
}

/// TODO
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum JustifyContent {
    Start,
    Center,
    // SpaceBetween,
    // SpaceAround,
    // SpaceEvenly,
}

#[automatically_derived]
impl Into<StyleValue> for JustifyContent {
    /// TODO
    fn into(self) -> StyleValue {
        StyleValue::JustifyContent(self)
    }
}

#[automatically_derived]
impl Style for JustifyContent {
    //..
}

//--
/// TODO
#[derive(crate::macros::Unit, Default, Clone, Copy, Debug, PartialEq)]
pub struct Gap(Unit<f32>);

/// TODO
#[derive(crate::macros::Color, Default, Copy, Clone, Debug, PartialEq)]
pub struct BackgroundColor(Color);

/// TODO
#[derive(Default, Clone, Debug, PartialEq)]
pub struct FontFamily(String);

impl FontFamily {
    pub fn new(family: &str) -> Self {
        FontFamily(String::from(family))
    }

    pub fn name(&self) -> &str {
        self.0.as_ref()
    }
}

impl Into<StyleValue> for FontFamily {
    /// TODO
    fn into(self) -> StyleValue {
        StyleValue::FontFamily(self)
    }
}

impl Style for FontFamily {
    //..
}

/// TODO
#[derive(crate::macros::Unit, Default, Clone, Copy, Debug, PartialEq)]
pub struct FontSize(Unit<f32>);

/// TODO
#[derive(crate::macros::Color, Default, Copy, Clone, Debug, PartialEq)]
pub struct ContentColor(Color);

/// TODO
#[derive(crate::macros::Rect, Default, Copy, Clone, Debug, PartialEq)]
pub struct Margin(Rect<f32>);

/// TODO
#[derive(crate::macros::Rect, Default, Copy, Clone, Debug, PartialEq)]
pub struct Padding(Rect<f32>);

/// TODO
#[derive(crate::macros::Size2d, Default, Copy, Clone, Debug, PartialEq)]
pub struct BoxSize(Size2d<f32>);

//--
/// TODO
#[derive(crate::macros::Unit, Default, Clone, Copy, Debug, PartialEq)]
pub struct Width(Unit<f32>);

//--
/// TODO
#[derive(crate::macros::Unit, Default, Clone, Copy, Debug, PartialEq)]
pub struct Height(Unit<f32>);

//--
/// TODO
#[derive(crate::macros::Unit, Default, Clone, Copy, Debug, PartialEq)]
pub struct MinWidth(Unit<f32>);

//--
/// TODO
#[derive(crate::macros::Unit, Default, Clone, Copy, Debug, PartialEq)]
pub struct MinHeight(Unit<f32>);

//--
/// TODO
#[derive(crate::macros::Unit, Default, Clone, Copy, Debug, PartialEq)]
pub struct MaxWidth(Unit<f32>);

//--
/// TODO
#[derive(crate::macros::Unit, Default, Clone, Copy, Debug, PartialEq)]
pub struct MaxHeight(Unit<f32>);

//--
/// TODO
#[derive(crate::macros::Rect, Default, Copy, Clone, Debug, PartialEq)]
pub struct BorderWeight(Rect<f32>);

//--
/// TODO
#[derive(crate::macros::Rect, Default, Copy, Clone, Debug, PartialEq)]
pub struct BorderRadius(Rect<f32>);

/// TODO
#[derive(crate::macros::Color, Default, Copy, Clone, Debug, PartialEq)]
pub struct BorderColor(Color);
