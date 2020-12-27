use bevy_asset::Handle;
use bevy_math::Size;

use crate::{Font, TextStyle};

#[derive(Debug, Default, Clone)]
pub struct Text {
    pub value: String,
    pub font: Handle<Font>,
    pub style: TextStyle,
}

#[derive(Default, Copy, Clone, Debug)]
pub struct CalculatedSize {
    pub size: Size,
}
