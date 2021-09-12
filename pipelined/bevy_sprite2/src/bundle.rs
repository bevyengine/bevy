use crate::{
    texture_atlas::{TextureAtlas, TextureAtlasEntry},
    Sprite2d, Sprite3d,
};
use bevy_asset::Handle;
use bevy_ecs::bundle::Bundle;
use bevy_render2::texture::Image;
use bevy_transform::components::{GlobalTransform, Transform};

#[derive(Bundle, Clone)]
pub struct Sprite2dBundle {
    pub sprite: Sprite2d,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub texture: Handle<Image>,
}

impl Default for Sprite2dBundle {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            texture: Default::default(),
        }
    }
}

/// A Bundle of components for drawing a single sprite from a sprite sheet (also referred
/// to as a `TextureAtlas`)
#[derive(Bundle, Clone)]
pub struct Sprite2dSheetBundle {
    pub sprite: Sprite2d,
    /// The specific sprite from the texture atlas to be drawn
    pub texture_atlas_entry: TextureAtlasEntry,
    /// A handle to the texture atlas that holds the sprite images
    pub texture_atlas: Handle<TextureAtlas>,
    /// Data pertaining to how the sprite is drawn on the screen
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for Sprite2dSheetBundle {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            texture_atlas_entry: Default::default(),
            texture_atlas: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}

#[derive(Bundle, Clone)]
pub struct Sprite3dBundle {
    pub sprite: Sprite3d,
    pub transform: Transform,
    pub global_transform: GlobalTransform,
    pub texture: Handle<Image>,
}

impl Default for Sprite3dBundle {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
            texture: Default::default(),
        }
    }
}

/// A Bundle of components for drawing a single sprite from a sprite sheet (also referred
/// to as a `TextureAtlas`)
#[derive(Bundle, Clone)]
pub struct Sprite3dSheetBundle {
    pub sprite: Sprite3d,
    /// The specific sprite from the texture atlas to be drawn
    pub texture_atlas_entry: TextureAtlasEntry,
    /// A handle to the texture atlas that holds the sprite images
    pub texture_atlas: Handle<TextureAtlas>,
    /// Data pertaining to how the sprite is drawn on the screen
    pub transform: Transform,
    pub global_transform: GlobalTransform,
}

impl Default for Sprite3dSheetBundle {
    fn default() -> Self {
        Self {
            sprite: Default::default(),
            texture_atlas_entry: Default::default(),
            texture_atlas: Default::default(),
            transform: Default::default(),
            global_transform: Default::default(),
        }
    }
}
