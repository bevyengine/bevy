use bevy_asset::{AssetId, Handle};
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::{Rect, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{texture::Image, view::Visibility, world_sync::SyncToRenderWorld};
use bevy_transform::components::Transform;

use crate::TextureSlicer;

/// Specifies the rendering properties of a sprite.
///
#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Default, Debug)]
pub struct SpriteProperties {
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
    /// the full image. This is an easy one-off alternative to using a [`TextureAtlas`](crate::TextureAtlas).
    ///
    /// When used with a [`TextureAtlas`](crate::TextureAtlas), the rect
    /// is offset by the atlas's minimal (top-left) corner position.
    pub rect: Option<Rect>,
    /// [`Anchor`] point of the sprite in the world
    pub anchor: Anchor,
}

impl SpriteProperties {
    /// Create a Sprite with a custom size
    pub fn sized(custom_size: Vec2) -> Self {
        SpriteProperties {
            custom_size: Some(custom_size),
            ..Default::default()
        }
    }
}

/// Controls how the image is altered when scaled.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component, Debug)]
pub enum ImageScaleMode {
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

/// How a sprite is positioned relative to its [`Transform`](bevy_transform::components::Transform).
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

/// A component for rendering sprites.
///
/// # Example
///
/// ```ignore
/// # use bevy_ecs::prelude::*;
/// # use bevy_render::texture::Image;
/// # use bevy_asset::{AssetServer, Assets};
/// #
/// // Spawn an entity with a sprite.
/// fn setup(
///     mut commands: Commands,
///     mut images: ResMut<Assets<Image>>,
///     asset_server: Res<AssetServer>
/// ) {
///     commands.spawn((
///         Sprite(images.add(Image::default())),
///     ));
/// }
/// ```
#[derive(Component, Clone, Debug, Default, Deref, DerefMut, Reflect, PartialEq, Eq)]
#[reflect(Component, Default)]
#[require(SpriteProperties, Transform, Visibility, SyncToRenderWorld)]
pub struct Sprite(pub Handle<Image>);

impl From<Handle<Image>> for Sprite {
    fn from(handle: Handle<Image>) -> Self {
        Self(handle)
    }
}

impl From<Sprite> for AssetId<Image> {
    fn from(texture: Sprite) -> Self {
        texture.id()
    }
}

impl From<&Sprite> for AssetId<Image> {
    fn from(texture: &Sprite) -> Self {
        texture.id()
    }
}
