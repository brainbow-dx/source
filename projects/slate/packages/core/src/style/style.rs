use core::any::TypeId;
use core::fmt::Debug;
use core::marker::PhantomData;

use hashbrown::DefaultHashBuilder;

use alloc::alloc::Global;
use alloc::vec::Vec;

use bumpalo::Bump;

use hashbrown::hash_map::Drain;
use hashbrown::hash_map::HashMap;

use enum_dispatch::enum_dispatch;

// use core::hash::Hasher;
// use core::hash::BuildHasherDefault;
// use ahash::AHasher;

//---
// TODO: Experiment with v2 syntax:
// let style = Margin::<0, 0, 0, 0>;
// struct Margin<
//     const TOP: u8=0,
//     const RIGHT: u8=0,
//     const BOTTOM: u8=0,
//     const LEFT: u8=0,
// >;

/// TODO
pub trait Style: Debug + PartialEq + Into<StyleValue> {
    //..
}

/// TODO
// #[derive(Debug)]
#[cfg(feature = "bump")]
#[derive(Debug)]
pub struct StyleSheet<'ctx> {
    /// TODO
    styles: HashMap<TypeId, Vec<StyleValue, &'ctx Bump>, DefaultHashBuilder, &'ctx Bump>,

    context: &'ctx Bump,
}

#[cfg(feature = "bump")]
impl<'ctx> StyleSheet<'ctx> {
    /// TODO
    pub fn new_in(arena: &'ctx Bump) -> Self {
        StyleSheet {
            styles: HashMap::new_in(arena),
            context: arena,
        }
    }
}

#[cfg(feature = "bump")]
impl<'ctx> StyleSheet<'ctx> {
    /// TODO
    pub fn get<P: Style + 'static>(&self) -> Option<&Vec<StyleValue, &Bump>> {
        let type_id = TypeId::of::<P>();
        self.styles.get(&type_id)
    }

    /// TODO
    pub fn styles(&self) -> &HashMap<TypeId, Vec<StyleValue, &Bump>, DefaultHashBuilder, &Bump> {
        &self.styles
    }

    /// TODO
    pub fn push<P: Style + 'static>(&mut self, value: P) {
        let type_id = TypeId::of::<P>();
        self.styles
            .entry(type_id)
            .or_insert_with(|| Vec::new_in(self.context))
            .push(value.into());
    }

    // Method to append styles from another StyleSheet
    pub fn append(&mut self, other: &mut StyleSheet<'ctx>) {
        for (type_id, mut values) in other.styles.drain() {
            self.styles
                .entry(type_id)
                .or_insert_with(|| Vec::new_in(self.context))
                .append(&mut values);
        }
    }

    pub fn drain(&mut self) -> Drain<'ctx, TypeId, Vec<StyleValue, &Bump>, &Bump> {
        self.styles.drain()
    }
}

//---
/// TODO
// #[derive(Debug)]
#[derive(Debug)]
pub struct ComputedStyles<'ctx> {
    /// TODO
    styles: HashMap<TypeId, Vec<StyleValue>>,

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
    pub fn get<P: Style + 'static>(&self) -> Option<&Vec<StyleValue>> {
        let type_id = TypeId::of::<P>();
        self.styles.get(&type_id)
    }

    /// TODO
    pub fn styles(&self) -> &HashMap<TypeId, Vec<StyleValue>> {
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
        let mut out_styles = HashMap::with_capacity(src_styles.styles.len());

        for (type_id, styles) in src_styles.styles.iter() {
            out_styles.insert(*type_id, styles.to_vec_in(Global));
        }

        self.styles.extend(out_styles)
    }

    pub fn drain(&mut self) -> Drain<'_, TypeId, Vec<StyleValue>> {
        self.styles.drain()
    }
}

/// Provides (faster?) dynamic dispatch for the StyleValue (via `enum_dispatch`).
///
/// Represents a handle to a StyleValue with a few extra features:
/// 1. TODO
#[derive(crate::macros::StyleProperty, Clone, PartialEq)]
#[enum_dispatch(StyleValue)]
pub enum StyleValue {
    Flex(crate::style::property::Flex),
    FlexBasis(crate::style::property::FlexBasis),
    FlexDirection(crate::style::property::FlexDirection),
    FlexGrow(crate::style::property::FlexGrow),
    FlexShrink(crate::style::property::FlexShrink),
    AlignItems(crate::style::property::AlignItems),
    JustifyContent(crate::style::property::JustifyContent),
    Gap(crate::style::property::Gap),
    BackgroundColor(crate::style::property::BackgroundColor),
    Margin(crate::style::property::Margin),
    Padding(crate::style::property::Padding),
    BoxSize(crate::style::property::BoxSize),
    Width(crate::style::property::Width),
    Height(crate::style::property::Height),
    MinWidth(crate::style::property::MinWidth),
    MinHeight(crate::style::property::MinHeight),
    MaxWidth(crate::style::property::MaxWidth),
    MaxHeight(crate::style::property::MaxHeight),
    FontFamily(crate::style::property::FontFamily),
    FontSize(crate::style::property::FontSize),
    ContentColor(crate::style::property::ContentColor),
    BorderWeight(crate::style::property::BorderWeight),
    BorderRadius(crate::style::property::BorderRadius),
    BorderColor(crate::style::property::BorderColor),
}

#[automatically_derived]
impl core::fmt::Debug for StyleValue {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            StyleValue::Flex(value) => write!(f, "{:?}", value),
            StyleValue::FlexBasis(value) => write!(f, "{:?}", value),
            StyleValue::FlexDirection(value) => write!(f, "{:?}", value),
            StyleValue::FlexGrow(value) => write!(f, "{:?}", value),
            StyleValue::FlexShrink(value) => write!(f, "{:?}", value),
            StyleValue::AlignItems(value) => write!(f, "{:?}", value),
            StyleValue::JustifyContent(value) => write!(f, "{:?}", value),
            StyleValue::Gap(value) => write!(f, "{:?}", value),
            StyleValue::BackgroundColor(value) => write!(f, "{:?}", value),
            StyleValue::Margin(value) => write!(f, "{:?}", value),
            StyleValue::Padding(value) => write!(f, "{:?}", value),
            StyleValue::BoxSize(value) => write!(f, "{:?}", value),
            StyleValue::Width(value) => write!(f, "{:?}", value),
            StyleValue::Height(value) => write!(f, "{:?}", value),
            StyleValue::MinWidth(value) => write!(f, "{:?}", value),
            StyleValue::MinHeight(value) => write!(f, "{:?}", value),
            StyleValue::MaxWidth(value) => write!(f, "{:?}", value),
            StyleValue::MaxHeight(value) => write!(f, "{:?}", value),
            StyleValue::FontFamily(value) => write!(f, "{:?}", value),
            StyleValue::FontSize(value) => write!(f, "{:?}", value),
            StyleValue::ContentColor(value) => write!(f, "{:?}", value),
            StyleValue::BorderWeight(value) => write!(f, "{:?}", value),
            StyleValue::BorderRadius(value) => write!(f, "{:?}", value),
            StyleValue::BorderColor(value) => write!(f, "{:?}", value),
        }
    }
}

// #[derive(slate_chalk::StyleProperty, PartialEq)]
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
