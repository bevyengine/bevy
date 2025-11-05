use std::any::type_name;

use crate::{AlphaMode2d, MeshMaterial2d};
use bevy_app::{App, Plugin, Update};
use bevy_asset::{Assets, Handle};
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    lifecycle::HookContext,
    query::Changed,
    reflect::{ReflectComponent, ReflectResource},
    resource::Resource,
    system::{Query, ResMut},
    world::DeferredWorld,
};
use bevy_image::Image;
use bevy_math::{primitives::Rectangle, UVec2};
use bevy_mesh::{Mesh, Mesh2d};
use bevy_platform::collections::HashMap;
use bevy_reflect::{prelude::*, Reflect};
use bevy_sprite::TileStorage;
use bevy_transform::components::Transform;
use bevy_utils::default;
use tracing::warn;

mod tilemap_chunk_material;

pub use tilemap_chunk_material::*;

/// Plugin that handles the initialization and updating of tilemap chunks.
/// Adds systems for processing newly added tilemap chunks and updating their indices.
pub struct TilemapChunkPlugin;

impl Plugin for TilemapChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TilemapChunkMeshCache>()
            .add_systems(Update, update_tilemap_chunk_indices);
    }
}

/// A resource storing the meshes for each tilemap chunk size.
#[derive(Resource, Default, Deref, DerefMut, Reflect)]
#[reflect(Resource, Default)]
pub struct TilemapChunkMeshCache(HashMap<UVec2, Handle<Mesh>>);

/// A component representing a chunk of a tilemap.
/// Each chunk is a rectangular section of tiles that is rendered as a single mesh.
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Clone, Debug, Default)]
#[component(immutable, on_insert = on_insert_tilemap_chunk)]
pub struct TilemapChunkRenderData {
    /// The size of the chunk in tiles.
    pub chunk_size: UVec2,
    /// The size to use for each tile, not to be confused with the size of a tile in the tileset image.
    /// The size of the tile in the tileset image is determined by the tileset image's dimensions.
    pub tile_display_size: UVec2,
    /// Handle to the tileset image containing all tile textures.
    pub tileset: Handle<Image>,
    /// The alpha mode to use for the tilemap chunk.
    pub alpha_mode: AlphaMode2d,
}

impl TilemapChunkRenderData {
    pub fn calculate_tile_transform(&self, position: UVec2) -> Transform {
        Transform::from_xyz(
            // tile position
            position.x as f32
            // times display size for a tile
            * self.tile_display_size.x as f32
            // plus 1/2 the tile_display_size to correct the center
            + self.tile_display_size.x as f32 / 2.
            // minus 1/2 the tilechunk size, in terms of the tile_display_size,
            // to place the 0 at left of tilemapchunk
            - self.tile_display_size.x as f32 * self.chunk_size.x as f32 / 2.,
            // tile position
            position.y as f32
            // times display size for a tile
            * self.tile_display_size.y as f32
            // minus 1/2 the tile_display_size to correct the center
            + self.tile_display_size.y as f32 / 2.
            // plus 1/2 the tilechunk size, in terms of the tile_display_size,
            // to place the 0 at bottom of tilemapchunk
            - self.tile_display_size.y as f32 * self.chunk_size.y as f32 / 2.,
            0.,
        )
    }
}

/// Data for a single tile in the tilemap chunk.
#[derive(Clone, Copy, Debug, Reflect)]
#[reflect(Clone, Debug, Default)]
pub struct TileRenderData {
    /// The index of the tile in the corresponding tileset array texture.
    pub tileset_index: u16,
    /// The color tint of the tile. White leaves the sampled texture color unchanged.
    pub color: Color,
    /// The visibility of the tile.
    pub visible: bool,
}

impl TileRenderData {
    /// Creates a new `TileData` with the given tileset index and default values.
    pub fn from_tileset_index(tileset_index: u16) -> Self {
        Self {
            tileset_index,
            ..default()
        }
    }
}

impl Default for TileRenderData {
    fn default() -> Self {
        Self {
            tileset_index: 0,
            color: Color::WHITE,
            visible: true,
        }
    }
}

