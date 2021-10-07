use crate::{image_atlas::ImageAtlas, AtlasSprite, Sprite};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render2::image::Image;
use bevy_transform::components::{GlobalTransform, Transform};

/// A bundle of components for drawing an sprite.
#[derive(Bundle, Clone)]
pub struct PipelinedSpriteBundle {
    /// The sprite information used to render the image.
    pub sprite: Sprite,
    /// The image of the sprite.
    pub image: Handle<Image>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for PipelinedSpriteBundle {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            image: Default::default(),
        }
    }
}

/// A bundle of components for drawing a single sprite from a [`ImageAtlas`].
#[derive(Bundle, Clone)]
pub struct PipelinedAtlasSpriteBundle {
    /// The specific sprite from the texture atlas to be drawn.
    pub sprite: AtlasSprite,
    /// The image atlas that holds the image of the sprite.
    pub image_atlas: Handle<ImageAtlas>,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for PipelinedAtlasSpriteBundle {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            image_atlas: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}
