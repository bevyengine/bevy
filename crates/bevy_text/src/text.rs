use bevy_asset::Handle;
use bevy_color::Color;
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_reflect::prelude::*;
use bevy_utils::default;
use serde::{Deserialize, Serialize};

use crate::Font;

#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default)]
pub struct Text {
    pub sections: Vec<TextSection>,
    /// The text's internal alignment.
    /// Should not affect its position within a container.
    pub justify: JustifyText,
    /// How the text should linebreak when running out of the bounds determined by `max_size`
    pub linebreak_behavior: BreakLineOn,
}

impl Text {
    /// Constructs a [`Text`] with a single section.
    ///
    /// ```
    /// # use bevy_asset::Handle;
    /// # use bevy_color::Color;
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
    ///     "hello world\nand bevy!",
    ///     TextStyle {
    ///         font: font_handle,
    ///         font_size: 60.0,
    ///         color: Color::WHITE,
    ///     },
    /// ) // You can still add text justifaction.
    /// .with_justify(JustifyText::Center);
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
    /// # use bevy_color::Color;
    /// # use bevy_color::palettes::basic::{RED, BLUE};
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
    ///             color: BLUE.into(),
    ///         },
    ///     ),
    ///     TextSection::new(
    ///         "World!",
    ///         TextStyle {
    ///             font: font_handle,
    ///             font_size: 60.0,
    ///             color: RED.into(),
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

    /// Returns this [`Text`] with soft wrapping disabled.
    /// Hard wrapping, where text contains an explicit linebreak such as the escape sequence `\n`, will still occur.
    pub const fn with_no_wrap(mut self) -> Self {
        self.linebreak_behavior = BreakLineOn::NoWrap;
        self
    }
}

#[derive(Debug, Default, Clone, Reflect)]
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

#[cfg(feature = "default_font")]
impl From<&str> for TextSection {
    fn from(value: &str) -> Self {
        Self {
            value: value.into(),
            ..default()
        }
    }
}

#[cfg(feature = "default_font")]
impl From<String> for TextSection {
    fn from(value: String) -> Self {
        Self {
            value,
            ..Default::default()
        }
    }
}

/// Describes the horizontal alignment of multiple lines of text relative to each other.
/// This only affects the internal positioning of the lines of text within a text entity and
/// does not affect the text entity's position.
///
/// _Has no affect on a single line text entity._
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum JustifyText {
    /// Leftmost character is immediately to the right of the render position.
    /// Bounds start from the render position and advance rightwards.
    #[default]
    Left,
    /// Leftmost & rightmost characters are equidistant to the render position.
    /// Bounds start from the render position and advance equally left & right.
    Center,
    /// Rightmost character is immediately to the left of the render position.
    /// Bounds start from the render position and advance leftwards.
    Right,
}

impl From<JustifyText> for glyph_brush_layout::HorizontalAlign {
    fn from(val: JustifyText) -> Self {
        match val {
            JustifyText::Left => glyph_brush_layout::HorizontalAlign::Left,
            JustifyText::Center => glyph_brush_layout::HorizontalAlign::Center,
            JustifyText::Right => glyph_brush_layout::HorizontalAlign::Right,
        }
    }
}

#[derive(Clone, Debug, Reflect)]
pub struct TextStyle {
    /// If this is not specified, then
    /// * if `default_font` feature is enabled (enabled by default in `bevy` crate),
    ///  `FiraMono-subset.ttf` compiled into the library is used.
    /// * otherwise no text will be rendered.
    pub font: Handle<Font>,
    /// The vertical height of rasterized glyphs in the font atlas in pixels.
    ///
    /// This is multiplied by the window scale factor and `UiScale`, but not the text entity
    /// transform or camera projection.
    ///
    /// A new font atlas is generated for every combination of font handle and scaled font size
    /// which can have a strong performance impact.
    pub font_size: f32,
    pub color: Color,
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            font: Default::default(),
            font_size: 24.0,
            color: Color::WHITE,
        }
    }
}

/// Determines how lines will be broken when preventing text from running out of bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum BreakLineOn {
    /// Uses the [Unicode Line Breaking Algorithm](https://www.unicode.org/reports/tr14/).
    /// Lines will be broken up at the nearest suitable word boundary, usually a space.
    /// This behavior suits most cases, as it keeps words intact across linebreaks.
    #[default]
    WordBoundary,
    /// Lines will be broken without discrimination on any character that would leave bounds.
    /// This is closer to the behavior one might expect from text in a terminal.
    /// However it may lead to words being broken up across linebreaks.
    AnyCharacter,
    /// No soft wrapping, where text is automatically broken up into separate lines when it overflows a boundary, will ever occur.
    /// Hard wrapping, where text contains an explicit linebreak such as the escape sequence `\n`, is still enabled.
    NoWrap,
}

impl From<BreakLineOn> for glyph_brush_layout::BuiltInLineBreaker {
    fn from(val: BreakLineOn) -> Self {
        match val {
            // If `NoWrap` is set the choice of `BuiltInLineBreaker` doesn't matter as the text is given unbounded width and soft wrapping will never occur.
            // But `NoWrap` does not disable hard breaks where a [`Text`] contains a newline character.
            BreakLineOn::WordBoundary | BreakLineOn::NoWrap => {
                glyph_brush_layout::BuiltInLineBreaker::UnicodeLineBreaker
            }
            BreakLineOn::AnyCharacter => glyph_brush_layout::BuiltInLineBreaker::AnyCharLineBreaker,
        }
    }
}
