use core::any::Any;
use core::any::TypeId;
use core::fmt::Debug;
use core::alloc::Allocator;

use derive_more::*;

use enum_dispatch::*;

use hashbrown::HashMap;
use hashbrown::DefaultHashBuilder;

use num_traits::AsPrimitive;
use num_traits::Zero;

use palette::LinSrgb;
use palette::LinSrgba;
use palette::rgb::FromHexError;
use palette::stimulus::Stimulus;

pub mod prelude {
    pub use super::Style;
    pub use super::StyleSheet;
    pub use super::Property;
}

//---
#[enum_dispatch]
pub trait Style {
    //..
}

#[derive(Debug, Index, IndexMut, Deref, DerefMut)]
pub struct StyleSheet<A: Allocator> {
    #[deref]
    #[deref_mut]
    #[index]
    #[index_mut]
    styles: HashMap<TypeId, Vec<Property, A>, DefaultHashBuilder, A>,
    arena: A,
}

impl<A: Allocator + Copy> StyleSheet<A> {
    pub fn new_in(arena: A) -> Self {
        StyleSheet {
            styles: HashMap::new_in(arena),
            arena,
        }
    }
}

impl<A: Allocator + Copy> StyleSheet<A> {
    pub fn insert<S: Into<Property> + Any>(&mut self, style: S) -> &Self {
        self.styles
            .entry(style.type_id())
            .or_insert_with(|| Vec::new_in(self.arena))
            .push(style.into());
        self
    }
}

#[derive(Debug)]
#[enum_dispatch(Style)]
pub enum Property {
    Margin(Margin),
    Padding(Padding),
    Size(Size),
    Gap(Gap),
    Flex(Flex),
    FlexDirection(FlexDirection),
    Heading(Heading),
    FontStyle(FontStyle),
    ContentColor(ContentColor),
    BackgroundColor(BackgroundColor),
    Border(Border),
    ScrollPosition(ScrollPosition),
}

//---
#[derive(Display, Debug, Clone, Copy, Default, Deref, DerefMut)]
pub struct Unit(pub f64);

impl From<u8> for Unit {
    fn from(value: u8) -> Self {
        Unit(value.as_())
    }
}

impl From<i8> for Unit {
    fn from(value: i8) -> Self {
        Unit(value.as_())
    }
}

impl From<u16> for Unit {
    fn from(value: u16) -> Self {
        Unit(value.as_())
    }
}

impl From<i32> for Unit {
    fn from(value: i32) -> Self {
        Unit(value.as_())
    }
}

impl From<f32> for Unit {
    fn from(value: f32) -> Self {
        Unit(value.as_())
    }
}

impl From<u64> for Unit {
    fn from(value: u64) -> Self {
        Unit(value.as_())
    }
}

impl From<i64> for Unit {
    fn from(value: i64) -> Self {
        Unit(value.as_())
    }
}

impl From<f64> for Unit {
    fn from(value: f64) -> Self {
        Unit(value.as_())
    }
}

impl From<usize> for Unit {
    fn from(value: usize) -> Self {
        Unit(value.as_())
    }
}

impl From<isize> for Unit {
    fn from(value: isize) -> Self {
        Unit(value.as_())
    }
}

impl AsPrimitive<u8> for Unit {
    fn as_(self) -> u8 {
        self.0.as_()
    }
}

impl AsPrimitive<i8> for Unit {
    fn as_(self) -> i8 {
        self.0.as_()
    }
}

impl AsPrimitive<u16> for Unit {
    fn as_(self) -> u16 {
        self.0.as_()
    }
}

impl AsPrimitive<i16> for Unit {
    fn as_(self) -> i16 {
        self.0.as_()
    }
}

impl AsPrimitive<u32> for Unit {
    fn as_(self) -> u32 {
        self.0.as_()
    }
}

impl AsPrimitive<i32> for Unit {
    fn as_(self) -> i32 {
        self.0.as_()
    }
}

impl AsPrimitive<f32> for Unit {
    fn as_(self) -> f32 {
        self.0.as_()
    }
}

impl AsPrimitive<u64> for Unit {
    fn as_(self) -> u64 {
        self.0.as_()
    }
}

impl AsPrimitive<i64> for Unit {
    fn as_(self) -> i64 {
        self.0.as_()
    }
}

impl AsPrimitive<f64> for Unit {
    fn as_(self) -> f64 {
        self.0.as_()
    }
}

