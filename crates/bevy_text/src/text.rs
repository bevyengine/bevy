use bevy_asset::Handle;
use bevy_color::Color;
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_reflect::prelude::*;
use bevy_utils::default;
use serde::{Deserialize, Serialize};

//use crate DEFAULT_FONT_HANDLE;
use crate::Font;
// TODO: reexport cosmic_text and these types in the prelude
pub use cosmic_text::{
    FamilyOwned as FontFamily, Stretch as FontStretch, Style as FontStyle, Weight as FontWeight,
};

/// A component that is the entry point for rendering text.
///
/// It contains all of the text value and styling information.
#[derive(Component, Debug, Clone, Default, Reflect)]
#[reflect(Component, Default)]
pub struct Text {
    /// The text's sections
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
    ///         font: font_handle.clone().into(),
    ///         font_size: 60.0,
    ///         color: Color::WHITE,
    ///     },
    /// );
    ///
    /// let hello_bevy = Text::from_section(
    ///     "hello world\nand bevy!",
    ///     TextStyle {
    ///         font: font_handle.into(),
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
    ///             font: font_handle.clone().into(),
    ///             font_size: 60.0,
    ///             color: BLUE.into(),
    ///         },
    ///     ),
    ///     TextSection::new(
    ///         "World!",
    ///         TextStyle {
    ///             font: font_handle.into(),
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

/// Contains the value of the text in a section and how it should be styled.
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

#[derive(Clone, Debug, Reflect)]
pub struct TextStyle {
    pub font: FontRef,
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

/// Identifies a font to use, which is either stored as an [`Asset`](bevy_asset::Asset) or loaded directly from the user's system.
#[derive(Clone, Debug, Reflect)]
pub enum FontRef {
    /// A reference to a font loaded as a bevy asset.
    Asset(Handle<Font>),
    /// A reference to a font queried by font family and attributes.
    /// This is useful for example for fonts that are not loaded as a bevy asset,
    /// such as system fonts.
    // TODO: Support Reflect?
    Query(#[reflect(ignore)] FontQuery),
}

impl Default for FontRef {
    fn default() -> Self {
        Self::Asset(Default::default())
    }
}

impl From<Handle<Font>> for FontRef {
    fn from(handle: Handle<Font>) -> Self {
        Self::Asset(handle)
    }
}

/// Queries for a font from those already loaded.
///
/// ```
/// # use bevy_text::{FontQuery, FontWeight, TextStyle};
///
/// let fira_sans_bold = FontQuery::family("FiraSans").weight(FontWeight::BOLD);
///
/// let text_style = TextStyle {
///     font: fira_sans_bold.into(),
///     ..Default::default()
/// };
/// ```
#[derive(Clone, Debug)]
pub struct FontQuery {
    /// The font family. See [`cosmic_text::fontdb::Family`] for details.
    pub family: FontFamily,
    /// The stretch (or width) of the font face in this family, e.g. condensed.
    /// See [`cosmic_text::fontdb::Stretch`] for details.
    pub stretch: FontStretch,
    /// The style of the font face in this family, e.g. italic.
    /// See [`cosmic_text::fontdb::Style`] for details.
    pub style: FontStyle,
    /// The weight of the font face in this family, e.g. bold.
    /// See [`cosmic_text::fontdb::Weight`] for details.
    pub weight: FontWeight,
}

impl FontQuery {
    pub fn sans_serif() -> Self {
        Self {
            family: FontFamily::SansSerif,
            stretch: Default::default(),
            style: Default::default(),
            weight: Default::default(),
        }
    }

    pub fn serif() -> Self {
        Self {
            family: FontFamily::Serif,
            stretch: Default::default(),
            style: Default::default(),
            weight: Default::default(),
        }
    }

    pub fn fantasy() -> Self {
        Self {
            family: FontFamily::Fantasy,
            stretch: Default::default(),
            style: Default::default(),
            weight: Default::default(),
        }
    }

    pub fn cursive() -> Self {
        Self {
            family: FontFamily::Cursive,
            stretch: Default::default(),
            style: Default::default(),
            weight: Default::default(),
        }
    }

    pub fn monospace() -> Self {
        Self {
            family: FontFamily::Monospace,
            stretch: Default::default(),
            style: Default::default(),
            weight: Default::default(),
        }
    }

    pub fn family<S: AsRef<str>>(name: S) -> Self {
        Self {
            family: FontFamily::Name(name.as_ref().to_string()),
            stretch: Default::default(),
            style: Default::default(),
            weight: Default::default(),
        }
    }

    pub fn stretch(self, stretch: FontStretch) -> Self {
        Self { stretch, ..self }
    }

    pub fn style(self, style: FontStyle) -> Self {
        Self { style, ..self }
    }

    pub fn weight(self, weight: FontWeight) -> Self {
        Self { weight, ..self }
    }
}

impl Default for FontQuery {
    fn default() -> Self {
        Self::sans_serif()
    }
}

impl From<FontQuery> for FontRef {
    fn from(query: FontQuery) -> Self {
        Self::Query(query)
    }
}
