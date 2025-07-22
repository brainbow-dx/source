// use core::alloc::Allocator;
use core::any::TypeId;
use core::fmt::Debug;
use core::marker::PhantomData;
// use core::fmt::Display;

use alloc::alloc::Global;
use alloc::vec::Vec;

use bumpalo::Bump;

use bumpalo_herd::Herd;
use bumpalo_herd::Member;

use enum_dispatch::enum_dispatch;

use crate::collections::Drain;
use crate::collections::HashMap;

//---
// TODO: Experiment with v2 syntax:
// let style = Margin::<0, 0, 0, 0>;
// struct Margin<
//     const TOP: u8=0,
//     const RIGHT: u8=0,
//     const BOTTOM: u8=0,
//     const LEFT: u8=0,
// >;

//---
/// TODO
pub trait Style: Debug + PartialEq + Into<StyleProperty> {
    //..
}

/// TODO
// #[derive(Debug)]
#[derive(Debug)]
pub struct StyleSheet<'ctx> {
    /// TODO
    styles: HashMap<TypeId, Vec<StyleProperty, &'ctx Bump>, &'ctx Bump>,

    arena: &'ctx Bump,
}

impl<'ctx> StyleSheet<'ctx> {
    /// TODO
    pub fn new_in(arena: &'ctx Bump) -> Self {
        StyleSheet {
            styles: HashMap::new_in(arena),
            arena,
        }
    }
}

impl<'ctx> StyleSheet<'ctx> {
    /// TODO
    pub fn get<P: Style + 'static>(&self) -> Option<&Vec<StyleProperty, &Bump>> {
        let type_id = TypeId::of::<P>();
        self.styles.get(&type_id)
    }

    /// TODO
    pub fn styles(&self) -> &HashMap<TypeId, Vec<StyleProperty, &Bump>, &Bump> {
        &self.styles
    }

    /// TODO
    pub fn push<P: Style + 'static>(&mut self, value: P) {
        let type_id = TypeId::of::<P>();
        self.styles
            .entry(type_id)
            .or_insert_with(|| Vec::new_in(self.arena))
            .push(value.into());
    }

    // Method to append styles from another StyleSheet
    pub fn append(&mut self, other: &mut StyleSheet<'ctx>) {
        for (type_id, mut values) in other.styles.drain() {
            self.styles
                .entry(type_id)
                .or_insert_with(|| Vec::new_in(self.arena))
                .append(&mut values);
        }
    }

    pub fn drain(&mut self) -> Drain<'ctx, TypeId, Vec<StyleProperty, &Bump>, &Bump> {
        self.styles.drain()
    }
}

//---
/// TODO
// #[derive(Debug)]
#[derive(Debug)]
pub struct ComputedStyles<'ctx> {
    /// TODO
    styles: HashMap<TypeId, Vec<StyleProperty>>,

    context: PhantomData<&'ctx ()>,
}

impl ComputedStyles<'_> {
    /// TODO
    pub fn new() -> Self {
        ComputedStyles {
            styles: HashMap::new(),
            context: PhantomData,
        }
    }
}

impl<'ctx> ComputedStyles<'ctx> {
    /// TODO
    pub fn get<P: Style + 'static>(&self) -> Option<&Vec<StyleProperty>> {
        let type_id = TypeId::of::<P>();
        self.styles.get(&type_id)
    }

    /// TODO
    pub fn styles(&self) -> &HashMap<TypeId, Vec<StyleProperty>> {
        &self.styles
    }
}

impl<'ctx> ComputedStyles<'ctx> {
    /// TODO
    pub fn push<P: Style + 'static>(&mut self, value: P) {
        let type_id = TypeId::of::<P>();
        self.styles
            .entry(type_id)
            .or_insert_with(|| Vec::new())
            .push(value.into());
    }

    // Method to append styles from another StyleSheet
    pub fn append(&mut self, other: &mut ComputedStyles) {
        for (type_id, mut values) in other.styles.drain() {
            self.styles
                .entry(type_id)
                .or_insert_with(|| Vec::new())
                .append(&mut values);
        }
    }

    pub fn extend<'src>(&mut self, src_styles: &StyleSheet<'src>) {
        let mut out_styles = HashMap::with_capacity_in(src_styles.styles.len(), Global);

        for (type_id, styles) in src_styles.styles.iter() {
            out_styles.insert(*type_id, styles.to_vec_in(Global));
        }

        self.styles.extend(out_styles)
    }

    pub fn drain(&mut self) -> Drain<'_, TypeId, Vec<StyleProperty>> {
        self.styles.drain()
    }
}

