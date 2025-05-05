use crate::MeshMaterial2d;
use bevy_app::{App, Plugin, PreUpdate};
use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_color::ColorToPacked;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::{Component, HookContext},
    entity::Entity,
    hierarchy::ChildOf,
    name::Name,
    query::Changed,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, ResMut},
    world::DeferredWorld,
};
use bevy_image::{Image, ImageSampler};
use bevy_math::{FloatOrd, IVec2, UVec2, Vec2, Vec3};
use bevy_platform::collections::{HashMap, HashSet};
use bevy_render::{
    mesh::{Indices, Mesh, Mesh2d, PrimitiveTopology},
    render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
    view::ViewVisibility,
};
use bevy_transform::components::Transform;
use bytemuck::{Pod, Zeroable};
#[cfg(target_arch = "wasm32")]
use tracing::error;
use tracing::warn;

use super::{
    TileData, TileStorage, TilemapChunkMaterial, TilemapLayer, Tileset, ATTRIBUTE_TILE_INDEX,
};

/// Plugin that handles the initialization and updating of tilemap chunks.
/// Adds systems for processing newly added tilemap chunks and updating their indices.
pub struct TilemapChunkPlugin;

impl Plugin for TilemapChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TilemapChunkMeshCache>().add_systems(
            PreUpdate,
            (spawn_missing_tilemap_chunks, update_visible_tilemap_chunks).chain(),
        );
    }
}

type TilemapChunkMeshCacheKey = (UVec2, FloatOrd, FloatOrd);

/// A resource storing the meshes for each tilemap chunk size.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct TilemapChunkMeshCache(HashMap<TilemapChunkMeshCacheKey, Handle<Mesh>>);

/// A component representing a chunk of a tilemap.
/// Each chunk is a rectangular section of tiles that is rendered as a single mesh.
#[derive(Component, Clone, Debug)]
#[component(
    immutable,
    on_insert = on_tilemap_chunk_insert,
    on_replace = on_tilemap_chunk_insert,
    on_remove = on_tilemap_chunk_remove,
)]
#[require(Mesh2d, MeshMaterial2d<TilemapChunkMaterial>)]
pub struct TilemapChunk {
    tilemap_layer: Entity,
    location: IVec2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct PackedTileData {
    tileset_index: u16,
    flags: u16,    // flags (visibility, etc.)
    color_rg: u16, // r in low 8 bits, g in high 8 bits
    color_ba: u16, // b in low 8 bits, a in high 8 bits
}

impl PackedTileData {
    fn new(tile: &TileData) -> Self {
        let [r, g, b, a] = tile.color.to_srgba().to_u8_array();

        Self {
            tileset_index: tile.tileset_index,
            flags: tile.visible as u16,
            color_rg: (r as u16) | ((g as u16) << 8),
            color_ba: (b as u16) | ((a as u16) << 8),
        }
    }

