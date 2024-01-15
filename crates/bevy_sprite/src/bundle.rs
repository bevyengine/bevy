use crate::{
    texture_atlas::{TextureAtlas, TextureAtlasSprite},
    Sprite, SpriteMaterial, SpriteMaterialMarke,
};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render::{
    texture::Image,
    view::{InheritedVisibility, ViewVisibility, Visibility},
};
use bevy_transform::components::{GlobalTransform, Transform};

/// A [`Bundle`] of components for drawing a single sprite from an image.
#[derive(Bundle, Clone, Default)]
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

/// A [`Bundle`] of components for drawing a single sprite from a sprite sheet (also referred
/// to as a `TextureAtlas`).
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
    pub sprite: Sprite,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub texture: Handle<Image>,
    pub material: Handle<M>,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    /// Inherited visibility of an entity.
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    pub marke: SpriteMaterialMarke,
}

impl<M: SpriteMaterial> Default for SpriteWithMaterialBundle<M> {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            texture: Default::default(),
            material: Default::default(),
            visibility: Default::default(),
            inherited_visibility: Default::default(),
            view_visibility: Default::default(),
            marke: Default::default(),
        }
    }
}

/// A Bundle of components for drawing a single sprite from a sprite sheet (also referred
/// to as a `TextureAtlas`)
#[derive(Bundle, Clone)]
pub struct SpriteSheetWithMaterialBundle<M: SpriteMaterial> {
    /// The specific sprite from the texture atlas to be drawn, defaulting to the sprite at index 0.
    pub sprite: TextureAtlasSprite,
    /// A handle to the texture atlas that holds the sprite images
    pub texture_atlas: Handle<TextureAtlas>,
    pub material: Handle<M>,
    /// Data pertaining to how the sprite is drawn on the screen
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    /// User indication of whether an entity is visible
    pub visibility: Visibility,
    pub inherited_visibility: InheritedVisibility,
    /// Algorithmically-computed indication of whether an entity is visible and should be extracted for rendering
    pub view_visibility: ViewVisibility,
    pub marke: SpriteMaterialMarke,
}

impl<M: SpriteMaterial> Default for SpriteSheetWithMaterialBundle<M> {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            texture_atlas: Default::default(),
            material: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            visibility: Default::default(),
            inherited_visibility: Default::default(),
            view_visibility: Default::default(),
            marke: Default::default(),
        }
    }
}
