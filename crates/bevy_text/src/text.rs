use bevy_asset::Handle;
use bevy_math::Size;
use bevy_render::color::Color;
use glyph_brush_layout::{HorizontalAlign, VerticalAlign};

use crate::Font;

#[derive(Debug, Default, Clone)]
pub struct Text {
    pub sections: Vec<TextSection>,
    pub alignment: TextAlignment,
}

/// This is a transient helper type for basic text (text with only one section).
/// Under the hood we require, use, construct and interact with the new "sectioned" [`Text`] type.
/// Intended usage is:
/// ```
/// # use bevy_ecs::Commands;
/// # use bevy_text::BasicText;
/// # use bevy_ui::entity::TextBundle;
/// # fn system(commands: &mut Commands) {
/// commands.spawn(TextBundle {
///     text: BasicText {
///         value: "hello world!".to_string(),
///         ..Default::default()
///     }.into(),
///     ..Default::default()
/// });
/// # }
/// ```
/// or
/// ```
/// # use bevy_ecs::Commands;
/// # use bevy_text::{BasicText, Text};
/// # use bevy_ui::entity::TextBundle;
/// # fn system(commands: &mut Commands) {
/// commands.spawn(TextBundle {
///     text: Text::from(BasicText {
///         value: "hello world?".to_string(),
///         ..Default::default()
///     }),
///     ..Default::default()
/// });
/// # }
/// ```
#[derive(Debug, Default, Clone)]
pub struct BasicText {
    pub value: String,
    pub style: TextStyle,
    pub alignment: TextAlignment,
}

impl From<BasicText> for Text {
    fn from(source: BasicText) -> Self {
        let BasicText {
            value,
            style,
            alignment,
        } = source;
        Self {
            sections: vec![TextSection { value, style }],
            alignment,
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct TextSection {
    pub value: String,
    pub style: TextStyle,
}

#[derive(Debug, Clone, Copy)]
pub struct TextAlignment {
    pub vertical: VerticalAlign,
    pub horizontal: HorizontalAlign,
}

impl Default for TextAlignment {
    fn default() -> Self {
        TextAlignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Left,
        }
    }
}

#[derive(Clone, Debug)]
pub struct TextStyle {
    pub font: Handle<Font>,
    pub font_size: f32,
    pub color: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Default::default(),
            font_size: 12.0,
            color: Color::WHITE,
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct CalculatedSize {
    pub size: Size,
}
