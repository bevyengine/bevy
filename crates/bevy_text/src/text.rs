use bevy_asset::Handle;
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_reflect::{prelude::*, FromReflect};
use bevy_render::color::Color;
use bevy_utils::default;
use serde::{Deserialize, Serialize};

use crate::Font;

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Text {
    pub sections: Vec<TextSection>,
    /// The text's internal justification.
    /// Should not affect its position within a container.
    pub justify: JustifyText,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            sections: Default::default(),
            justify: JustifyText::JustifyLeft,
        }
    }
}

impl Text {
    /// Constructs a [`Text`] with a single section.
    ///
    /// ```
    /// # use bevy_asset::Handle;
    /// # use bevy_render::color::Color;
    /// # use bevy_text::{Font, Text, TextStyle, JustifyText};
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
    /// ) // You can still add text justification.
    /// .with_justify(JustifyText::JustifyCenter);
    /// ```
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            sections: vec![TextSection::new(value, style)],
            ..default()
        }
    }

    /// Constructs a [`Text`] from a list of sections.
    ///
    /// ```
    /// # use bevy_asset::Handle;
    /// # use bevy_render::color::Color;
    /// # use bevy_text::{Font, Text, TextStyle, TextSection};
    /// #
    /// # let font_handle: Handle<Font> = Default::default();
    /// #
    /// let hello_world = Text::from_sections([
    ///     TextSection::new(
    ///         "Hello, ",
    ///         TextStyle {
    ///             font: font_handle.clone(),
    ///             font_size: 60.0,
    ///             color: Color::BLUE,
    ///         },
    ///     ),
    ///     TextSection::new(
    ///         "World!",
    ///         TextStyle {
    ///             font: font_handle,
    ///             font_size: 60.0,
    ///             color: Color::RED,
    ///         },
    ///     ),
    /// ]);
    /// ```
    pub fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            ..default()
        }
    }

    /// Returns this [`Text`] with a new [`JustifyText`].
    pub const fn with_justify(mut self, justify: JustifyText) -> Self {
        self.justify = justify;
        self
    }
}

#[derive(Debug, Default, Clone, FromReflect, Reflect)]
pub struct TextSection {
    pub value: String,
    pub style: TextStyle,
}

impl TextSection {
    /// Create a new [`TextSection`].
    pub fn new(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            value: value.into(),
            style,
        }
    }

    /// Create an empty [`TextSection`] from a style. Useful when the value will be set dynamically.
    pub const fn from_style(style: TextStyle) -> Self {
        Self {
            value: String::new(),
            style,
        }
    }
}

/// Describes horizontal alignment preference for positioning & bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum JustifyText {
    /// Leftmost character is immediately to the right of the render position.<br/>
    /// Bounds start from the render position and advance rightwards.
    JustifyLeft,
    /// Leftmost & rightmost characters are equidistant to the render position.<br/>
    /// Bounds start from the render position and advance equally left & right.
    JustifyCenter,
    /// Rightmost character is immediately to the left of the render position.<br/>
    /// Bounds start from the render position and advance leftwards.
    JustifyRight,
}

impl From<JustifyText> for glyph_brush_layout::HorizontalAlign {
    fn from(val: JustifyText) -> Self {
        match val {
            JustifyText::JustifyLeft => glyph_brush_layout::HorizontalAlign::Left,
            JustifyText::JustifyCenter => glyph_brush_layout::HorizontalAlign::Center,
            JustifyText::JustifyRight => glyph_brush_layout::HorizontalAlign::Right,
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
