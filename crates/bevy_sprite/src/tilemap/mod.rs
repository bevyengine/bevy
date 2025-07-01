use crate::{AlphaMode2d, Anchor};
use bevy_app::{App, Plugin};
use bevy_asset::Handle;
use bevy_ecs::{component::Component, entity::Entity, name::Name, reflect::ReflectComponent};
use bevy_image::Image;
use bevy_math::UVec2;
use bevy_platform::collections::HashMap;
use bevy_reflect::Reflect;
use bevy_render::view::Visibility;
use bevy_transform::components::Transform;

mod chunk;
mod storage;
mod tile;

pub use chunk::*;
pub use storage::*;
pub use tile::*;

/// Plugin that handles the initialization and updating of tilemap chunks.
/// Adds systems for processing newly added tilemap chunks and updating their indices.
pub struct TilemapPlugin;

impl Plugin for TilemapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(TilemapChunkPlugin).add_plugins(TilePlugin);
    }
}

/// A component representing a tileset image containing all tile textures.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Clone, Debug)]
pub struct Tileset {
    pub image: Handle<Image>,
    pub tile_size: UVec2,
}

#[derive(Component, Clone, Debug, Reflect)]
#[reflect(Component, Clone, Debug)]
#[require(
    TileStorage,
    Tiles,
    Tileset,
    Name::new("TilemapLayer"),
    Transform,
    Visibility,
    Anchor
)]
pub struct TilemapLayer {
    pub chunks: HashMap<UVec2, Entity>,
    pub chunk_size: UVec2,
    pub tile_display_size: Option<UVec2>,
    pub alpha_mode: AlphaMode2d,
}

impl Default for TilemapLayer {
    fn default() -> Self {
        Self {
            chunks: HashMap::new(),
            chunk_size: UVec2::splat(32),
            tile_display_size: None,
            alpha_mode: AlphaMode2d::Blend,
        }
    }
}

#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Clone, Debug)]
#[relationship_target(relationship = TileOf)]
pub struct Tiles(Vec<Entity>);
