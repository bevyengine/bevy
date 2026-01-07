use bevy_asset::{AsAssetId, AssetId, Assets, Handle};
use bevy_camera::visibility::{self, Visibility, VisibilityClass};
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_image::{Image, TextureAtlas, TextureAtlasLayout};
use bevy_math::{Rect, UVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::components::Transform;

use crate::{Anchor, TextureSlicer};

/// This is a carbon copy of [`Sprite`](crate::sprite::Sprite) that uses the
/// Mesh backend instead of the Sprite backend.
#[derive(Component, Debug, Default, Clone, Reflect)]
#[require(Transform, Visibility, VisibilityClass, Anchor)]
#[reflect(Component, Default, Debug, Clone)]
pub struct SpriteMesh {
    /// The image used to render the sprite
    pub image: Handle<Image>,
    /// The (optional) texture atlas used to render the sprite
    pub texture_atlas: Option<TextureAtlas>,
    /// The sprite's color tint
    pub color: Color,

    /// The sprite's alpha mode, defaulting to `Mask(0.5)`.
    /// If you wish to render a sprite with transparent pixels,
    /// set it to `Blend` instead (significantly worse for performance).
    pub alpha_mode: SpriteAlphaMode,
}

// This is different from AlphaMode2d in bevy_sprite_render because that crate depends on this one,
// so using it would've been caused a circular dependency. An option would be to move the Enum here
// but it uses a bevy_render dependency in its documentation, and I wanted to avoid bringing that
// dependency to this crate.

#[derive(Debug, Reflect, Copy, Clone, PartialEq)]
#[reflect(Default, Debug, Clone)]
pub enum SpriteAlphaMode {
    /// Base color alpha values are overridden to be fully opaque (1.0).
    Opaque,
    /// Reduce transparency to fully opaque or fully transparent
    /// based on a threshold.
    ///
    /// Compares the base color alpha value to the specified threshold.
    /// If the value is below the threshold,
    /// considers the color to be fully transparent (alpha is set to 0.0).
    /// If it is equal to or above the threshold,
    /// considers the color to be fully opaque (alpha is set to 1.0).
    Mask(f32),
    /// The base color alpha value defines the opacity of the color.
    /// Standard alpha-blending is used to blend the fragment's color
    /// with the color behind it.
    Blend,
}

impl Default for SpriteAlphaMode {
    fn default() -> Self {
        Self::Mask(0.5)
    }
}
