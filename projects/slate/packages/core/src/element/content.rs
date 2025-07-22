use alloc::string::String;

use crate::element::DrawFn;
use crate::element::Element;

//---
pub enum Content<'element> {
    Text(&'element str),
    Image(&'element [u8]),
    WebView(&'element str),
}

#[derive(Default, Clone, Hash, Debug)]
pub struct TextBlock<'element> {
    text: &'element str,
}

impl<'element> TextBlock<'element> {
    pub fn with_text<S: Into<&'element str>>(mut self, value: S) -> Self {
        self.text = value.into();
        self // etc.
    }
}

impl Element for TextBlock<'_> {
    fn content(&self) -> Option<Content<'_>> {
        Some(Content::Text(self.text))
    }
}

//--
#[derive(Default, Clone, Hash, Debug)]
pub struct WebView {
    address: String,
}

impl WebView {
    pub fn with_address<S: Into<String>>(mut self, value: S) -> Self {
        self.address = value.into();
        self // etc.
    }
}

impl Element for WebView {
    fn content(&self) -> Option<Content<'_>> {
        Some(Content::WebView(self.address.as_ref()))
    }

    fn draw(&self) -> DrawFn {
        self.draw_mobile()
    }
}

impl WebView {
    pub fn draw_mobile(&self) -> DrawFn {
        slate_macros::uix! {
            //..
        }
    }
}
