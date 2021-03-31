use bevy_asset::{Handle, HandleUntyped};
use bevy_math::Size;
use bevy_reflect::TypeUuid;
use bevy_render::color::Color;
use glyph_brush_layout::{HorizontalAlign, VerticalAlign};

use crate::Font;

#[derive(Debug, Default, Clone)]
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

#[cfg(feature = "bevy_default_assets")]
pub const DEFAULT_FONT_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(crate::font::Font::TYPE_UUID, 12210261929130131812);

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            #[cfg(feature = "bevy_default_assets")]
            font: DEFAULT_FONT_HANDLE.typed(),
            #[cfg(not(feature = "bevy_default_assets"))]
            font: Default::default(),
            font_size: 12.0,
            color: Color::WHITE,
        }
    }
}

#[derive(Default, Copy, Clone, Debug)]
pub struct Text2dSize {
    pub size: Size,
}

#[cfg(feature = "bevy_default_assets")]
pub(crate) mod default_font {
    use crate::{Font, DEFAULT_FONT_HANDLE};
    use ab_glyph::{FontArc, FontRef};
    use bevy_app::AppBuilder;
    use bevy_asset::Assets;

    pub(crate) fn load_default_font(app: &mut AppBuilder) {
        let world = app.world_mut();
        let world_cell = world.cell();
        let mut fonts = world_cell.get_resource_mut::<Assets<Font>>().unwrap();
        let font_bytes = include_bytes!("assets/fonts/FiraSans-Bold.ttf");
        let font = FontRef::try_from_slice(font_bytes).unwrap();
        let font = FontArc::new(font);
        let font = Font { font };
        fonts.set_untracked(DEFAULT_FONT_HANDLE, font);
    }
}