/// Provides (faster?) dynamic dispatch for the StyleValue (via `enum_dispatch`).
///
/// Represents a handle to a StyleValue with a few extra features:
/// 1. TODO
#[derive(chalk::StyleProperty, Clone, PartialEq)]
#[enum_dispatch(StyleValue)]
pub enum StyleProperty {
    Flex(Flex),
    FlexBasis(FlexBasis),
    FlexDirection(FlexDirection),
    FlexGrow(FlexGrow),
    FlexShrink(FlexShrink),
    AlignItems(AlignItems),
    JustifyContent(JustifyContent),
    Gap(Gap),
    BackgroundColor(BackgroundColor),
    Margin(Margin),
    Padding(Padding),
    BoxSize(BoxSize),
    Width(Width),
    Height(Height),
    MinWidth(MinWidth),
    MinHeight(MinHeight),
    MaxWidth(MaxWidth),
    MaxHeight(MaxHeight),
    FontFamily(FontFamily),
    FontSize(FontSize),
    ContentColor(ContentColor),
    BorderWeight(BorderWeight),
    BorderRadius(BorderRadius),
    BorderColor(BorderColor),
}

#[automatically_derived]
impl core::fmt::Debug for StyleProperty {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StyleProperty::Flex(value) => write!(f, "{:?}", value),
            StyleProperty::FlexBasis(value) => write!(f, "{:?}", value),
            StyleProperty::FlexDirection(value) => write!(f, "{:?}", value),
            StyleProperty::FlexGrow(value) => write!(f, "{:?}", value),
            StyleProperty::FlexShrink(value) => write!(f, "{:?}", value),
            StyleProperty::AlignItems(value) => write!(f, "{:?}", value),
            StyleProperty::JustifyContent(value) => write!(f, "{:?}", value),
            StyleProperty::Gap(value) => write!(f, "{:?}", value),
            StyleProperty::BackgroundColor(value) => write!(f, "{:?}", value),
            StyleProperty::Margin(value) => write!(f, "{:?}", value),
            StyleProperty::Padding(value) => write!(f, "{:?}", value),
            StyleProperty::BoxSize(value) => write!(f, "{:?}", value),
            StyleProperty::Width(value) => write!(f, "{:?}", value),
            StyleProperty::Height(value) => write!(f, "{:?}", value),
            StyleProperty::MinWidth(value) => write!(f, "{:?}", value),
            StyleProperty::MinHeight(value) => write!(f, "{:?}", value),
            StyleProperty::MaxWidth(value) => write!(f, "{:?}", value),
            StyleProperty::MaxHeight(value) => write!(f, "{:?}", value),
            StyleProperty::FontFamily(value) => write!(f, "{:?}", value),
            StyleProperty::FontSize(value) => write!(f, "{:?}", value),
            StyleProperty::ContentColor(value) => write!(f, "{:?}", value),
            StyleProperty::BorderWeight(value) => write!(f, "{:?}", value),
            StyleProperty::BorderRadius(value) => write!(f, "{:?}", value),
            StyleProperty::BorderColor(value) => write!(f, "{:?}", value),
        }
    }
}

// #[derive(chalk::StyleProperty, PartialEq)]
// #[enum_dispatch(StyleValue)]
// pub enum Style {
//     // Display & Box Model
//     Display(Display),
//     Position(Position),
//     Top(Top),
//     Right(Right),
//     Bottom(Bottom),
//     Left(Left),
//     ZIndex(ZIndex),
//     Overflow(Overflow),
//     OverflowX(OverflowX),
//     OverflowY(OverflowY),
//     BoxSizing(BoxSizing),

//     // Flexbox
//     Flex(Flex),
//     FlexBasis(FlexBasis),
//     FlexDirection(FlexDirection),
//     FlexGrow(FlexGrow),
//     FlexShrink(FlexShrink),
//     FlexWrap(FlexWrap),
//     Order(Order),
//     AlignItems(AlignItems),
//     AlignSelf(AlignSelf),
//     JustifyContent(JustifyContent),

