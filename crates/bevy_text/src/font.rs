use alloc::sync::Arc;
use bevy_asset::{Asset, Handle};
use bevy_ecs::component::Component;
use bevy_ecs::{prelude::*, reflect::ReflectComponent};
use bevy_reflect::prelude::*;
use bevy_reflect::TypePath;
use bevy_utils::default;
use serde::{Deserialize, Serialize};

/// An [`Asset`] that contains the data for a loaded font, if loaded as an asset.
///
/// Loaded by [`FontLoader`](crate::FontLoader).
///
/// # A note on fonts
///
/// `Font` may differ from the everyday notion of what a "font" is.
/// A font *face* (e.g. Fira Sans Semibold Italic) is part of a font *family* (e.g. Fira Sans),
/// and is distinguished from other font faces in the same family
/// by its style (e.g. italic), its weight (e.g. bold) and its stretch (e.g. condensed).
///
/// Bevy currently loads a single font face as a single `Font` asset.
#[derive(Debug, TypePath, Clone, Asset)]
pub struct FontFace {
    /// Content of a font file as bytes
    pub data: Arc<Vec<u8>>,
}

impl FontFace {
    /// Creates a [`Font`] from bytes
    pub fn try_from_bytes(
        font_data: Vec<u8>,
    ) -> Result<Self, cosmic_text::ttf_parser::FaceParsingError> {
        use cosmic_text::ttf_parser;
        ttf_parser::Face::parse(&font_data, 0)?;
        Ok(Self {
            data: Arc::new(font_data),
        })
    }
}

/// Determines the style of a text span within a [`ComputedTextBlock`](`crate::ComputedTextBlock`), specifically
/// the font face, the font size, and antialiasing method.
#[derive(Component, Clone, Debug, Reflect, PartialEq)]
#[reflect(Component, Default, Debug, Clone)]
pub struct Font {
    /// The specific font face to use, as a `Handle` to a [`Font`] asset.
    ///
    /// If the `font` is not specified, then
    /// * if `default_font` feature is enabled (enabled by default in `bevy` crate),
    ///   `FiraMono-subset.ttf` compiled into the library is used.
    /// * otherwise no text will be rendered, unless a custom font is loaded into the default font
    ///   handle.
    pub face: Handle<FontFace>,
    /// The vertical height of rasterized glyphs in the font atlas in pixels.
    ///
    /// This is multiplied by the window scale factor and `UiScale`, but not the text entity
    /// transform or camera projection.
    ///
    /// A new font atlas is generated for every combination of font handle and scaled font size
    /// which can have a strong performance impact.
    pub size: f32,
    /// The antialiasing method to use when rendering text.
    pub smoothing: FontSmoothing,
}

impl Font {
    /// Returns a new [`Font`] with the specified font face and size.
    pub fn new(font_face: Handle<FontFace>, font_size: f32) -> Font {
        Self {
            face: font_face,
            size: font_size,
            ..default()
        }
    }

    /// Returns a new [`Font`] with the specified font size.
    pub fn from_font_size(font_size: f32) -> Self {
        Self::default().with_font_size(font_size)
    }

    /// Returns this [`Font`] with the specified font face handle.
    pub fn with_font(mut self, font_face: Handle<FontFace>) -> Self {
        self.face = font_face;
        self
    }

    /// Returns this [`Font`] with the specified font size.
    pub const fn with_font_size(mut self, font_size: f32) -> Self {
        self.size = font_size;
        self
    }

    /// Returns this [`Font`] with the specified [`FontSmoothing`].
    pub const fn with_font_smoothing(mut self, font_smoothing: FontSmoothing) -> Self {
        self.smoothing = font_smoothing;
        self
    }
}

impl From<Handle<FontFace>> for Font {
    fn from(font_face: Handle<FontFace>) -> Self {
        Self {
            face: font_face,
            ..default()
        }
    }
}

impl Default for Font {
    fn default() -> Self {
        Self {
            face: Default::default(),
            size: 20.0,
            smoothing: Default::default(),
        }
    }
}

/// Determines which antialiasing method to use when rendering text. By default, text is
/// rendered with grayscale antialiasing, but this can be changed to achieve a pixelated look.
///
/// **Note:** Subpixel antialiasing is not currently supported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Reflect, Serialize, Deserialize)]
#[reflect(Serialize, Deserialize, Clone, PartialEq, Hash, Default)]
#[doc(alias = "antialiasing")]
#[doc(alias = "pixelated")]
pub enum FontSmoothing {
    /// No antialiasing. Useful for when you want to render text with a pixel art aesthetic.
    ///
    /// Combine this with `UiAntiAlias::Off` and `Msaa::Off` on your 2D camera for a fully pixelated look.
    ///
    /// **Note:** Due to limitations of the underlying text rendering library,
    /// this may require specially-crafted pixel fonts to look good, especially at small sizes.
    None,
    /// The default grayscale antialiasing. Produces text that looks smooth,
    /// even at small font sizes and low resolutions with modern vector fonts.
    #[default]
    AntiAliased,
    // TODO: Add subpixel antialias support
    // SubpixelAntiAliased,
}
