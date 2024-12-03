use bevy_asset::{Assets, Handle};
use bevy_color::Color;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_image::Image;
use bevy_math::{Rect, UVec2, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{sync_world::SyncToRenderWorld, view::Visibility};
use bevy_transform::components::Transform;

use crate::{TextureAtlas, TextureAtlasLayout, TextureSlicer};

/// Describes a sprite to be rendered to a 2D camera
#[derive(Component, Debug, Default, Clone, Reflect)]
#[require(Transform, Visibility, SyncToRenderWorld)]
#[reflect(Component, Default, Debug)]
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
    /// [`Anchor`] point of the sprite in the world
    pub anchor: Anchor,
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
        let sprite_center = -self.anchor.as_vec() * sprite_size;

        let mut point_relative_to_sprite_center = point_relative_to_sprite - sprite_center;

        if self.flip_x {
            point_relative_to_sprite_center.x *= -1.0;
        }
        // Texture coordinates start at the top left, whereas world coordinates start at the bottom
        // left. So flip by default, and then don't flip if `flip_y` is set.
        if !self.flip_y {
            point_relative_to_sprite_center.y *= -1.0;
        }

        let sprite_to_texture_ratio = {
            let texture_size = texture_rect.size();
            let div_or_zero = |a, b| if b == 0.0 { 0.0 } else { a / b };
            Vec2::new(
                div_or_zero(texture_size.x, sprite_size.x),
                div_or_zero(texture_size.y, sprite_size.y),
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
#[reflect(Debug)]
pub enum SpriteImageMode {
    /// The sprite will take on the size of the image by default, and will be stretched or shrunk if [`Sprite::custom_size`] is set.
    #[default]
    Auto,
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
}

/// How a sprite is positioned relative to its [`Transform`].
/// It defaults to `Anchor::Center`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default, Reflect)]
#[reflect(Component, Default, Debug, PartialEq)]
#[doc(alias = "pivot")]
pub enum Anchor {
    #[default]
    Center,
    BottomLeft,
    BottomCenter,
    BottomRight,
    CenterLeft,
    CenterRight,
    TopLeft,
    TopCenter,
    TopRight,
    /// Custom anchor point. Top left is `(-0.5, 0.5)`, center is `(0.0, 0.0)`. The value will
    /// be scaled with the sprite size.
    Custom(Vec2),
}

impl Anchor {
    pub fn as_vec(&self) -> Vec2 {
        match self {
            Anchor::Center => Vec2::ZERO,
            Anchor::BottomLeft => Vec2::new(-0.5, -0.5),
            Anchor::BottomCenter => Vec2::new(0.0, -0.5),
            Anchor::BottomRight => Vec2::new(0.5, -0.5),
            Anchor::CenterLeft => Vec2::new(-0.5, 0.0),
            Anchor::CenterRight => Vec2::new(0.5, 0.0),
            Anchor::TopLeft => Vec2::new(-0.5, 0.5),
            Anchor::TopCenter => Vec2::new(0.0, 0.5),
            Anchor::TopRight => Vec2::new(0.5, 0.5),
            Anchor::Custom(point) => *point,
        }
    }
}

#[cfg(test)]
mod tests {
    use bevy_asset::{Assets, RenderAssetUsages};
    use bevy_color::Color;
    use bevy_image::Image;
    use bevy_math::{Rect, URect, UVec2, Vec2};
    use bevy_render::render_resource::{Extent3d, TextureDimension, TextureFormat};

    use crate::{Anchor, TextureAtlas, TextureAtlasLayout};

    use super::Sprite;

    /// Makes a new image of the specified size.
    fn make_image(size: UVec2) -> Image {
        Image::new_fill(
            Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[0, 0, 0, 255],
            TextureFormat::Rgba8Unorm,
            RenderAssetUsages::all(),
        )
    }

    #[test]
    fn compute_pixel_space_point_for_regular_sprite() {
        let mut image_assets = Assets::<Image>::default();
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();

        let image = image_assets.add(make_image(UVec2::new(5, 10)));

        let sprite = Sprite {
            image,
            ..Default::default()
        };

        let compute =
            |point| sprite.compute_pixel_space_point(point, &image_assets, &texture_atlas_assets);
        assert_eq!(compute(Vec2::new(-2.0, -4.5)), Ok(Vec2::new(0.5, 9.5)));
        assert_eq!(compute(Vec2::new(0.0, 0.0)), Ok(Vec2::new(2.5, 5.0)));
        assert_eq!(compute(Vec2::new(0.0, 4.5)), Ok(Vec2::new(2.5, 0.5)));
        assert_eq!(compute(Vec2::new(3.0, 0.0)), Err(Vec2::new(5.5, 5.0)));
        assert_eq!(compute(Vec2::new(-3.0, 0.0)), Err(Vec2::new(-0.5, 5.0)));
    }

    #[test]
    fn compute_pixel_space_point_for_color_sprite() {
        let image_assets = Assets::<Image>::default();
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();

        // This also tests the `custom_size` field.
        let sprite = Sprite::from_color(Color::BLACK, Vec2::new(50.0, 100.0));

        let compute = |point| {
            sprite
                .compute_pixel_space_point(point, &image_assets, &texture_atlas_assets)
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

    #[test]
    fn compute_pixel_space_point_for_sprite_with_anchor_bottom_left() {
        let mut image_assets = Assets::<Image>::default();
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();

        let image = image_assets.add(make_image(UVec2::new(5, 10)));

        let sprite = Sprite {
            image,
            anchor: Anchor::BottomLeft,
            ..Default::default()
        };

        let compute =
            |point| sprite.compute_pixel_space_point(point, &image_assets, &texture_atlas_assets);
        assert_eq!(compute(Vec2::new(0.5, 9.5)), Ok(Vec2::new(0.5, 0.5)));
        assert_eq!(compute(Vec2::new(2.5, 5.0)), Ok(Vec2::new(2.5, 5.0)));
        assert_eq!(compute(Vec2::new(2.5, 9.5)), Ok(Vec2::new(2.5, 0.5)));
        assert_eq!(compute(Vec2::new(5.5, 5.0)), Err(Vec2::new(5.5, 5.0)));
        assert_eq!(compute(Vec2::new(-0.5, 5.0)), Err(Vec2::new(-0.5, 5.0)));
    }

    #[test]
    fn compute_pixel_space_point_for_sprite_with_anchor_top_right() {
        let mut image_assets = Assets::<Image>::default();
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();

        let image = image_assets.add(make_image(UVec2::new(5, 10)));

        let sprite = Sprite {
            image,
            anchor: Anchor::TopRight,
            ..Default::default()
        };

        let compute =
            |point| sprite.compute_pixel_space_point(point, &image_assets, &texture_atlas_assets);
        assert_eq!(compute(Vec2::new(-4.5, -0.5)), Ok(Vec2::new(0.5, 0.5)));
        assert_eq!(compute(Vec2::new(-2.5, -5.0)), Ok(Vec2::new(2.5, 5.0)));
        assert_eq!(compute(Vec2::new(-2.5, -0.5)), Ok(Vec2::new(2.5, 0.5)));
        assert_eq!(compute(Vec2::new(0.5, -5.0)), Err(Vec2::new(5.5, 5.0)));
        assert_eq!(compute(Vec2::new(-5.5, -5.0)), Err(Vec2::new(-0.5, 5.0)));
    }

    #[test]
    fn compute_pixel_space_point_for_sprite_with_anchor_flip_x() {
        let mut image_assets = Assets::<Image>::default();
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();

        let image = image_assets.add(make_image(UVec2::new(5, 10)));

        let sprite = Sprite {
            image,
            anchor: Anchor::BottomLeft,
            flip_x: true,
            ..Default::default()
        };

        let compute =
            |point| sprite.compute_pixel_space_point(point, &image_assets, &texture_atlas_assets);
        assert_eq!(compute(Vec2::new(0.5, 9.5)), Ok(Vec2::new(4.5, 0.5)));
        assert_eq!(compute(Vec2::new(2.5, 5.0)), Ok(Vec2::new(2.5, 5.0)));
        assert_eq!(compute(Vec2::new(2.5, 9.5)), Ok(Vec2::new(2.5, 0.5)));
        assert_eq!(compute(Vec2::new(5.5, 5.0)), Err(Vec2::new(-0.5, 5.0)));
        assert_eq!(compute(Vec2::new(-0.5, 5.0)), Err(Vec2::new(5.5, 5.0)));
    }

    #[test]
    fn compute_pixel_space_point_for_sprite_with_anchor_flip_y() {
        let mut image_assets = Assets::<Image>::default();
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();

        let image = image_assets.add(make_image(UVec2::new(5, 10)));

        let sprite = Sprite {
            image,
            anchor: Anchor::TopRight,
            flip_y: true,
            ..Default::default()
        };

        let compute =
            |point| sprite.compute_pixel_space_point(point, &image_assets, &texture_atlas_assets);
        assert_eq!(compute(Vec2::new(-4.5, -0.5)), Ok(Vec2::new(0.5, 9.5)));
        assert_eq!(compute(Vec2::new(-2.5, -5.0)), Ok(Vec2::new(2.5, 5.0)));
        assert_eq!(compute(Vec2::new(-2.5, -0.5)), Ok(Vec2::new(2.5, 9.5)));
        assert_eq!(compute(Vec2::new(0.5, -5.0)), Err(Vec2::new(5.5, 5.0)));
        assert_eq!(compute(Vec2::new(-5.5, -5.0)), Err(Vec2::new(-0.5, 5.0)));
    }

    #[test]
    fn compute_pixel_space_point_for_sprite_with_rect() {
        let mut image_assets = Assets::<Image>::default();
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();

        let image = image_assets.add(make_image(UVec2::new(5, 10)));

        let sprite = Sprite {
            image,
            rect: Some(Rect::new(1.5, 3.0, 3.0, 9.5)),
            anchor: Anchor::BottomLeft,
            ..Default::default()
        };

        let compute =
            |point| sprite.compute_pixel_space_point(point, &image_assets, &texture_atlas_assets);
        assert_eq!(compute(Vec2::new(0.5, 0.5)), Ok(Vec2::new(2.0, 9.0)));
        // The pixel is outside the rect, but is still a valid pixel in the image.
        assert_eq!(compute(Vec2::new(2.0, 2.5)), Err(Vec2::new(3.5, 7.0)));
    }

    #[test]
    fn compute_pixel_space_point_for_texture_atlas_sprite() {
        let mut image_assets = Assets::<Image>::default();
        let mut texture_atlas_assets = Assets::<TextureAtlasLayout>::default();

        let image = image_assets.add(make_image(UVec2::new(5, 10)));
        let texture_atlas = texture_atlas_assets.add(TextureAtlasLayout {
            size: UVec2::new(5, 10),
            textures: vec![URect::new(1, 1, 4, 4)],
        });

        let sprite = Sprite {
            image,
            anchor: Anchor::BottomLeft,
            texture_atlas: Some(TextureAtlas {
                layout: texture_atlas,
                index: 0,
            }),
            ..Default::default()
        };

        let compute =
            |point| sprite.compute_pixel_space_point(point, &image_assets, &texture_atlas_assets);
        assert_eq!(compute(Vec2::new(0.5, 0.5)), Ok(Vec2::new(1.5, 3.5)));
        // The pixel is outside the texture atlas, but is still a valid pixel in the image.
        assert_eq!(compute(Vec2::new(4.0, 2.5)), Err(Vec2::new(5.0, 1.5)));
    }

    #[test]
    fn compute_pixel_space_point_for_texture_atlas_sprite_with_rect() {
        let mut image_assets = Assets::<Image>::default();
        let mut texture_atlas_assets = Assets::<TextureAtlasLayout>::default();

        let image = image_assets.add(make_image(UVec2::new(5, 10)));
        let texture_atlas = texture_atlas_assets.add(TextureAtlasLayout {
            size: UVec2::new(5, 10),
            textures: vec![URect::new(1, 1, 4, 4)],
        });

        let sprite = Sprite {
            image,
            anchor: Anchor::BottomLeft,
            texture_atlas: Some(TextureAtlas {
                layout: texture_atlas,
                index: 0,
            }),
            // The rect is relative to the texture atlas sprite.
            rect: Some(Rect::new(1.5, 1.5, 3.0, 3.0)),
            ..Default::default()
        };

        let compute =
            |point| sprite.compute_pixel_space_point(point, &image_assets, &texture_atlas_assets);
        assert_eq!(compute(Vec2::new(0.5, 0.5)), Ok(Vec2::new(3.0, 3.5)));
        // The pixel is outside the texture atlas, but is still a valid pixel in the image.
        assert_eq!(compute(Vec2::new(4.0, 2.5)), Err(Vec2::new(6.5, 1.5)));
    }

    #[test]
    fn compute_pixel_space_point_for_sprite_with_custom_size_and_rect() {
        let mut image_assets = Assets::<Image>::default();
        let texture_atlas_assets = Assets::<TextureAtlasLayout>::default();

        let image = image_assets.add(make_image(UVec2::new(5, 10)));

        let sprite = Sprite {
            image,
            custom_size: Some(Vec2::new(100.0, 50.0)),
            rect: Some(Rect::new(0.0, 0.0, 5.0, 5.0)),
            ..Default::default()
        };

        let compute =
            |point| sprite.compute_pixel_space_point(point, &image_assets, &texture_atlas_assets);
        assert_eq!(compute(Vec2::new(30.0, 15.0)), Ok(Vec2::new(4.0, 1.0)));
        assert_eq!(compute(Vec2::new(-10.0, -15.0)), Ok(Vec2::new(2.0, 4.0)));
        // The pixel is outside the texture atlas, but is still a valid pixel in the image.
        assert_eq!(compute(Vec2::new(0.0, 35.0)), Err(Vec2::new(2.5, -1.0)));
    }
}
