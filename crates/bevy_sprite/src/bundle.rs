use crate::{Sprite, SpriteImage};
use bevy_ecs::bundle::Bundle;
use bevy_render::view::Visibility;
use bevy_transform::components::{GlobalTransform, Transform};

#[derive(Bundle, Clone, Default)]
pub struct SpriteBundle {
    pub sprite: Sprite,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// The sprite texture
    pub texture: SpriteImage,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
}
