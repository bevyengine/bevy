use bevy_asset::Handle;
use bevy_ecs::component::Component;
use bevy_math::Size;
use bevy_render::color::Color;
use glyph_brush_layout::{HorizontalAlign, VerticalAlign};

use crate::Font;

#[derive(Component, Debug, Default, Clone)]
pub struct Text {
    pub sections: Vec<TextSection>,
    pub alignment: TextAlignment,
}

impl Text {
    /// Constructs a [`Text`] with (initially) one section.
    ///
    /// ```
    /// # use bevy_asset::{AssetServer, Handle};
    /// # use bevy_render::color::Color;
    /// # use bevy_text::{Font, Text, TextAlignment, TextStyle};
    /// # use glyph_brush_layout::{HorizontalAlign, VerticalAlign};
    /// #
    /// # let font_handle: Handle<Font> = Default::default();
    /// #
    /// // basic usage
    /// let hello_world = Text::with_section(
    ///     "hello world!".to_string(),
    ///     TextStyle {
    ///         font: font_handle.clone(),
    ///         font_size: 60.0,
    ///         color: Color::WHITE,
    ///     },
    ///     TextAlignment {
    ///         vertical: VerticalAlign::Center,
    ///         horizontal: HorizontalAlign::Center,
    ///     },
    /// );
    ///
    /// let hello_bevy = Text::with_section(
    ///     // accepts a String or any type that converts into a String, such as &str
    ///     "hello bevy!",
    ///     TextStyle {
    ///         font: font_handle,
    ///         font_size: 60.0,
    ///         color: Color::WHITE,
    ///     },
    ///     // you can still use Default
    ///     Default::default(),
    /// );
    /// ```
    pub fn with_section<S: Into<String>>(
        value: S,
        style: TextStyle,
        alignment: TextAlignment,
    ) -> Self {
        Self {
            sections: vec![TextSection {
                value: value.into(),
                style,
            }],
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

#[derive(Component, Default, Copy, Clone, Debug)]
pub struct Text2dSize {
    pub size: Size,
}