//     // Grid Layout
//     GridTemplateColumns(GridTemplateColumns),
//     GridTemplateRows(GridTemplateRows),
//     GridColumnGap(GridColumnGap),
//     GridRowGap(GridRowGap),
//     GridTemplateAreas(GridTemplateAreas),
//     GridAutoColumns(GridAutoColumns),
//     GridAutoRows(GridAutoRows),
//     GridAutoFlow(GridAutoFlow),
//     GridColumn(GridColumn),
//     GridRow(GridRow),
//     GridArea(GridArea),

//     // Sizing
//     Width(Width),
//     Height(Height),
//     MinWidth(MinWidth),
//     MinHeight(MinHeight),
//     MaxWidth(MaxWidth),
//     MaxHeight(MaxHeight),
//     BoxSize(BoxSize),
//     AspectRatio(AspectRatio),

//     // Spacing
//     Margin(Margin),
//     Padding(Padding),
//     Gap(Gap),

//     // Borders
//     BorderStyle(BorderStyle),
//     BorderWeight(BorderWeight),
//     BorderRadius(BorderRadius),
//     BorderColor(BorderColor),
//     Border(Border),
//     BorderTop(BorderTop),
//     BorderRight(BorderRight),
//     BorderBottom(BorderBottom),
//     BorderLeft(BorderLeft),

//     // Backgrounds
//     BackgroundColor(BackgroundColor),
//     BackgroundImage(BackgroundImage),
//     BackgroundPosition(BackgroundPosition),
//     BackgroundRepeat(BackgroundRepeat),
//     BackgroundSize(BackgroundSize),
//     BackgroundAttachment(BackgroundAttachment),

//     // Typography
//     FontFamily(FontFamily),
//     FontSize(FontSize),
//     FontStyle(FontStyle),
//     FontWeight(FontWeight),
//     LineHeight(LineHeight),
//     TextAlign(TextAlign),
//     TextDecoration(TextDecoration),
//     TextTransform(TextTransform),
//     LetterSpacing(LetterSpacing),
//     WordSpacing(WordSpacing),
//     WhiteSpace(WhiteSpace),

//     // Effects
//     Opacity(Opacity),
//     BoxShadow(BoxShadow),
//     TextShadow(TextShadow),
//     Filter(Filter),
//     ClipPath(ClipPath),

//     // Transitions & Animations
//     Transition(Transition),
//     TransitionDuration(TransitionDuration),
//     TransitionTimingFunction(TransitionTimingFunction),
//     TransitionDelay(TransitionDelay),
//     Transform(Transform),
//     TransformOrigin(TransformOrigin),
//     Animation(Animation),
//     AnimationName(AnimationName),
//     AnimationDuration(AnimationDuration),
//     AnimationTimingFunction(AnimationTimingFunction),
//     AnimationDelay(AnimationDelay),
//     AnimationIterationCount(AnimationIterationCount),
//     AnimationDirection(AnimationDirection),

//     // Miscellaneous
//     ContentColor(ContentColor),
//     Visibility(Visibility),
//     Cursor(Cursor),
// }

use core::fmt::Debug;
use core::fmt::Display;
use core::num::ParseIntError;

// use alloc::vec::Vec;
// use alloc::format;

//---
#[derive(Default, Debug, PartialEq)]
pub struct Weight<P>(pub P)
where
    P: Display + Debug + PartialEq;

/// TODO
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub enum Unit<P = f32>
where
    P: Display + Debug + PartialEq,
{
    /// TODO
    Px(P),

    /// TOD=
    Percent(P),

    /// TODO
    Full,

    /// TODO
    Auto,

    /// TODO
    #[default]
    None,
}

impl<P> Display for Unit<P>
where
    P: Display + Debug + PartialEq,
{
    /// TODO
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Unit::Px(value) => write!(f, "{}px", value),
            Unit::Percent(value) => write!(f, "{}%", value),
            Unit::Full => write!(f, "full"),
            Unit::Auto => write!(f, "auto"),
            Unit::None => write!(f, "none"),
        }
    }
}

impl From<f32> for Unit<f32> {
    /// TODO
    fn from(value: f32) -> Self {
        Unit::Px(value)
    }
}

