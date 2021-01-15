use bevy_asset::Handle;
use bevy_math::Size;

use crate::{Font, TextAlignment, TextStyle};

#[derive(Debug, Default, Clone)]
pub struct Text {
    pub sections: Vec<TextSection>,
    pub alignment: TextAlignment,
}

/// This is a transient helper type for basic text (text with only one section).
/// Under the hood we require, use, construct and interact with the new "sectioned" [`Text`] type.
/// Intended usage is:
/// ```no_run
/// commands.spawn(TextBundle {
///     text: BasicText {
///         value: "hello world!",
///         ..Default::default()
///     }.into(),
///     ..Default::default()
/// });
/// ```
/// or
/// ```no_run
/// commands.spawn(TextBundle {
///     text: Text::from(BasicText {
///         value: "hello world?",
///         ..Default::default()
///     }),
///     ..Default::default()
/// });
/// ```
#[derive(Debug, Default, Clone)]
pub struct BasicText {
    pub value: String,
    pub font: Handle<Font>,
    pub style: TextStyle,
    pub alignment: TextAlignment,
}

impl From<BasicText> for Text {
    fn from(source: BasicText) -> Self {
        let BasicText {
            value,
            font,
            style,
            alignment,
        } = source;
        Self {
            sections: vec![TextSection { value, font, style }],
            alignment,
        }
    }
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