fn on_insert_tilemap_chunk(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(tilemap_chunk) = world.get::<TilemapChunkRenderData>(entity) else {
        warn!(
            "{} not found for tilemap chunk {}",
            type_name::<TilemapChunkRenderData>(),
            entity
        );
        return;
    };

    let chunk_size = tilemap_chunk.chunk_size;
    let alpha_mode = tilemap_chunk.alpha_mode;
    let tileset = tilemap_chunk.tileset.clone();

    let Some(tile_data) = world.get::<TileStorage<TileRenderData>>(entity) else {
        warn!(
            "{} not found for tilemap chunk {}",
            type_name::<TileStorage<TileRenderData>>(),
            entity
        );
        return;
    };

    let expected_tile_data_length = chunk_size.element_product() as usize;
    if tile_data.tiles.len() != expected_tile_data_length {
        warn!(
            "Invalid tile data length for tilemap chunk {} of size {}. Expected {}, got {}",
            entity,
            chunk_size,
            expected_tile_data_length,
            tile_data.tiles.len(),
        );
        return;
    }

    let packed_tile_data: Vec<PackedTileData> =
        tile_data.tiles.iter().map(|&tile| tile.into()).collect();

    let tile_data_image = make_chunk_tile_data_image(&chunk_size, &packed_tile_data);

    let tilemap_chunk_mesh_cache = world.resource::<TilemapChunkMeshCache>();

    let mesh_size = chunk_size * tilemap_chunk.tile_display_size;

    let mesh = if let Some(mesh) = tilemap_chunk_mesh_cache.get(&mesh_size) {
        mesh.clone()
    } else {
        let mut meshes = world.resource_mut::<Assets<Mesh>>();
        meshes.add(Rectangle::from_size(mesh_size.as_vec2()))
    };

    let mut images = world.resource_mut::<Assets<Image>>();
    let tile_data = images.add(tile_data_image);

    let mut materials = world.resource_mut::<Assets<TilemapChunkMaterial>>();
    let material = materials.add(TilemapChunkMaterial {
        tileset,
        tile_data,
        alpha_mode,
    });

    world
        .commands()
        .entity(entity)
        .insert((Mesh2d(mesh), MeshMaterial2d(material)));
}

pub fn update_tilemap_chunk_indices(
    query: Query<
        (
            Entity,
            &TilemapChunkRenderData,
            &TileStorage<TileRenderData>,
            &MeshMaterial2d<TilemapChunkMaterial>,
        ),
        Changed<TileStorage<TileRenderData>>,
    >,
    mut materials: ResMut<Assets<TilemapChunkMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (chunk_entity, TilemapChunkRenderData { chunk_size, .. }, tile_data, material) in query {
        let expected_tile_data_length = chunk_size.element_product() as usize;
        if tile_data.tiles.len() != expected_tile_data_length {
            warn!(
                "Invalid TilemapChunkTileData length for tilemap chunk {} of size {}. Expected {}, got {}",
                chunk_entity,
                chunk_size,
                tile_data.tiles.len(),
                expected_tile_data_length
            );
            continue;
        }

        let packed_tile_data: Vec<PackedTileData> =
            tile_data.tiles.iter().map(|&tile| tile.into()).collect();

        // Getting the material mutably to trigger change detection
        let Some(material) = materials.get_mut(material.id()) else {
            warn!(
                "TilemapChunkMaterial not found for tilemap chunk {}",
                chunk_entity
            );
            continue;
        };
        let Some(tile_data_image) = images.get_mut(&material.tile_data) else {
            warn!(
                "TilemapChunkMaterial tile data image not found for tilemap chunk {}",
                chunk_entity
            );
            continue;
        };
        let Some(data) = tile_data_image.data.as_mut() else {
            warn!(
                "TilemapChunkMaterial tile data image data not found for tilemap chunk {}",
                chunk_entity
            );
            continue;
        };
        data.clear();
        data.extend_from_slice(bytemuck::cast_slice(&packed_tile_data));
    }
}