impl<P> From<Option<P>> for Unit<P>
where
    P: Display + Debug + PartialEq,
{
    /// TODO
    fn from(value: Option<P>) -> Self {
        match value {
            Some(value) => Unit::Px(value),
            None => Unit::None,
        }
    }
}

/// TODO
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Size2d<P = f32>(pub(crate) Unit<P>, pub(crate) Unit<P>)
where
    P: Display + Debug + PartialEq;

impl<P> Size2d<P>
where
    P: Display + Debug + PartialEq,
{
    /// TODO
    pub fn xy<U1: Into<Unit<P>>, U2: Into<Unit<P>>>(x: U1, y: U2) -> Self {
        Size2d(x.into(), y.into())
    }

    /// TODO
    pub fn x(&self) -> &Unit<P> {
        &self.0
    }

    /// TODO
    pub fn y(&self) -> &Unit<P> {
        &self.1
    }
}

/// TODO
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub struct Rect<P = f32>(
    pub(crate) Unit<P>,
    pub(crate) Unit<P>,
    pub(crate) Unit<P>,
    pub(crate) Unit<P>,
)
where
    P: Display + Debug + PartialEq;

impl<P> Rect<P>
where
    P: Copy + Display + Debug + PartialEq,
{
    /// TODO
    pub fn all<U1: Into<Unit<P>>, U2: Into<Unit<P>>, U3: Into<Unit<P>>, U4: Into<Unit<P>>>(
        top: U1,
        right: U2,
        bottom: U3,
        left: U4,
    ) -> Self {
        Rect(top.into(), right.into(), bottom.into(), left.into())
    }

    /// TODO
    pub fn xy<U1: Into<Unit<P>> + Copy, U2: Into<Unit<P>> + Copy>(x: U1, y: U2) -> Self {
        Rect(x.into(), y.into(), x.into(), y.into())
    }
}

impl<P> Rect<P>
where
    P: Display + Debug + PartialEq,
{
    /// TODO
    pub fn top(&self) -> &Unit<P> {
        &self.0
    }

    /// TODO
    pub fn right(&self) -> &Unit<P> {
        &self.1
    }

    /// TODO
    pub fn bottom(&self) -> &Unit<P> {
        &self.2
    }

    /// TODO
    pub fn left(&self) -> &Unit<P> {
        &self.3
    }
}

/// Represents a color in various formats.
#[derive(Default, Copy, Clone, Debug, PartialEq)]
pub enum Color {
    /// RGBA color format.
    Rgba(u8, u8, u8, u8),

    /// HSLA color format.
    Hsla(f32, f32, f32, f32),

    /// TODO
    #[default]
    Transparent,
}

impl Color {
    /// TODO
    pub fn hex(hex: &str) -> Result<Self, ColorError> {
        Self::decode_hex_color(hex)
    }

    /// TODO
    /// Hat-tip: https://play.rust-lang.org/?version=stable&mode=debug&edition=2015&gist=e241493d100ecaadac3c99f37d0f766f
    pub fn decode_hex_color(s: &str) -> Result<Color, ColorError> {
        let mut bytes: [u8] = [0, 0, 0, 255];
        let s = s.trim_start_matches('#');

        match s.len() {
            6 | 8 => {
                for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
                    let chunk = chunk.map_err(Color::chunk_error)?;

                    let chunk = chunk.unwrap("TODO: Unwrap this safely.");
                    bytes[i] = u8::from_str_radix(core::str::from_utf8(chunk), 16)
                        .map_err(ColorError::ParseInt)?;
                }
            }
            _ => return Err(ColorError::InvalidLength),
        }

        Ok(Color::Rgba(bytes[0], bytes[1], bytes[2], bytes[3]))
    }

    pub fn chunk_error<E: Error>(error: E) -> E {
        tracing::warn!("Chunk Error: {:}", err);
        return error;
    }
}

/// TODO
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColorError {
    /// TODO
    OddLength,

    /// TODO
    InvalidLength,

    /// TODO
    ParseInt(ParseIntError),

    /// TODO
    ChunkError(String),
}

impl From<ChunkError> for ColorError {
    /// TODO
    fn from(error: ChunkError) -> Self {
        ColorError::ChunkError(error)
    }
}

impl From<ParseIntError> for ColorError {
    /// TODO
    fn from(error: ParseIntError) -> Self {
        ColorError::ParseInt(error)
    }
}
