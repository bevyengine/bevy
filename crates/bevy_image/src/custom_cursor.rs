use bevy_app::{App, Plugin};
use bevy_asset::{Assets, Handle};
use bevy_ecs::component::Component;
use bevy_ecs::prelude::ReflectComponent;
use bevy_image::{Image, TextureAtlas, TextureAtlasLayout, TextureAtlasPlugin};
use bevy_math::{ops, Rect, URect, UVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

/// A custom cursor created from an image.
#[derive(Debug, Clone, Default, Reflect, PartialEq, Eq, Hash)]
#[reflect(Debug, Default, Hash, PartialEq, Clone)]
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
