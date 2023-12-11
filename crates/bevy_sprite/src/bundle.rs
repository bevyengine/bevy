use crate::{
    texture_atlas::{TextureAtlas, TextureAtlasSprite},
    Sprite, SpriteMaterial, SpriteWithMaterial, TextureAtlasSpriteWithMaterial,
};
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
/// to as a `TextureAtlas`)
#[derive(Bundle, Clone, Default)]
pub struct SpriteSheetBundle {
    /// The specific sprite from the texture atlas to be drawn, defaulting to the sprite at index 0.
    pub sprite: TextureAtlasSprite,
    /// A handle to the texture atlas that holds the sprite images
    pub texture_atlas: Handle<TextureAtlas>,
    /// Data pertaining to how the sprite is drawn on the screen
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
}

#[derive(Bundle, Clone)]
pub struct SpriteWithMaterialBundle<M: SpriteMaterial> {
    pub sprite: SpriteWithMaterial<M>,
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

impl<M: SpriteMaterial> Default for SpriteWithMaterialBundle<M> {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            texture: Default::default(),
            visibility: Default::default(),
            inherited_visibility: Default::default(),
            view_visibility: Default::default(),
        }
    }
}

/// A Bundle of components for drawing a single sprite from a sprite sheet (also referred
/// to as a `TextureAtlas`)
#[derive(Bundle, Clone)]
pub struct SpriteSheetWithMaterialBundle<M: SpriteMaterial> {
    /// The specific sprite from the texture atlas to be drawn, defaulting to the sprite at index 0.
    pub sprite: TextureAtlasSpriteWithMaterial<M>,
    /// A handle to the texture atlas that holds the sprite images
    pub texture_atlas: Handle<TextureAtlas>,
    /// Data pertaining to how the sprite is drawn on the screen
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
}

impl<M: SpriteMaterial> Default for SpriteSheetWithMaterialBundle<M> {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            texture_atlas: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            inherited_visibility: Default::default(),
            view_visibility: Default::default(),
        }
    }
}
