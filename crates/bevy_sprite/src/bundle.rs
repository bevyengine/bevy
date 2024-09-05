use crate::Sprite;
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render::{
    texture::Image,
    view::{InheritedVisibility, ViewVisibility, Visibility},
};
use bevy_transform::components::{GlobalTransform, Transform};

/// A [`Bundle`] of components for drawing a single sprite from an image.
///
/// # Extra behaviours
///
/// You may add one or both of the following components to enable additional behaviours:
/// - [`ImageScaleMode`](crate::ImageScaleMode) to enable either slicing or tiling of the texture
/// - [`TextureAtlas`](crate::TextureAtlas) to draw a specific section of the texture
#[derive(Bundle, Clone, Debug, Default)]
pub struct SpriteBundle {
    /// Specifies the rendering properties of the sprite, such as color tint and flip.
    pub sprite: Sprite,
    /// The local transform of the sprite, relative to its parent.
    pub transform: Transform,
    /// The absolute transform of the sprite. This should generally not be written to directly.
    pub global_transform: GlobalTransform,
    /// A reference-counted handle to the image asset to be drawn.
    pub texture: Handle<Image>,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
}
