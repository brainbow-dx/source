use core::fmt::Debug;

use derive_more::Display;

use crate::draw::DrawContext;
use crate::scaffold::Scaffold;
use crate::style::Value;
use crate::style::Size;
use crate::style::Gap;
use crate::style::FlexDirection;
use crate::style::FontStyle;
use crate::style::ContentColor;
use crate::event::keyboard::KeyboardEvent;

//--
pub trait Element {
    fn draw(&self, _: DrawContext) -> impl FnOnce(Scaffold) -> Scaffold {
        |s| s
    }
}

//--
#[derive(Default, Display, Debug)]
pub struct Container;

impl Element for Container {
    //..
}

#[derive(Default, Display, Debug)]
pub struct Text<T>(pub T);

impl<T> Text<T> {
    pub fn new<V: Into<T>>(text: V) -> Self {
        Text(text.into())
    }
    
    pub fn with_content<C: Into<T>>(mut self, value: C) -> Self {
        self.0 = value.into();
        self // etc ..
    }
}

impl<T: AsRef<str>> Text<T> {
    pub fn content(&self) -> &str {
        self.0.as_ref()
    }
}

impl<T: AsRef<str>> Element for Text<T> {
    fn draw(&self, _: DrawContext) -> impl FnOnce(Scaffold) -> Scaffold {
        move |text| {
            text
                // TODO: Allocate content in the DrawContext ..
                .with_content(Some(self.0.as_ref().to_string()))
        }
    }
}

#[derive(Default)]
pub struct Input<T>{
    pub value: T,
    pub placeholder: Option<T>,
}

impl<T> Input<T> {
    pub fn new<V: Into<T>>(text: V) -> Self {
        Input {
            value: text.into(),
            placeholder: None,
        }
    }
    
    pub fn with_value<V: Into<T>>(mut self, value: V) -> Self {
        self.value = value.into();
        self // etc ..
    }
    
    pub fn with_placeholder<P: Into<T>>(mut self, placeholder: P) -> Self {
        self.placeholder = Some(placeholder.into());
        self // etc ..
    }
}

impl<V: AsRef<str> + Default> Element for Input<V> {
    fn draw(&self, _: DrawContext) -> impl FnOnce(Scaffold) -> Scaffold {
        move |input| {
            let current_value = self.value.as_ref().to_owned();
            let placeholder = self.placeholder.as_ref().map(|placeholder| placeholder.as_ref().to_owned());
            
            // TODO: Support local allocation for the input's contents ..
            // let some_bump_string = crate::draw::format!(in ctx.arena(), "TODO: {}", "");
            
            input
                .with_style(FlexDirection::Row)
                .with_style(Gap(Value::from(1)))
                .with_handler::<KeyboardEvent>({
                    move |event| match event.key {
                        _ => {} // tracing::debug!("TODO: Handle keyboard events!"),
                    }
                })
                .with_slot::<InputIcon>(|prefix| {
                    prefix
                        .with_style(FlexDirection::Row)
                        .with_style(Size::width(1))
                        .with_content(Some("$"))
                })
                .with_slot::<InputValue>(|text| {
                    if current_value.is_empty() {
                        text
                            .with_style(FlexDirection::Row)
                            .with_style(FontStyle::Italic)
                            .with_style(ContentColor::from("#555555ff"))
                            .with_content(placeholder)
                    } else {
                        text
                            .with_style(FlexDirection::Row)
                            // .with_style(Size::width(current_value.len()))
                            .with_style(Size::width(current_value.len() + 1))
                            .with_content(Some(current_value))
                            .with_slot::<InputCursor>(|cursor| {
                                cursor
                                    .with_style(FlexDirection::Row)
                                    .with_style(Size::width(1))
                                    // .with_style(BackgroundColor::from("#aaaaaaff"))
                                    .with_content(Some("_"))
                            })
                    }
                })
        }
    }
}

//---
pub struct InputIcon;

pub struct InputValue;

pub struct InputCursor;

//---
// TODO: Move these to escher_scaffold and impl Slot for each.
pub struct Legend;

pub struct Header;

pub struct Body;

pub struct Footer;

pub struct Content;

pub struct Children;
