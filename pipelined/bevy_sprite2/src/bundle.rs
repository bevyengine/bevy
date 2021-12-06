use crate::{
    texture_atlas::{TextureAtlas, TextureAtlasSprite},
    Sprite,
};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render2::{
    texture::Image,
    view::{ComputedVisibility, Visibility},
};
use bevy_transform::components::{GlobalTransform, Transform};

#[derive(Bundle, Clone, Default)]
pub struct PipelinedSpriteBundle {
    pub sprite: Sprite,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub texture: Handle<Image>,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}

/// A Bundle of components for drawing a single sprite from a sprite sheet (also referred
/// to as a `TextureAtlas`)
#[derive(Bundle, Clone, Default)]
pub struct PipelinedSpriteSheetBundle {
    /// The specific sprite from the texture atlas to be drawn
    pub sprite: TextureAtlasSprite,
    /// A handle to the texture atlas that holds the sprite images
    pub texture_atlas: Handle<TextureAtlas>,
    /// Data pertaining to how the sprite is drawn on the screen
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub computed_visibility: ComputedVisibility,
}
