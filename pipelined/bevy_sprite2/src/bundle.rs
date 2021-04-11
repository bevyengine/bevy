use crate::Sprite;
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render2::texture::Texture;
use bevy_transform::components::{GlobalTransform, Transform};

#[derive(Bundle, Clone)]
pub struct PipelinedSpriteBundle {
    pub sprite: Sprite,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub texture: Handle<Texture>,
}

impl Default for PipelinedSpriteBundle {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            texture: Default::default(),
        }
    }
}
