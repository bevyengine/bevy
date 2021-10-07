use bevy_math::Vec2;
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render2::color::Color;

/// A sprite stores all the metadata required to draw a 2D [`Image`](bevy_render2::image::Image).
/// It is used to render it's corresponding image resized or flipped.
#[derive(Debug, Default, Clone, TypeUuid, Reflect)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
#[repr(C)]
pub struct Sprite {
    /// Flip the sprite along the X axis.
    pub flip_x: bool, // Todo: actually use these
    /// Flip the sprite along the Y axis.
    pub flip_y: bool, // Todo: actually use these
    /// An optional custom size for the sprite that will be used when rendering, instead of the size
    /// of the sprite's image.
    pub custom_size: Option<Vec2>,
}

// Todo: mirror Sprite API
/// A sprite that stores all the metadata required to draw a region of an [`ImageAtlas`].
/// It is used to render it's corresponding region resized or flipped.
#[derive(Debug, Clone, TypeUuid, Reflect)]
#[uuid = "7233c597-ccfa-411f-bd59-9af349432ada"]
pub struct AtlasSprite {
    pub region_index: u32,
    pub color: Color, // what is this used for?
    pub flip_x: bool, // Todo: actually use these
    pub flip_y: bool, // Todo: actually use these
                      // /// An optional custom size for the sprite that will be used when rendering, instead of the size
                      // /// of the sprite's region.
                      // pub custom_size: Option<Vec2>, // Todo: actually use these
}

impl Default for AtlasSprite {
    fn default() -> Self {
        Self {
            region_index: 0,
            color: Color::WHITE,
            flip_x: false,
            flip_y: false,
        }
    }
}

impl AtlasSprite {
    pub fn new(index: u32) -> AtlasSprite {
        Self {
            region_index: index,
            ..Default::default()
        }
    }
}
