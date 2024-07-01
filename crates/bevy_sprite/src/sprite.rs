use bevy_color::Color;
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::{Rect, Vec2};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

use crate::TextureSlicer;

/// Specifies the rendering properties of a sprite.
///
/// This is commonly used as a component within [`SpriteBundle`](crate::bundle::SpriteBundle).
#[derive(Component, Debug, Default, Clone, Reflect)]
#[reflect(Component, Default)]
#[repr(C)]
pub struct Sprite {
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

/// Controls how the image is altered when scaled.
#[derive(Component, Debug, Clone, Reflect)]
#[reflect(Component)]
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
