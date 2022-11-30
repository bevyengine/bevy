use bevy_asset::Handle;
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_reflect::{prelude::*, FromReflect};
use bevy_render::color::Color;
use serde::{Deserialize, Serialize};

use crate::Font;

#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Default)]
pub struct Text {
    pub sections: Vec<TextSection>,
    pub alignment: TextAlignment,
}

impl Default for Text {
    fn default() -> Self {
        Self {
            sections: Default::default(),
            alignment: TextAlignment::Left,
        }
    }
}

pub trait TextBlock<A>: Sized {
    fn from_section(value: impl Into<String>, style: TextStyle) -> Self;
    fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self;
    fn with_alignment(self, alignment: A) -> Self;
}

impl TextBlock<TextAlignment> for Text {
    /// Constructs a [`Text`] with a single section.
    ///
    /// ```
    /// # use bevy_asset::Handle;
    /// # use bevy_render::color::Color;
    /// # use bevy_text::{Font, Text, TextStyle, HorizontalAlign, VerticalAlign};
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
    /// .with_alignment(HorizontalAlign::CENTER);
    /// ```
    fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            sections: vec![TextSection::new(value, style)],
            ..Default::default()
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
    fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            ..Default::default()
        }
    }

    /// Returns this [`Text`] with a new [`HorizontalAlign`].
    fn with_alignment(mut self, alignment: TextAlignment) -> Self {
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

#[derive(Debug, Clone, Copy, Reflect)]
pub struct Text2dAlignment {
    pub vertical: VerticalAlign,
    pub horizontal: TextAlignment,
}

impl Text2dAlignment {
    /// A [`TextAlignment`] set to the top-left.
    pub const TOP_LEFT: Self = Text2dAlignment {
        vertical: VerticalAlign::Top,
        horizontal: TextAlignment::Left,
    };

    /// A [`TextAlignment`] set to the top-center.
    pub const TOP_CENTER: Self = Text2dAlignment {
        vertical: VerticalAlign::Top,
        horizontal: TextAlignment::Center,
    };

    /// A [`TextAlignment`] set to the top-right.
    pub const TOP_RIGHT: Self = Text2dAlignment {
        vertical: VerticalAlign::Top,
        horizontal: TextAlignment::Right,
    };

    /// A [`TextAlignment`] set to center the center-left.
    pub const CENTER_LEFT: Self = Text2dAlignment {
        vertical: VerticalAlign::Center,
        horizontal: TextAlignment::Left,
    };

    /// A [`TextAlignment`] set to center on both axes.
    pub const CENTER: Self = Text2dAlignment {
        vertical: VerticalAlign::Center,
        horizontal: TextAlignment::Center,
    };

    /// A [`TextAlignment`] set to the center-right.
    pub const CENTER_RIGHT: Self = Text2dAlignment {
        vertical: VerticalAlign::Center,
        horizontal: TextAlignment::Right,
    };

    /// A [`TextAlignment`] set to the bottom-left.
    pub const BOTTOM_LEFT: Self = Text2dAlignment {
        vertical: VerticalAlign::Bottom,
        horizontal: TextAlignment::Left,
    };

    /// A [`TextAlignment`] set to the bottom-center.
    pub const BOTTOM_CENTER: Self = Text2dAlignment {
        vertical: VerticalAlign::Bottom,
        horizontal: TextAlignment::Center,
    };

    /// A [`TextAlignment`] set to the bottom-right.
    pub const BOTTOM_RIGHT: Self = Text2dAlignment {
        vertical: VerticalAlign::Bottom,
        horizontal: TextAlignment::Right,
    };
}

impl Default for Text2dAlignment {
    fn default() -> Self {
        Text2dAlignment::TOP_LEFT
    }
}

/// Describes horizontal alignment preference for positioning & bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum TextAlignment {
    /// Leftmost character is immediately to the right of the render position.<br/>
    /// Bounds start from the render position and advance rightwards.
    Left,
    /// Leftmost & rightmost characters are equidistant to the render position.<br/>
    /// Bounds start from the render position and advance equally left & right.
    Center,
    /// Rightmost character is immediately to the left of the render position.<br/>
    /// Bounds start from the render position and advance leftwards.
    Right,
}

impl From<TextAlignment> for glyph_brush_layout::HorizontalAlign {
    fn from(val: TextAlignment) -> Self {
        match val {
            TextAlignment::Left => glyph_brush_layout::HorizontalAlign::Left,
            TextAlignment::Center => glyph_brush_layout::HorizontalAlign::Center,
            TextAlignment::Right => glyph_brush_layout::HorizontalAlign::Right,
        }
    }
}

/// Describes vertical alignment preference for positioning & bounds. Currently a placeholder
/// for future functionality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
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