impl AsPrimitive<usize> for Unit {
    fn as_(self) -> usize {
        self.0.as_()
    }
}

#[derive(Default, Display, Debug, Clone, Copy, IsVariant)]
pub enum Value {
    #[default]
    Auto,
    Px(Unit),
    Fill(Unit),
    Percent(Unit),
}

impl From<u8> for Value {
    fn from(value: u8) -> Self {
        Value::Px(Unit(value.as_()))
    }
}

impl From<i8> for Value {
    fn from(value: i8) -> Self {
        Value::Px(Unit(value.as_()))
    }
}

impl From<u16> for Value {
    fn from(value: u16) -> Self {
        Value::Px(Unit(value.as_()))
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Value::Px(Unit(value.as_()))
    }
}

impl From<f32> for Value {
    fn from(value: f32) -> Self {
        Value::Px(Unit(value.as_()))
    }
}

impl From<u64> for Value {
    fn from(value: u64) -> Self {
        Value::Px(Unit(value.as_()))
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Value::Px(Unit(value.as_()))
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Value::Px(Unit(value.as_()))
    }
}

impl From<usize> for Value {
    fn from(value: usize) -> Self {
        Value::Px(Unit(value.as_()))
    }
}

impl From<isize> for Value {
    fn from(value: isize) -> Self {
        Value::Px(Unit(value.as_()))
    }
}

impl AsPrimitive<u8> for Value {
    fn as_(self) -> u8 {
        match self {
            Value::Px(n) => n.as_(),
            Value::Fill(n) => n.as_(),
            Value::Percent(n) => n.as_(),
            _ => 0,
        }
    }
}

impl AsPrimitive<i8> for Value {
    fn as_(self) -> i8 {
        match self {
            Value::Px(n) => n.as_(),
            Value::Fill(n) => n.as_(),
            Value::Percent(n) => n.as_(),
            _ => 0,
        }
    }
}

impl AsPrimitive<u16> for Value {
    fn as_(self) -> u16 {
        match self {
            Value::Px(n) => n.as_(),
            Value::Fill(n) => n.as_(),
            Value::Percent(n) => n.as_(),
            _ => 0,
        }
    }
}

impl AsPrimitive<i16> for Value {
    fn as_(self) -> i16 {
        match self {
            Value::Px(n) => n.as_(),
            Value::Fill(n) => n.as_(),
            Value::Percent(n) => n.as_(),
            _ => 0,
        }
    }
}

impl AsPrimitive<u32> for Value {
    fn as_(self) -> u32 {
        match self {
            Value::Px(n) => n.as_(),
            Value::Fill(n) => n.as_(),
            Value::Percent(n) => n.as_(),
            _ => 0,
        }
    }
}

impl AsPrimitive<i32> for Value {
    fn as_(self) -> i32 {
        match self {
            Value::Px(n) => n.as_(),
            Value::Fill(n) => n.as_(),
            Value::Percent(n) => n.as_(),
            _ => 0,
        }
    }
}

impl AsPrimitive<f32> for Value {
    fn as_(self) -> f32 {
        match self {
            Value::Px(n) => n.as_(),
            Value::Fill(n) => n.as_(),
            Value::Percent(n) => n.as_(),
            _ => 0.,
        }
    }
}

impl AsPrimitive<u64> for Value {
    fn as_(self) -> u64 {
        match self {
            Value::Px(n) => n.as_(),
            Value::Fill(n) => n.as_(),
            Value::Percent(n) => n.as_(),
            _ => 0,
        }
    }
}

impl AsPrimitive<i64> for Value {
    fn as_(self) -> i64 {
        match self {
            Value::Px(n) => n.as_(),
            Value::Fill(n) => n.as_(),
            Value::Percent(n) => n.as_(),
            _ => 0,
        }
    }
}

impl AsPrimitive<f64> for Value {
    fn as_(self) -> f64 {
        match self {
            Value::Px(n) => n.as_(),
            Value::Fill(n) => n.as_(),
            Value::Percent(n) => n.as_(),
            _ => 0.,
        }
    }
}

impl AsPrimitive<usize> for Value {
    fn as_(self) -> usize {
        match self {
            Value::Px(n) => n.as_(),
            Value::Fill(n) => n.as_(),
            Value::Percent(n) => n.as_(),
            _ => 0,
        }
    }
}
    
impl Value {
    pub fn is_zero(&self) -> bool {
        match self {
            Value::Auto => false,
            Value::Px(n) => n.is_zero(),
            Value::Fill(n) => n.is_zero(),
            Value::Percent(n) => n.is_zero(),
        }
    }
}

