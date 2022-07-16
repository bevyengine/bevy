use bevy_asset::Handle;
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_reflect::{prelude::*, FromReflect};
use bevy_render::color::Color;
use serde::{Deserialize, Serialize};

use crate::Font;

#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Text {
    pub sections: Vec<TextSection>,
    pub alignment: TextAlignment,
}

impl Text {
    /// Constructs a [`Text`] with a single section.
    ///
    /// ```
    /// # use bevy_asset::{AssetServer, Handle};
    /// # use bevy_render::color::Color;
    /// # use bevy_text::{Font, Text, TextAlignment, TextStyle, HorizontalAlign, VerticalAlign};
    /// #
    /// # let font_handle: Handle<Font> = Default::default();
    /// #
    /// // Basic usage.
    /// let hello_world = Text::from_section(
    ///     // Accepts a String or any type that converts into a String, such as &str.
    ///     "hello world!",
    ///     TextStyle {
    ///         font: font_handle.clone(),
    ///         font_size: 60.0,
    ///         color: Color::WHITE,
    ///     },
    /// );
    ///
    /// let hello_bevy = Text::from_section(
    ///     "hello bevy!",
    ///     TextStyle {
    ///         font: font_handle,
    ///         font_size: 60.0,
    ///         color: Color::WHITE,
    ///     },
    /// ) // You can still add an alignment.
    /// .with_alignment(TextAlignment::CENTER);
    /// ```
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            sections: vec![TextSection {
                value: value.into(),
                style,
            }],
            alignment: Default::default(),
        }
    }

    pub fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            alignment: Default::default(),
        }
    }

    /// Returns this [`Text`] with a new [`TextAlignment`].
    pub fn with_alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }
}

#[derive(Debug, Default, Clone, FromReflect, Reflect)]
pub struct TextSection {
    pub value: String,
    pub style: TextStyle,
}

impl TextSection {
    /// Create a [`TextSection`] from a string of text.
    pub fn from_text(text: impl Into<String>) -> Self {
        Self {
            value: text.into(),
            style: Default::default(),
        }
    }

    /// Create an empty [`TextSection`] from a style. Useful when the text will be set dynamically.
    pub fn from_style(style: TextStyle) -> Self{
        Self {
            value: Default::default(),
            style,
        }
    }
}

#[derive(Debug, Clone, Copy, Reflect)]
pub struct TextAlignment {
    pub vertical: VerticalAlign,
    pub horizontal: HorizontalAlign,
}

impl TextAlignment {
    /// A [`TextAlignment`] set to center on both axes.
    pub const CENTER: Self = TextAlignment {
        vertical: VerticalAlign::Center,
        horizontal: HorizontalAlign::Center,
    };
}

impl Default for TextAlignment {
    fn default() -> Self {
        TextAlignment {
            vertical: VerticalAlign::Top,
            horizontal: HorizontalAlign::Left,
        }
    }
}

/// Describes horizontal alignment preference for positioning & bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[reflect_value(Serialize, Deserialize)]
pub enum HorizontalAlign {
    /// Leftmost character is immediately to the right of the render position.<br/>
    /// Bounds start from the render position and advance rightwards.
    Left,
    /// Leftmost & rightmost characters are equidistant to the render position.<br/>
    /// Bounds start from the render position and advance equally left & right.
    Center,
    /// Rightmost character is immetiately to the left of the render position.<br/>
    /// Bounds start from the render position and advance leftwards.
    Right,
}

impl From<HorizontalAlign> for glyph_brush_layout::HorizontalAlign {
    fn from(val: HorizontalAlign) -> Self {
        match val {
            HorizontalAlign::Left => glyph_brush_layout::HorizontalAlign::Left,
            HorizontalAlign::Center => glyph_brush_layout::HorizontalAlign::Center,
            HorizontalAlign::Right => glyph_brush_layout::HorizontalAlign::Right,
        }
    }
}

/// Describes vertical alignment preference for positioning & bounds. Currently a placeholder
/// for future functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[reflect_value(Serialize, Deserialize)]
pub enum VerticalAlign {
    /// Characters/bounds start underneath the render position and progress downwards.
    Top,
    /// Characters/bounds center at the render position and progress outward equally.
    Center,
    /// Characters/bounds start above the render position and progress upward.
    Bottom,
}

impl From<VerticalAlign> for glyph_brush_layout::VerticalAlign {
    fn from(val: VerticalAlign) -> Self {
        match val {
            VerticalAlign::Top => glyph_brush_layout::VerticalAlign::Top,
            VerticalAlign::Center => glyph_brush_layout::VerticalAlign::Center,
            VerticalAlign::Bottom => glyph_brush_layout::VerticalAlign::Bottom,
        }
    }
}

#[derive(Clone, Debug, Reflect, FromReflect)]
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
