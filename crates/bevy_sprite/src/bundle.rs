use crate::{Sprite, TextureAtlas};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render::{
    texture::Image,
    view::{InheritedVisibility, ViewVisibility, Visibility},
};
use bevy_transform::components::{GlobalTransform, Transform};

#[derive(Bundle, Clone, Default)]
pub struct SpriteBundle {
    pub sprite: Sprite,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub texture: Handle<Image>,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
}

/// A Bundle of components for drawing a single sprite from a sprite sheet (also referred
/// to as a `TextureAtlas`) or for animated sprites.
///
/// Note:
/// This bundle is identical to [`SpriteBundle`] with an additional [`TextureAtlas`] component.
///
/// Check the following examples for usage:
/// - [`animated sprite sheet example`](https://github.com/bevyengine/bevy/blob/latest/examples/2d/sprite_sheet.rs)
/// - [`texture atlas example`](https://github.com/bevyengine/bevy/blob/latest/examples/2d/texture_atlas.rs)
#[derive(Bundle, Clone, Default)]
pub struct SpriteSheetBundle {
    pub sprite: Sprite,
    /// The sprite sheet base texture
    pub texture: Handle<Image>,
    /// The sprite sheet texture atlas, allowing to draw a custom section of `texture`.
    pub atlas: TextureAtlas,
    /// Data pertaining to how the sprite is drawn on the screen
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
}