#[derive(Clone, Copy, Default, Display, Debug, IsVariant)]
pub enum Edge {
    #[default]
    All,
    Top,
    Right,
    Bottom,
    Left,
    None,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Size(pub Value, pub Value, pub Value);

impl Style for Size {
    //..
}

impl Size {
    pub fn new<V: Into<Value> + Copy>(value: V) -> Self {
        Size(value.into(), value.into(), value.into())
    }
    
    pub fn width<V: Into<Value> + Copy>(value: V) -> Self {
        Size(value.into(), Value::Auto, Value::Auto)
    }
    
    pub fn x<V: Into<Value> + Copy>(value: V) -> Self {
        Size::width(value.into())
    }
    
    pub fn height<V: Into<Value> + Copy>(value: V) -> Self {
        Size(Value::Auto, value.into(), Value::Auto)
    }
    
    pub fn y<V: Into<Value> + Copy>(value: V) -> Self {
        Size::height(value)
    }
    
    pub fn depth<V: Into<Value> + Copy>(value: V) -> Self {
        Size(Value::Auto, Value::Auto, value.into())
    }
    
    pub fn z<V: Into<Value> + Copy>(value: V) -> Self {
        Size::depth(value)
    }
    
    pub fn xy<V: Into<Value> + Copy>(value: V) -> Self {
        Size(value.into(), value.into(), Value::Auto)
    }
    
    pub fn xyz<V: Into<Value> + Copy>(value: V) -> Self {
        Size::new(value)
    }
}

#[derive(Clone, Copy, Default, Display, Debug)]
#[display("{:} {:}", self.0, self.1)]
pub struct Margin(pub Edge, pub Value);

impl Style for Margin {
    //..
}

impl Margin {
    pub fn new(weight: impl Into<Value>) -> Self {
        Margin::all(weight)
    }
    
    pub fn all(weight: impl Into<Value>) -> Self {
        Margin(Edge::default(), weight.into())
    }
    
    pub fn top(weight: impl Into<Value>) -> Self {
        Margin(Edge::Top, weight.into())
    }
    
    pub fn bottom(weight: impl Into<Value>) -> Self {
        Margin(Edge::Bottom, weight.into())
    }
    
    pub fn left(weight: impl Into<Value>) -> Self {
        Margin(Edge::Left, weight.into())
    }
    
    pub fn right(weight: impl Into<Value>) -> Self {
        Margin(Edge::Right, weight.into())
    }
}

#[derive(Clone, Copy, Default, Display, Debug)]
#[display("{:} {:}", self.0, self.1)]
pub struct Padding(pub Edge, pub Value);

impl Style for Padding {
    //..
}

impl Padding {
    pub fn new(weight: impl Into<Value>) -> Self {
        Padding::all(weight)
    }
    
    pub fn all(weight: impl Into<Value>) -> Self {
        Padding(Edge::default(), weight.into())
    }
    
    pub fn top(weight: impl Into<Value>) -> Self {
        Padding(Edge::Top, weight.into())
    }
    
    pub fn right(weight: impl Into<Value>) -> Self {
        Padding(Edge::Right, weight.into())
    }
    
    pub fn bottom(weight: impl Into<Value>) -> Self {
        Padding(Edge::Bottom, weight.into())
    }
    
