use bevy_asset::Handle;
use bevy_ecs::{prelude::Component, reflect::ReflectComponent};
use bevy_reflect::{FromReflect, Reflect, ReflectDeserialize};
use bevy_render::color::Color;
use serde::{Deserialize, Serialize};
use unicode_bidi::BidiInfo;

use crate::Font;

#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component)]
pub struct Text {
    pub sections: Vec<TextSection>,
    pub alignment: TextAlignment,
    bidi_corrected: Vec<TextSection>,
}

impl Text {
    /// Constructs a [`Text`] with (initially) one section.
    ///
    /// ```
    /// # use bevy_asset::{AssetServer, Handle};
    /// # use bevy_render::color::Color;
    /// # use bevy_text::{Font, Text, TextAlignment, TextStyle, HorizontalAlign, VerticalAlign};
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
            bidi_corrected: Default::default(),
            alignment,
        }
    }

    /// Add more sections to an existing [`Text`].
    ///
    /// ```
    /// # use bevy_asset::{AssetServer, Handle};
    /// # use bevy_render::color::Color;
    /// # use bevy_text::{Font, Text, TextAlignment, TextStyle, HorizontalAlign, VerticalAlign};
    /// #
    /// # let font_handle: Handle<Font> = Default::default();
    /// #
    /// let hello_world = Text::with_section(
    ///     "hello".to_string(),
    ///     TextStyle {
    ///         font: font_handle.clone(),
    ///         font_size: 60.0,
    ///         color: Color::WHITE,
    ///     },
    ///     TextAlignment {
    ///         vertical: VerticalAlign::Center,
    ///         horizontal: HorizontalAlign::Center,
    ///     },
    /// ).add_section(    
    ///     " world!".to_string(),
    ///     TextStyle {
    ///         font: font_handle.clone(),
    ///         font_size: 100.0,
    ///         color: Color::WHITE,
    ///     },
    ///);
    /// ```
    pub fn add_section<S: Into<String>>(mut self, value: S, style: TextStyle) -> Self {
        self.sections.push(TextSection {
            value: value.into(),
            style,
        });
        self
    }

    /// This will run the unicode bidirectional algorithm over all sections and create a bidi corrected copy for later use
    pub fn bidi_correct(&mut self) {
        self.bidi_corrected.clear();
        for section in &self.sections {
            let bidi_info = BidiInfo::new(&section.value, None);
            for para in &bidi_info.paragraphs {
                let line = para.range.clone();
                let display = bidi_info.reorder_line(para, line);
                let section = TextSection {
                    value: display.into_owned(),
                    style: section.style.clone(),
                };
                self.bidi_corrected.push(section);
            }
        }
    }

    /// Get the bidi corrected sections - needs to run [`Self::bidi_correct`] first to create bidi corrected sections!
    pub fn bidi_corrected_sections(&self) -> &Vec<TextSection> {
        &self.bidi_corrected
    }
}

#[derive(Debug, Default, Clone, FromReflect, Reflect)]
pub struct TextSection {
    pub value: String,
    pub style: TextStyle,
}

#[derive(Debug, Clone, Copy, Reflect)]
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
