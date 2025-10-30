use crate::{AlphaMode2d, MeshMaterial2d};
use bevy_app::{App, Plugin, Update};
use bevy_asset::{Assets, Handle};
use bevy_color::Color;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    hierarchy::ChildOf,
    lifecycle::HookContext,
    query::Changed,
    reflect::{ReflectComponent, ReflectResource},
    relationship::Relationship,
    resource::Resource,
    system::{Commands, Query, ResMut},
    world::DeferredWorld,
};
use bevy_image::Image;
use bevy_math::{primitives::Rectangle, UVec2};
use bevy_mesh::{Mesh, Mesh2d};
use bevy_platform::collections::HashMap;
use bevy_reflect::{prelude::*, Reflect};
use bevy_sprite::{TileData, TileStorage, Tilemap};
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

/// Information for rendering chunks in a tilemap
#[derive(Component, Clone, Debug, Default, Reflect)]
#[reflect(Component, Clone, Debug, Default)]
#[component(immutable)]
#[require(Transform)]
pub struct TilemapChunkRenderer {
    /// Handle to the tileset image containing all tile textures.
    pub tileset: Handle<Image>,
    /// The alpha mode to use for the tilemap chunk.
    pub alpha_mode: AlphaMode2d,
}

impl TilemapChunk {
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

impl TileData for TileRenderData {}

impl Default for TileRenderData {
    fn default() -> Self {
        Self {
            tileset_index: 0,
            color: Color::WHITE,
            visible: true,
        }
    }
}

/// Component storing the data of tiles within a chunk.
/// Each index corresponds to a specific tile in the tileset. `None` indicates an empty tile.
#[derive(Component, Clone, Debug, Deref, DerefMut, Reflect)]
#[reflect(Component, Clone, Debug)]
pub struct TilemapChunkTileData(pub Vec<Option<TileData>>);

fn on_insert_tilemap_chunk(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(tilemap_chunk) = world.get::<TilemapChunk>(entity) else {
        warn!("TilemapChunk not found for tilemap chunk {}", entity);
        return;
    };

    let chunk_size = tilemap_chunk.chunk_size;
    let alpha_mode = tilemap_chunk.alpha_mode;
    let tileset = tilemap_chunk.tileset.clone();

    let Some(tile_data) = world.get::<TilemapChunkTileData>(entity) else {
        warn!("TilemapChunkIndices not found for tilemap chunk {}", entity);
        return;
    };

    let expected_tile_data_length = chunk_size.element_product() as usize;
    if tile_data.len() != expected_tile_data_length {
        warn!(
            "Invalid tile data length for tilemap chunk {} of size {}. Expected {}, got {}",
            entity,
            chunk_size,
            expected_tile_data_length,
            tile_data.len(),
        );
        return;
    }

    let packed_tile_data: Vec<PackedTileData> =
        tile_data.0.iter().map(|&tile| tile.into()).collect();

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
            &ChildOf,
            &TileStorage<TileRenderData>,
            Option<&MeshMaterial2d<TilemapChunkMaterial>>,
        ),
        Changed<TileStorage<TileRenderData>>,
    >,
    map_query: Query<(&Tilemap, &TilemapChunkRenderer)>,
    mut tilemap_chunk_mesh_cache: ResMut<TilemapChunkMeshCache>,
    mut materials: ResMut<Assets<TilemapChunkMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut commands: Commands,
) {
    for (chunk_id, in_map, storage, material) in query {
        let Ok((map, map_renderer)) = map_query.get(in_map.get()) else {
            warn!(
                "Could not find Tilemap {} for chunk {}",
                in_map.get(),
                chunk_id
            );
            continue;
        };

        let packed_tile_data: Vec<PackedTileData> =
            storage.tiles.iter().map(|&tile| tile.into()).collect();

        // Getting the material mutably to trigger change detection
        if let Some(material) = material.and_then(|material| materials.get_mut(material.id())) {
            let Some(tile_data_image) = images.get_mut(&material.tile_data) else {
                warn!(
                    "TilemapChunkMaterial tile data image not found for tilemap chunk {}",
                    chunk_id
                );
                continue;
            };
            let Some(data) = tile_data_image.data.as_mut() else {
                warn!(
                    "TilemapChunkMaterial tile data image data not found for tilemap chunk {}",
                    chunk_id
                );
                continue;
            };
            data.clear();
            data.extend_from_slice(bytemuck::cast_slice(&packed_tile_data));
        } else {
            let tile_data_image = make_chunk_tile_data_image(&storage.size, &packed_tile_data);

            let mesh_size = storage.size * map.tile_display_size;

            let mesh = if let Some(mesh) = tilemap_chunk_mesh_cache.get(&mesh_size) {
                mesh.clone()
            } else {
                let mesh = meshes.add(Rectangle::from_size(mesh_size.as_vec2()));
                tilemap_chunk_mesh_cache.insert(mesh_size, mesh.clone());
                mesh
            };
            let tile_data = images.add(tile_data_image);

            let material = materials.add(TilemapChunkMaterial {
                tileset: map_renderer.tileset.clone(),
                tile_data,
                alpha_mode: map_renderer.alpha_mode,
            });

            commands
                .entity(chunk_id)
                .insert((Mesh2d(mesh), MeshMaterial2d(material)));
        };
    }
}
