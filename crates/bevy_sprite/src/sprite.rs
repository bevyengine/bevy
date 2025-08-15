use bevy_asset::{Assets, Handle};
use bevy_camera::visibility::{self, Visibility, VisibilityClass};
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_image::{Image, TextureAtlas, TextureAtlasLayout};
use bevy_math::{Rect, UVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_transform::components::Transform;

use crate::TextureSlicer;

/// Describes a sprite to be rendered to a 2D camera
#[derive(Component, Debug, Default, Clone, Reflect)]
#[require(Transform, Visibility, VisibilityClass, Anchor)]
#[reflect(Component, Default, Debug, Clone)]
#[component(on_add = visibility::add_visibility_class::<Sprite>)]
pub struct Sprite {
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
}

impl Sprite {
    /// Create a Sprite with a custom size
    pub fn sized(custom_size: Vec2) -> Self {
        Sprite {
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

impl From<Handle<Image>> for Sprite {
    fn from(image: Handle<Image>) -> Self {
        Self::from_image(image)
    }
}

/// Controls how the image is altered when scaled.
#[derive(Default, Debug, Clone, Reflect, PartialEq)]
#[reflect(Debug, Default, Clone)]
pub enum SpriteImageMode {
    /// The sprite will take on the size of the image by default, and will be stretched or shrunk if [`Sprite::custom_size`] is set.
    #[default]
    Auto,
    /// The texture will be scaled to fit the rect bounds defined in [`Sprite::custom_size`].
    /// Otherwise no scaling will be applied.
    Scale(ScalingMode),
    /// The texture will be cut in 9 slices, keeping the texture in proportions on resize
    Sliced(TextureSlicer),
    /// The texture will be repeated if stretched beyond `stretched_value`
    Tiled {
        /// Should the image repeat horizontally
        tile_x: bool,
        /// Should the image repeat vertically
        tile_y: bool,
        /// The texture will repeat when the ratio between the *drawing dimensions* of texture and the
        /// *original texture size* are above this value.
        stretch_value: f32,
    },
}

impl SpriteImageMode {
    /// Returns true if this mode uses slices internally ([`SpriteImageMode::Sliced`] or [`SpriteImageMode::Tiled`])
    #[inline]
    pub fn uses_slices(&self) -> bool {
        matches!(
            self,
            SpriteImageMode::Sliced(..) | SpriteImageMode::Tiled { .. }
        )
    }

    /// Returns [`ScalingMode`] if scale is presented or [`Option::None`] otherwise.
    #[inline]
    #[must_use]
    pub const fn scale(&self) -> Option<ScalingMode> {
        if let SpriteImageMode::Scale(scale) = self {
            Some(*scale)
        } else {
            None
        }
    }
}

/// Represents various modes for proportional scaling of a texture.
///
/// Can be used in [`SpriteImageMode::Scale`].
#[derive(Debug, Clone, Copy, PartialEq, Default, Reflect)]
#[reflect(Debug, Default, Clone)]
pub enum ScalingMode {
    /// Scale the texture uniformly (maintain the texture's aspect ratio)
    /// so that both dimensions (width and height) of the texture will be equal
    /// to or larger than the corresponding dimension of the target rectangle.
    /// Fill sprite with a centered texture.
    #[default]
    FillCenter,
    /// Scales the texture to fill the target rectangle while maintaining its aspect ratio.
    /// One dimension of the texture will match the rectangle's size,
    /// while the other dimension may exceed it.
    /// The exceeding portion is aligned to the start:
    /// * Horizontal overflow is left-aligned if the width exceeds the rectangle.
    /// * Vertical overflow is top-aligned if the height exceeds the rectangle.
    FillStart,
    /// Scales the texture to fill the target rectangle while maintaining its aspect ratio.
    /// One dimension of the texture will match the rectangle's size,
    /// while the other dimension may exceed it.
    /// The exceeding portion is aligned to the end:
    /// * Horizontal overflow is right-aligned if the width exceeds the rectangle.
    /// * Vertical overflow is bottom-aligned if the height exceeds the rectangle.
    FillEnd,
    /// Scaling the texture will maintain the original aspect ratio
    /// and ensure that the original texture fits entirely inside the rect.
    /// At least one axis (x or y) will fit exactly. The result is centered inside the rect.
    FitCenter,
    /// Scaling the texture will maintain the original aspect ratio
    /// and ensure that the original texture fits entirely inside rect.
    /// At least one axis (x or y) will fit exactly.
    /// Aligns the result to the left and top edges of rect.
    FitStart,
    /// Scaling the texture will maintain the original aspect ratio
    /// and ensure that the original texture fits entirely inside rect.
    /// At least one axis (x or y) will fit exactly.
    /// Aligns the result to the right and bottom edges of rect.
    FitEnd,
}

/// Normalized (relative to its size) offset of a 2d renderable entity from its [`Transform`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Deref, DerefMut, Reflect)]
#[reflect(Component, Default, Debug, PartialEq, Clone)]
#[doc(alias = "pivot")]
pub struct Anchor(pub Vec2);

impl Anchor {
    pub const BOTTOM_LEFT: Self = Self(Vec2::new(-0.5, -0.5));
    pub const BOTTOM_CENTER: Self = Self(Vec2::new(0.0, -0.5));
    pub const BOTTOM_RIGHT: Self = Self(Vec2::new(0.5, -0.5));
    pub const CENTER_LEFT: Self = Self(Vec2::new(-0.5, 0.0));
    pub const CENTER: Self = Self(Vec2::ZERO);
    pub const CENTER_RIGHT: Self = Self(Vec2::new(0.5, 0.0));
    pub const TOP_LEFT: Self = Self(Vec2::new(-0.5, 0.5));
    pub const TOP_CENTER: Self = Self(Vec2::new(0.0, 0.5));
    pub const TOP_RIGHT: Self = Self(Vec2::new(0.5, 0.5));

    pub fn as_vec(&self) -> Vec2 {
        self.0
    }
}

impl Default for Anchor {
    fn default() -> Self {
        Self::CENTER
    }
}

impl From<Vec2> for Anchor {
    fn from(value: Vec2) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use bevy_asset::{Assets, RenderAssetUsages};
    use bevy_color::Color;
    use bevy_image::{Image, ToExtents};
    use bevy_image::{TextureAtlas, TextureAtlasLayout};
    use bevy_math::{Rect, URect, UVec2, Vec2};

    use crate::Anchor;

    use super::Sprite;

    #[test]
    fn compute_pixel_space_point_for_color_sprite() {
        let image_assets = Assets::<Image>::default();
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();

        // This also tests the `custom_size` field.
        let sprite = Sprite::from_color(Color::BLACK, Vec2::new(50.0, 100.0));

        let compute = |point| {
            sprite
                .compute_pixel_space_point(
                    point,
                    Anchor::default(),
                    &image_assets,
                    &texture_atlas_assets,
                )
                // Round to remove floating point errors.
                .map(|x| (x * 1e5).round() / 1e5)
                .map_err(|x| (x * 1e5).round() / 1e5)
        };
        assert_eq!(compute(Vec2::new(-20.0, -40.0)), Ok(Vec2::new(0.1, 0.9)));
        assert_eq!(compute(Vec2::new(0.0, 10.0)), Ok(Vec2::new(0.5, 0.4)));
        assert_eq!(compute(Vec2::new(75.0, 100.0)), Err(Vec2::new(2.0, -0.5)));
        assert_eq!(compute(Vec2::new(-75.0, -100.0)), Err(Vec2::new(-1.0, 1.5)));
        assert_eq!(compute(Vec2::new(-30.0, -40.0)), Err(Vec2::new(-0.1, 0.9)));
    }

}
