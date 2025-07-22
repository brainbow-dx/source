use crate::element::DrawFn;

use super::*;

use alloc::string::String;

#[derive(Element, Default, Clone, Hash, Debug)]
pub struct Label;

impl Label {
    pub fn with_text(self, _text: &str) -> Self {
        self // etc.
    }
}

#[derive(Element, Default, Clone, Hash, Debug)]
pub struct TextInput;

impl TextInput {
    pub fn with_value(self, _value: &str) -> Self {
        self // etc.
    }
}

#[derive(Default, Clone, Hash, Debug)]
// #[render(self.draw)]
pub struct Button(String);

impl Button {
    pub fn with_value(mut self, value: &str) -> Self {
        self.0 = String::from(value);
        self // etc.
    }
}

impl Element for Button {
    fn draw(&self) -> DrawFn {
        // chizel::uix! {
        //     ^self {
        //         BackgroundColor::hex("#000000"),
        //     }

        //     .label {
        //         ContentColor::hex("#000000"),
        //     }

        //     #[class(label)]
        //     <Label text="TODO" />
        // }
        |_scaffold| Ok(())
    }
}
