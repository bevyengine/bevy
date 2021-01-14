use bevy_asset::Handle;
use bevy_math::Size;

use crate::{Font, TextAlignment, TextStyle};

#[derive(Debug, Default, Clone)]
pub struct Text {
    pub sections: Vec<TextSection>,
    pub alignment: TextAlignment,
}

#[derive(Debug, Default, Clone)]
pub struct TextSection {
    pub value: String,
    pub font: Handle<Font>,
    pub style: TextStyle,
}

#[derive(Default, Copy, Clone, Debug)]
pub struct CalculatedSize {
    pub size: Size,
}
