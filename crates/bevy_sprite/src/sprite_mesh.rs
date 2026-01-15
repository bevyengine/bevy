use bevy_asset::{Assets, Handle};
use bevy_camera::visibility::{Visibility, VisibilityClass};
use bevy_color::Color;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_image::{Image, TextureAtlas, TextureAtlasLayout};
use bevy_math::{Rect, UVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, PartialReflect, Reflect};
use bevy_transform::components::Transform;

use crate::{Anchor, SpriteImageMode};

/// This is a carbon copy of [`Sprite`](crate::sprite::Sprite) that uses the
/// Mesh backend instead of the Sprite backend.
///
/// The only API difference is the added [`alpha mode`](SpriteMesh::alpha_mode).
#[derive(Component, Debug, Default, Clone, Reflect, PartialEq)]
#[require(Transform, Visibility, VisibilityClass, Anchor)]
#[reflect(Component, Default, Debug, Clone)]
pub struct SpriteMesh {
    /// The image used to render the sprite
    pub image: Handle<Image>,
    /// The (optional) texture atlas used to render the sprite
    pub texture_atlas: Option<TextureAtlas>,
    /// The sprite's color tint
    pub color: Color,
    /// Flip the sprite along the `X` axis
    pub flip_x: bool,
    /// Flip the sprite along the `Y` axis
    pub flip_y: bool,
    /// An optional custom size for the sprite that will be used when rendering, instead of the size
    /// of the sprite's image
    pub custom_size: Option<Vec2>,
    /// An optional rectangle representing the region of the sprite's image to render, instead of rendering
    /// the full image. This is an easy one-off alternative to using a [`TextureAtlas`].
    ///
    /// When used with a [`TextureAtlas`], the rect
    /// is offset by the atlas's minimal (top-left) corner position.
    pub rect: Option<Rect>,
    /// How the sprite's image will be scaled.
    pub image_mode: SpriteImageMode,
    /// The sprite's alpha mode, defaulting to `Mask(0.5)`.
    /// If you wish to render a sprite with translucent pixels,
    /// set it to `Blend` instead (significantly worse for performance).
    pub alpha_mode: SpriteAlphaMode,
}

impl core::hash::Hash for SpriteMesh {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.image.hash(state);
        self.texture_atlas.hash(state);
        self.color.reflect_hash().hash(state);
        self.custom_size.reflect_hash().hash(state);
        self.flip_x.hash(state);
        self.flip_y.hash(state);
    }
}

impl Eq for SpriteMesh {}

// NOTE: The SpriteImageMode, SpriteScalingMode and Anchor are imported from the sprite module.

impl SpriteMesh {
    /// Create a Sprite with a custom size
    pub fn sized(custom_size: Vec2) -> Self {
        SpriteMesh {
            custom_size: Some(custom_size),
            ..Default::default()
        }
    }

    /// Create a sprite from an image
    pub fn from_image(image: Handle<Image>) -> Self {
        Self {
            image,
            ..Default::default()
        }
    }

    /// Create a sprite from an image, with an associated texture atlas
    pub fn from_atlas_image(image: Handle<Image>, atlas: TextureAtlas) -> Self {
        Self {
            image,
            texture_atlas: Some(atlas),
            ..Default::default()
        }
    }

    /// Create a sprite from a solid color
    pub fn from_color(color: impl Into<Color>, size: Vec2) -> Self {
        Self {
            color: color.into(),
            custom_size: Some(size),
            ..Default::default()
        }
    }

    /// Computes the pixel point where `point_relative_to_sprite` is sampled
    /// from in this sprite. `point_relative_to_sprite` must be in the sprite's
    /// local frame. Returns an Ok if the point is inside the bounds of the
    /// sprite (not just the image), and returns an Err otherwise.
    pub fn compute_pixel_space_point(
        &self,
        point_relative_to_sprite: Vec2,
        anchor: Anchor,
        images: &Assets<Image>,
        texture_atlases: &Assets<TextureAtlasLayout>,
    ) -> Result<Vec2, Vec2> {
        let image_size = images
            .get(&self.image)
            .map(Image::size)
            .unwrap_or(UVec2::ONE);

        let atlas_rect = self
            .texture_atlas
            .as_ref()
            .and_then(|s| s.texture_rect(texture_atlases))
            .map(|r| r.as_rect());
        let texture_rect = match (atlas_rect, self.rect) {
            (None, None) => Rect::new(0.0, 0.0, image_size.x as f32, image_size.y as f32),
            (None, Some(sprite_rect)) => sprite_rect,
            (Some(atlas_rect), None) => atlas_rect,
            (Some(atlas_rect), Some(mut sprite_rect)) => {
                // Make the sprite rect relative to the atlas rect.
                sprite_rect.min += atlas_rect.min;
                sprite_rect.max += atlas_rect.min;
                sprite_rect
            }
        };

        let sprite_size = self.custom_size.unwrap_or_else(|| texture_rect.size());
        let sprite_center = -anchor.as_vec() * sprite_size;

        let mut point_relative_to_sprite_center = point_relative_to_sprite - sprite_center;

        if self.flip_x {
            point_relative_to_sprite_center.x *= -1.0;
        }
        // Texture coordinates start at the top left, whereas world coordinates start at the bottom
        // left. So flip by default, and then don't flip if `flip_y` is set.
        if !self.flip_y {
            point_relative_to_sprite_center.y *= -1.0;
        }

        if sprite_size.x == 0.0 || sprite_size.y == 0.0 {
            return Err(point_relative_to_sprite_center);
        }

        let sprite_to_texture_ratio = {
            let texture_size = texture_rect.size();
            Vec2::new(
                texture_size.x / sprite_size.x,
                texture_size.y / sprite_size.y,
            )
        };

        let point_relative_to_texture =
            point_relative_to_sprite_center * sprite_to_texture_ratio + texture_rect.center();

        // TODO: Support `SpriteImageMode`.

        if texture_rect.contains(point_relative_to_texture) {
            Ok(point_relative_to_texture)
        } else {
            Err(point_relative_to_texture)
        }
    }
}

// This is different from AlphaMode2d in bevy_sprite_render because that crate depends on this one,
// so using it would've been caused a circular dependency. An option would be to move the Enum here
// but it uses a bevy_render dependency in its documentation, and I wanted to avoid bringing that
// dependency to this crate.

// NOTE: If this is ever replaced by AlphaMode2d, make a custom Default impl for Sprite,
// because AlphaMode2d defaults to Opaque, but the sprite's alpha mode is most commonly Mask(0.5)

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
