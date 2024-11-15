#![expect(deprecated)]
use crate::Sprite;
use bevy_ecs::bundle::Bundle;
use bevy_render::{
    sync_world::SyncToRenderWorld,
    view::{InheritedVisibility, ViewVisibility, Visibility},
};
use bevy_transform::components::{GlobalTransform, Transform};

/// A [`Bundle`] of components for drawing a single sprite from an image.
#[derive(Bundle, Clone, Debug, Default)]
#[deprecated(
    since = "0.15.0",
    note = "Use the `Sprite` component instead. Inserting it will now also insert `Transform` and `Visibility` automatically."
)]
pub struct SpriteBundle {
    /// Specifies the rendering properties of the sprite, such as color tint and flip.
    pub sprite: Sprite,
    /// The local transform of the sprite, relative to its parent.
    pub transform: Transform,
    /// The absolute transform of the sprite. This should generally not be written to directly.
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    /// Marker component that indicates that its entity needs to be synchronized to the render world
    pub sync: SyncToRenderWorld,
}