    pub fn left(weight: impl Into<Value>) -> Self {
        Padding(Edge::Left, weight.into())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Gap(pub Value);

impl Style for Gap {
    //..
}

//---
#[derive(Clone, Copy, Default, Debug)]
pub struct Flex(pub Unit);

impl Flex {
    pub fn new<U: Into<Unit>>(value: U) -> Self {
        Flex(value.into())
    }
}

impl Style for Flex {
    //..
}

#[derive(Clone, Copy, Default, Debug, IsVariant)]
pub enum FlexDirection {
    #[default]
    Column,
    Row,
}

impl Style for FlexDirection {
    //..
}

//---
#[derive(Clone, Copy, Debug)]
pub enum Heading {
    // Level(usize),
    Primary,
    Secondary,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
}

impl Style for Heading {
    //..
}

//---
#[derive(Clone, Copy, Default, Debug, IsVariant)]
pub enum FontStyle {
    #[default]
    Normal,
    Italic,
}

impl Style for FontStyle {
    //..
}

//---
#[derive(Clone, Copy, Default, Display, Debug, Deref, DerefMut)]
#[display("{:?}", self.0)]
pub struct Color<N: Stimulus + Debug = u8>(pub Option<LinSrgba<N>>);

impl<N: Stimulus + Debug> Color<N> {
    pub fn new(r: N, g: N, b: N, a: N) -> Self {
        Color(Some(LinSrgba::new(r, g, b, a)))
    }
}

impl Color<u8> {
    pub fn try_from(color: &str) -> Result<Self, FromHexError> {
        let color = color.strip_prefix('#').unwrap_or(color);
        match color {
            hex => match hex.len() {
                8 => {
                    color.parse::<LinSrgba<u8>>().map(|color| Color::<u8>(Some(color)))
                }
                _ => {
                    color.parse::<LinSrgb<u8>>().map(|color| Color::<u8>(Some(color.into())))
                }
            }
        }
    }
}

impl From<&str> for Color<u8> {
    fn from(color: &str) -> Self {
        Self::try_from(color).unwrap_or_default()
    }
}

#[derive(Clone, Copy, Default, Display, Debug, Deref, DerefMut)]
pub struct BackgroundColor<N: Stimulus + Debug = u8>(pub Color<N>);

impl<N: Stimulus + Debug> BackgroundColor<N> {
    pub fn new(r: N, g: N, b: N, a: N) -> Self {
        BackgroundColor(Color::new(r, g, b, a))
    }
}

impl BackgroundColor<u8> {
    pub fn try_from(color: &str) -> Result<Self, FromHexError> {
        Color::try_from(color)
            .map(|color| BackgroundColor(color))
    }
}

impl From<&str> for BackgroundColor<u8> {
    fn from(color: &str) -> Self {
        BackgroundColor(Color::from(color))
    }
}

impl Style for BackgroundColor {
    //..
}

#[derive(Clone, Copy, Default, Display, Debug, Deref, DerefMut)]
pub struct ContentColor<N: Stimulus + Debug = u8>(pub Color<N>);

impl<N: Stimulus + Debug> ContentColor<N> {
    pub fn new(r: N, g: N, b: N, a: N) -> Self {
        ContentColor(Color::new(r, g, b, a))
    }
}

impl ContentColor<u8> {
    pub fn try_from(color: &str) -> Result<Self, FromHexError> {
        Color::try_from(color)
            .map(|color| ContentColor(color))
    }
}

impl From<&str> for ContentColor<u8> {
    fn from(color: &str) -> Self {
        ContentColor(Color::from(color))
    }
}

impl Style for ContentColor {
    //..
}

//---
#[derive(Clone, Copy, Display, Default, Debug)]
#[display("{} {} {} {:?}", self.0, self.1, self.2, self.3)]
pub struct Border(pub Edge, pub Value, pub BorderStyle, pub Color);

impl Border {
    pub fn new(weight: impl Into<Value>, style: BorderStyle, color: Option<Color>) -> Self {
        Border::all(weight, style, color)
    }
    
    pub fn all(weight: impl Into<Value>, style: BorderStyle, color: Option<Color>) -> Self {
        Border(Edge::default(), weight.into(), style, color.unwrap_or_default())
    }
    
    pub fn top(weight: impl Into<Value>, style: BorderStyle, color: Option<Color>) -> Self {
        Border(Edge::Top, weight.into(), style, color.unwrap_or_default())
    }
    
    pub fn bottom(weight: impl Into<Value>, style: BorderStyle, color: Option<Color>) -> Self {
        Border(Edge::Bottom, weight.into(), style, color.unwrap_or_default())
    }
    
    pub fn left(weight: impl Into<Value>, style: BorderStyle, color: Option<Color>) -> Self {
        Border(Edge::Left, weight.into(), style, color.unwrap_or_default())
    }
    
    pub fn right(weight: impl Into<Value>, style: BorderStyle, color: Option<Color>) -> Self {
        Border(Edge::Right, weight.into(), style, color.unwrap_or_default())
    }
}

impl Style for Border {
    //..
}

#[derive(Clone, Copy, Default, Display, Debug, IsVariant)]
pub enum BorderStyle {
    #[default]
    Solid,
    Dotted,
    Dashed,
    None,
}

//---
#[derive(Clone, Copy, Default, Debug)]
pub struct ScrollPosition(pub Unit);

impl ScrollPosition {
    pub fn new(value: impl Into<Unit>) -> Self {
        ScrollPosition(value.into())
    }
}

impl Style for ScrollPosition {
    //..
}
