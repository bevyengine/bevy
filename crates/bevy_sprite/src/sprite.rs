use crate::{BorderRect, Rect};
use bevy_ecs::component::Component;
use bevy_math::{Rect, Vec2};
use bevy_reflect::Reflect;
use bevy_render::color::Color;

#[derive(Component, Debug, Default, Clone, Reflect)]
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
    /// An optional rectangle representing the region of the sprite's image to render, instead of
    /// rendering the full image. This is an easy one-off alternative to using a texture atlas.
    pub rect: Option<Rect>,
    /// [`Anchor`] point of the sprite in the world
    pub anchor: Anchor,
}

/// Defines how the non corner sections of a [`SpriteSlice`] are scaled.
#[derive(Debug, Copy, Clone, Default, Reflect)]
pub enum SliceScaleMode {
    /// The sections will stretch
    #[default]
    Stretch,
    /// The sections will repeat
    Tile,
}

/// Component for [9-sliced](https://en.wikipedia.org/wiki/9-slice_scaling) sprites.
///
/// When resizing a 9-sliced sprite the corners will remain unscaled while the other sections will be scaled or tiled
#[derive(Component, Debug, Clone, Default, Reflect)]
pub struct SpriteSlice {
    /// The sprite borders, defining the 9 sections of the image
    pub border: BorderRect,
    /// How do the the non corner sections scale
    pub scale_mode: SliceScaleMode,
}

/// How a sprite is positioned relative to its [`Transform`](bevy_transform::components::Transform).
/// It defaults to `Anchor::Center`.
#[derive(Debug, Clone, Default, Reflect)]
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

impl SpriteSlice {
    /// Computes the 9 [`Rect`] and size values for a [`Sprite`] given its `image_size`
    pub fn slice_rects(&self, image_size: Vec2) -> [Rect; 9] {
        // corners
        let bl_corner = Rect {
            min: Vec2::ZERO,
            max: Vec2::new(self.border.left, self.border.bottom),
        };
        let br_corner = Rect {
            min: Vec2::new(image_size.x - self.border.right, 0.0),
            max: Vec2::new(image_size.x, self.border.bottom),
        };
        let tl_corner = Rect {
            min: Vec2::new(0.0, image_size.y - self.border.top),
            max: Vec2::new(self.border.left, image_size.y),
        };
        let tr_corner = Rect {
            min: Vec2::new(
                image_size.x - self.border.right,
                image_size.y - self.border.top,
            ),
            max: Vec2::new(image_size.x, image_size.y),
        };
        // Sides
        let left_side = Rect {
            min: Vec2::new(0.0, self.border.bottom),
            max: Vec2::new(self.border.left, image_size.y - self.border.top),
        };
        let right_side = Rect {
            min: Vec2::new(image_size.x - self.border.right, self.border.bottom),
            max: Vec2::new(image_size.x, image_size.y - self.border.top),
        };
        let bot_side = Rect {
            min: Vec2::new(self.border.left, 0.0),
            max: Vec2::new(image_size.x - self.border.right, self.border.bottom),
        };
        let top_side = Rect {
            min: Vec2::new(self.border.left, image_size.y - self.border.top),
            max: Vec2::new(image_size.x - self.border.right, image_size.y),
        };
        // Center
        let center = Rect {
            min: Vec2::new(self.border.left, self.border.bottom),
            max: Vec2::new(
                image_size.x - self.border.right,
                image_size.y - self.border.top,
            ),
        };
        [
            bl_corner, br_corner, tl_corner, tr_corner, left_side, right_side, bot_side, top_side,
            center,
        ]
    }
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