    fn empty() -> Self {
        Self {
            tileset_index: u16::MAX,
            flags: 0,
            color_rg: 0,
            color_ba: 0,
        }
    }
}

fn on_tilemap_chunk_insert(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(tilemap_chunk) = world.get::<TilemapChunk>(entity) else {
        return;
    };

    let location = tilemap_chunk.location;

    let Some(mut tilemap_layer) = world.get_mut::<TilemapLayer>(tilemap_chunk.tilemap_layer) else {
        return;
    };

    tilemap_layer.chunks.insert(location, entity);
}

fn on_tilemap_chunk_remove(mut world: DeferredWorld, HookContext { entity, .. }: HookContext) {
    let Some(tilemap_chunk) = world.get::<TilemapChunk>(entity) else {
        return;
    };

    let location = tilemap_chunk.location;

    let Some(mut tilemap_layer) = world.get_mut::<TilemapLayer>(tilemap_chunk.tilemap_layer) else {
        return;
    };

    tilemap_layer.chunks.remove(&location);
}

fn spawn_missing_tilemap_chunks(
    tilemap_layer_query: Query<
        (Entity, &TilemapLayer, &TileStorage, &Tileset),
        Changed<TileStorage>,
    >,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut tilemap_chunk_mesh_cache: ResMut<TilemapChunkMeshCache>,
) {
    for (tilemap_layer_entity, tilemap, tile_storage, tileset) in tilemap_layer_query {
        let chunk_size = tile_storage.chunk_size();
        let display_size = (chunk_size * tileset.tile_size).as_vec2();

        for chunk_position in tile_storage
            .iter_dirty_chunk_positions()
            .filter(|pos| !tilemap.chunks.contains_key(*pos))
        {
            let chunk_world_position = chunk_position.as_vec2() * display_size;

            let mesh_key: TilemapChunkMeshCacheKey = (
                chunk_size,
                FloatOrd(display_size.x),
                FloatOrd(display_size.y),
            );

            let mesh = tilemap_chunk_mesh_cache
                .entry(mesh_key)
                .or_insert_with(|| meshes.add(make_chunk_mesh(&chunk_size, &display_size)));

            commands.spawn((
                Name::new(format!("TilemapChunk: {chunk_position}")),
                TilemapChunk {
                    tilemap_layer: tilemap_layer_entity,
                    location: *chunk_position,
                },
                Transform::from_translation(chunk_world_position.extend(0.0)),
                Mesh2d(mesh.clone()),
                ChildOf(tilemap_layer_entity),
            ));
        }
    }
}

fn update_visible_tilemap_chunks(
    tilemap_layer_query: Query<(Entity, &TilemapLayer, &mut TileStorage, &Tileset)>,
    mut chunk_query: Query<(
        Entity,
        &TilemapChunk,
        &mut MeshMaterial2d<TilemapChunkMaterial>,
        &ViewVisibility,
    )>,
    mut chunk_materials: ResMut<Assets<TilemapChunkMaterial>>,
    mut images: ResMut<Assets<Image>>,
    #[cfg(target_arch = "wasm32")] mut commands: Commands,
) {
    for (tilemap_layer_entity, tilemap_layer, mut tile_storage, tileset) in tilemap_layer_query {
        #[cfg(target_arch = "wasm32")]
        if let Some(tileset_image) = images.get(&tileset.image) {
            let layer_count = tileset_image.texture_descriptor.array_layer_count();
            if layer_count % 6 == 0 {
                commands.entity(tilemap_layer_entity).remove::<Tileset>();

                if layer_count == 6 {
                    error!("WebGL2: Tileset image has 6 layers which WebGL2 will interpret as a Cube texture. Ensure the layer count is not a multiple of 6.");
                } else {
                    error!("WebGL2: Tileset image has {} layers. This is a multiple of 6, which WebGL2 will interpret as a CubeArray texture. Ensure the layer count is not a multiple of 6.", layer_count);
                }
            }
        };

        let mut chunk_positions_to_clear = HashSet::new();
        for chunk_pos in tile_storage.iter_dirty_chunk_positions() {
            let Some(chunk_entity) = tilemap_layer.chunks.get(chunk_pos) else {
                continue;
            };
            let Ok((chunk_entity, chunk, mut chunk_material, visibility)) =
                chunk_query.get_mut(*chunk_entity)
            else {
                continue;
            };
            if !visibility.get() {
                continue;
            }

            chunk_positions_to_clear.insert(chunk.location);

            let chunk_size = tile_storage.chunk_size();
            let Ok(chunk_tiles) = tile_storage.iter_chunk_tiles(chunk.location) else {
                warn!(
                    "Unable to access TileStorage data for tilemap chunk {} in tilemap layer {}",
                    chunk_entity, tilemap_layer_entity
                );
                continue;
            };

            let packed_tiles: Vec<PackedTileData> = chunk_tiles
                .map(|tile_opt| {
                    tile_opt
                        .map(PackedTileData::new)
                        .unwrap_or_else(PackedTileData::empty)
                })
                .collect();

            if let Some(material) = chunk_materials.get_mut(chunk_material.id()) {
                let Some(chunk_image) = images.get_mut(&material.tile_data) else {
                    return;
                };
                let Some(data) = chunk_image.data.as_mut() else {
                    warn!(
                        "TilemapChunkMaterial tile data image data not found for tilemap chunk {} in tilemap layer {}",
                        chunk_entity,
                        tilemap_layer_entity
                    );
                    return;
                };
                data.clear();
                data.extend_from_slice(bytemuck::cast_slice(&packed_tiles));
            } else {
                let tile_data_image = make_chunk_tile_data_image(&chunk_size, &packed_tiles);

                let material = chunk_materials.add(TilemapChunkMaterial {
                    alpha_mode: tilemap_layer.alpha_mode,
                    tileset: tileset.image.clone(),
                    tile_data: images.add(tile_data_image),
                });

                *chunk_material = MeshMaterial2d(material);
            }
        }

        tile_storage.clear_dirty_chunk_positions(chunk_positions_to_clear);
    }
}

fn make_chunk_tile_data_image(size: &UVec2, data: &[PackedTileData]) -> Image {
    Image {
        data: Some(bytemuck::cast_slice(data).to_vec()),
        texture_descriptor: TextureDescriptor {
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba16Uint,
            label: None,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
            view_formats: &[],
        },
        sampler: ImageSampler::nearest(),
        texture_view_descriptor: None,
        asset_usage: RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    }
}

fn make_chunk_mesh(size: &UVec2, display_size: &Vec2) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    let num_quads = size.element_product() as usize;
    let quad_size = display_size / size.as_vec2();

    let mut positions = Vec::with_capacity(4 * num_quads);
    let mut uvs = Vec::with_capacity(4 * num_quads);
    let mut indices = Vec::with_capacity(6 * num_quads);

    for y in 0..size.y {
        for x in 0..size.x {
            let i = positions.len() as u32;

            let p0 = quad_size * UVec2::new(x, y).as_vec2();
            let p1 = p0 + quad_size;

            positions.extend([
                Vec3::new(p0.x, p0.y, 0.0),
                Vec3::new(p1.x, p0.y, 0.0),
                Vec3::new(p0.x, p1.y, 0.0),
                Vec3::new(p1.x, p1.y, 0.0),
            ]);

            uvs.extend([
                Vec2::new(0.0, 1.0),
                Vec2::new(1.0, 1.0),
                Vec2::new(0.0, 0.0),
                Vec2::new(1.0, 0.0),
            ]);

            indices.extend([i, i + 2, i + 1]);
            indices.extend([i + 3, i + 1, i + 2]);
        }
    }

    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(
        ATTRIBUTE_TILE_INDEX,
        (0..size.element_product())
            .flat_map(|i| [i; 4])
            .collect::<Vec<u32>>(),
    );
    mesh.insert_indices(Indices::U32(indices));

    mesh
}
