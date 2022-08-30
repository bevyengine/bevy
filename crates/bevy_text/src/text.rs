use core::panic;

use bevy_asset::{AssetPath, AssetServer, Assets};
use bevy_ecs::{
    prelude::{Component, EventWriter},
    query::Added,
    reflect::ReflectComponent,
    system::{Query, Res, Resource},
};
use bevy_reflect::{prelude::*, FromReflect};
use bevy_render::color::Color;
use bevy_utils::tracing::warn;
use bevy_window::RequestRedraw;
use serde::{Deserialize, Serialize};

use crate::{Font, FontRef};

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
    /// # use bevy_asset::Handle;
    /// # use bevy_render::color::Color;
    /// # use bevy_text::{Font, Text, TextAlignment, TextStyle, HorizontalAlign, VerticalAlign};
    /// #
    /// // Basic usage.
    /// let hello_world = Text::from_section(
    ///     // Accepts a String or any type that converts into a String, such as &str.
    ///     "hello world!",
    ///     TextStyle {
    ///         font: "path/to/font".into(),
    ///         font_size: 60.0,
    ///         color: Color::WHITE,
    ///     },
    /// );
    ///
    /// let hello_bevy = Text::from_section(
    ///     "hello bevy!",
    ///     TextStyle {
    ///         font: "path/to/font".into(),
    ///         font_size: 60.0,
    ///         color: Color::WHITE,
    ///     },
    /// ) // You can still add an alignment.
    /// .with_alignment(TextAlignment::CENTER);
    /// ```
    pub fn from_section(value: impl Into<String>, style: TextStyle) -> Self {
        Self {
            sections: vec![TextSection::new(value, style)],
            alignment: Default::default(),
        }
    }

    /// Constructs a [`Text`] from a list of sections.
    ///
    /// ```
    /// # use bevy_asset::Handle;
    /// # use bevy_render::color::Color;
    /// # use bevy_text::{Font, Text, TextStyle, TextSection};
    /// #
    /// let hello_world = Text::from_sections([
    ///     TextSection::new(
    ///         "Hello, ",
    ///         TextStyle {
    ///             font: "path/to/font".into(),
    ///             font_size: 60.0,
    ///             color: Color::BLUE,
    ///         },
    ///     ),
    ///     TextSection::new(
    ///         "World!",
    ///         TextStyle {
    ///             font: "path/to/font".into(),
    ///             font_size: 60.0,
    ///             color: Color::RED,
    ///         },
    ///     ),
    /// ]);
    /// ```
    pub fn from_sections(sections: impl IntoIterator<Item = TextSection>) -> Self {
        Self {
            sections: sections.into_iter().collect(),
            alignment: Default::default(),
        }
    }

    /// Returns this [`Text`] with a new [`TextAlignment`].
    pub const fn with_alignment(mut self, alignment: TextAlignment) -> Self {
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
pub struct TextAlignment {
    pub vertical: VerticalAlign,
    pub horizontal: HorizontalAlign,
}

impl TextAlignment {
    /// A [`TextAlignment`] set to the top-left.
    pub const TOP_LEFT: Self = TextAlignment {
        vertical: VerticalAlign::Top,
        horizontal: HorizontalAlign::Left,
    };

    /// A [`TextAlignment`] set to the top-center.
    pub const TOP_CENTER: Self = TextAlignment {
        vertical: VerticalAlign::Top,
        horizontal: HorizontalAlign::Center,
    };

    /// A [`TextAlignment`] set to the top-right.
    pub const TOP_RIGHT: Self = TextAlignment {
        vertical: VerticalAlign::Top,
        horizontal: HorizontalAlign::Right,
    };

    /// A [`TextAlignment`] set to center the center-left.
    pub const CENTER_LEFT: Self = TextAlignment {
        vertical: VerticalAlign::Center,
        horizontal: HorizontalAlign::Left,
    };

    /// A [`TextAlignment`] set to center on both axes.
    pub const CENTER: Self = TextAlignment {
        vertical: VerticalAlign::Center,
        horizontal: HorizontalAlign::Center,
    };

    /// A [`TextAlignment`] set to the center-right.
    pub const CENTER_RIGHT: Self = TextAlignment {
        vertical: VerticalAlign::Center,
        horizontal: HorizontalAlign::Right,
    };

    /// A [`TextAlignment`] set to the bottom-left.
    pub const BOTTOM_LEFT: Self = TextAlignment {
        vertical: VerticalAlign::Bottom,
        horizontal: HorizontalAlign::Left,
    };

    /// A [`TextAlignment`] set to the bottom-center.
    pub const BOTTOM_CENTER: Self = TextAlignment {
        vertical: VerticalAlign::Bottom,
        horizontal: HorizontalAlign::Center,
    };

    /// A [`TextAlignment`] set to the bottom-right.
    pub const BOTTOM_RIGHT: Self = TextAlignment {
        vertical: VerticalAlign::Bottom,
        horizontal: HorizontalAlign::Right,
    };
}

impl Default for TextAlignment {
    fn default() -> Self {
        TextAlignment::TOP_LEFT
    }
}

/// Describes horizontal alignment preference for positioning & bounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize)]
pub enum HorizontalAlign {
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

/// Used to configure the default font used by [`TextStyle`] when the font is set to [`FontRef::Default`]
#[derive(Default, Resource)]
pub struct DefaultFont {
    pub path: Option<AssetPath<'static>>,
}

/// Loads fonts defined by a [`FontRef`]
/// If the font has already been loaded then it simply converts the [`FontRef::Path`] to a [`FontRef::Handle`]
/// Otherwise it tries to load the font with the [`AssetServer`]
pub fn load_font(
    mut query: Query<&mut Text, Added<Text>>,
    asset_server: Res<AssetServer>,
    fonts: Res<Assets<Font>>,
    default_font: Res<DefaultFont>,
    mut event: EventWriter<RequestRedraw>,
) {
    let mut needs_redraw = false;
    for mut text in &mut query {
        for mut section in &mut text.sections {
            let path = match &section.style.font {
                FontRef::Default => {
                    if let Some(path) = default_font.path.as_ref() {
                        path
                    } else if let Some((handle, _)) = fonts.iter().next() {
                        warn!("Default font not set. Using first available font");
                        section.style.font = FontRef::Handle(fonts.get_handle(handle));
                        continue;
                    } else {
                        panic!("No default font found");
                    }
                }
                FontRef::Path(path) => path,
                FontRef::Handle(_) => continue,
            };
            let handle = match asset_server.get_load_state(path.clone()) {
                bevy_asset::LoadState::NotLoaded | bevy_asset::LoadState::Unloaded => {
                    asset_server.load(path.clone())
                }
                bevy_asset::LoadState::Loading | bevy_asset::LoadState::Loaded => {
                    fonts.get_handle(path.clone())
                }
                bevy_asset::LoadState::Failed => panic!("Failed to load font {:?}", path),
            };
            section.style.font = FontRef::Handle(handle);
            needs_redraw = true;
        }
    }
    if needs_redraw {
        // This is to avoid an issue in low power mode where the fonts were not loaded in time
        // and there would be no text until the next redraw.
        event.send(RequestRedraw);
    }
}
