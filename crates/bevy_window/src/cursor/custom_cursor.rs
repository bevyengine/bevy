use crate::cursor::CursorIcon;
use alloc::string::String;
use bevy_asset::Handle;
use bevy_image::{Image, TextureAtlas};
use bevy_math::URect;

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// A custom cursor created from an image.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Default, Hash, PartialEq, Clone)
)]
pub struct CustomCursorImage {
    /// Handle to the image to use as the cursor. The image must be in 8 bit int
    /// or 32 bit float rgba. PNG images work well for this.
    pub handle: Handle<Image>,
    /// An optional texture atlas used to render the image.
    pub texture_atlas: Option<TextureAtlas>,
    /// Whether the image should be flipped along its x-axis.
    ///
    /// If true, the cursor's `hotspot` automatically flips along with the
    /// image.
    pub flip_x: bool,
    /// Whether the image should be flipped along its y-axis.
    ///
    /// If true, the cursor's `hotspot` automatically flips along with the
    /// image.
    pub flip_y: bool,
    /// An optional rectangle representing the region of the image to render,
    /// instead of rendering the full image. This is an easy one-off alternative
    /// to using a [`TextureAtlas`].
    ///
    /// When used with a [`TextureAtlas`], the rect is offset by the atlas's
    /// minimal (top-left) corner position.
    pub rect: Option<URect>,
    /// X and Y coordinates of the hotspot in pixels. The hotspot must be within
    /// the image bounds.
    ///
    /// If you are flipping the image using `flip_x` or `flip_y`, you don't need
    /// to adjust this field to account for the flip because it is adjusted
    /// automatically.
    pub hotspot: (u16, u16),
}

/// A custom cursor created from a URL. Note that this currently only works on the web.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Default, Hash, PartialEq, Clone)
)]
pub struct CustomCursorUrl {
    /// Web URL to an image to use as the cursor. PNGs are preferred. Cursor
    /// creation can fail if the image is invalid or not reachable.
    pub url: String,
    /// X and Y coordinates of the hotspot in pixels. The hotspot must be within
    /// the image bounds.
    pub hotspot: (u16, u16),
}

/// Custom cursor image data.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Clone, PartialEq, Hash)
)]
pub enum CustomCursor {
    /// Use an image as the cursor.
    Image(CustomCursorImage),
    /// Use a URL to an image as the cursor. Note that this currently only works on the web.
    Url(CustomCursorUrl),
}

impl From<CustomCursor> for CursorIcon {
    fn from(cursor: CustomCursor) -> Self {
        CursorIcon::Custom(cursor)
    }
}
