use bevy_app::{App, Plugin, Update};
use bevy_asset::{
    embedded_asset, embedded_path, Asset, AssetPath, Assets, Handle, RenderAssetUsages,
};
use bevy_color::ColorToPacked;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    lifecycle::HookContext,
    query::With,
    reflect::ReflectComponent,
    resource::Resource,
    system::{Commands, Query, ResMut},
    world::DeferredWorld,
};
use bevy_image::{Image, ImageSampler, ToExtents};
use bevy_math::UVec2;
use bevy_platform::collections::{HashMap, HashSet};
use bevy_reflect::{Reflect, TypePath};
use bevy_render::{
    mesh::Mesh,
    render_resource::{
        AsBindGroup, ShaderRef, TextureDataOrder, TextureDescriptor, TextureDimension,
        TextureFormat, TextureUsages,
    },
    view::ViewVisibility,
};
use bytemuck::{Pod, Zeroable};
use tracing::warn;

use crate::{
    AlphaMode2d, Material2d, Material2dPlugin, MeshMaterial2d, TileColor, TileIndex, TileStorage,
    TileVisible, TilemapLayer, Tileset,
};

/// Plugin that adds support for tilemap chunk materials.
pub struct TilemapChunkPlugin;

impl Plugin for TilemapChunkPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "tilemap_chunk.wgsl");

        app.add_plugins(Material2dPlugin::<TilemapChunkMaterial>::default())
            .init_resource::<TilemapChunkMeshCache>()
            .add_systems(Update, update_visible_tilemap_chunks);
    }
}

/// A resource storing the meshes for each tilemap chunk size.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct TilemapChunkMeshCache(HashMap<UVec2, Handle<Mesh>>);

/// A component representing a chunk of a tilemap.
/// Each chunk is a rectangular section of tiles that is rendered as a single mesh.
#[derive(Component, Clone, Debug)]
#[component(
    immutable,
    on_insert = on_tilemap_chunk_insert,
    on_replace = on_tilemap_chunk_insert,
    on_remove = on_tilemap_chunk_remove,
)]
#[require(MeshMaterial2d<TilemapChunkMaterial>)]
pub struct TilemapChunk {
    pub tilemap_layer: Entity,
    pub location: UVec2,
}

#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct TilemapChunkDirty;

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

fn update_visible_tilemap_chunks(
    tilemap_layer_query: Query<(Entity, &TilemapLayer, &TileStorage, &Tileset)>,
    tile_query: Query<(&TileIndex, Option<&TileVisible>, Option<&TileColor>)>,
    mut chunk_query: Query<
        (
            Entity,
            &TilemapChunk,
            &mut MeshMaterial2d<TilemapChunkMaterial>,
            &ViewVisibility,
        ),
        With<TilemapChunkDirty>,
    >,
    mut chunk_materials: ResMut<Assets<TilemapChunkMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    let mut chunk_entities_to_undirty = HashSet::new();
    for (chunk_entity, chunk, mut chunk_material, visibility) in &mut chunk_query {
        if !visibility.get() {
            continue;
        }

        let Ok((tilemap_layer_entity, tilemap_layer, tile_storage, tileset)) =
            tilemap_layer_query.get(chunk.tilemap_layer)
        else {
            warn!("TilemapChunk: TilemapLayer not found");
            continue;
        };

        #[cfg(target_arch = "wasm32")]
        if let Some(tileset_image) = images.get(&tileset.image) {
            let layer_count = tileset_image.texture_descriptor.array_layer_count();
            if layer_count % 6 == 0 {
                commands.entity(tilemap_layer_entity).remove::<Tileset>();

                if layer_count == 6 {
                    error!(
                        "WebGL2: Tileset image has 6 layers which WebGL2 will interpret as a Cube texture. Ensure the layer count is not a multiple of 6. The Tileset component has been removed."
                    );
                } else {
                    error!(
                        "WebGL2: Tileset image has {} layers. This is a multiple of 6, which WebGL2 will interpret as a CubeArray texture. Ensure the layer count is not a multiple of 6. The Tileset component has been removed.",
                        layer_count
                    );
                }
            }
        };

        chunk_entities_to_undirty.insert(chunk_entity);

        let chunk_size = tilemap_layer.chunk_size;
        let chunk_tiles = tile_storage.iter_chunk_tiles(chunk.location, chunk_size);

        let packed_tiles: Vec<PackedTileData> = chunk_tiles
            .map(|tile_opt| {
                tile_opt
                    .map(|tile_entity| {
                        let Ok((tile_index, tile_visible, tile_color)) =
                            tile_query.get(tile_entity)
                        else {
                            return PackedTileData::empty();
                        };

                        PackedTileData::new(
                            tile_index.clone(),
                            tile_visible.cloned().unwrap_or_default(),
                            tile_color.cloned().unwrap_or_default(),
                        )
                    })
                    .unwrap_or_else(PackedTileData::empty)
            })
            .collect();

        if let Some(material) = chunk_materials.get_mut(chunk_material.id()) {
            let Some(chunk_image) = images.get_mut(&material.tile_data) else {
                warn!(
                    "TilemapChunkMaterial tile data image not found for tilemap chunk {} in tilemap layer {}",
                    chunk_entity, tilemap_layer_entity
                );
                return;
            };
            let Some(data) = chunk_image.data.as_mut() else {
                warn!(
                    "TilemapChunkMaterial tile data image data not found for tilemap chunk {} in tilemap layer {}",
                    chunk_entity, tilemap_layer_entity
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

    for chunk_entity in chunk_entities_to_undirty {
        commands.entity(chunk_entity).remove::<TilemapChunkDirty>();
    }
}

/// Material used for rendering tilemap chunks.
///
/// This material is used internally by the tilemap system to render chunks of tiles
/// efficiently using a single draw call per chunk.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TilemapChunkMaterial {
    pub alpha_mode: AlphaMode2d,

    #[texture(0, dimension = "2d_array")]
    #[sampler(1)]
    pub tileset: Handle<Image>,

    #[texture(2, sample_type = "u_int")]
    pub tile_data: Handle<Image>,
}

impl Material2d for TilemapChunkMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("tilemap_chunk.wgsl")).with_source("embedded"),
        )
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        self.alpha_mode
    }
}

/// Packed per-tile data for use in the `Rgba16Uint` tile data texture in `TilemapChunkMaterial`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PackedTileData {
    index: u16,     // red channel
    color: [u8; 4], // green and blue channels
    flags: u16,     // alpha channel
}

impl PackedTileData {
    fn new(index: TileIndex, visible: TileVisible, color: TileColor) -> Self {
        Self {
            index: index.0,
            color: color.0.to_srgba().to_u8_array(),
            flags: visible.0 as u16,
        }
    }

    fn empty() -> Self {
        Self {
            index: u16::MAX,
            color: [0, 0, 0, 0],
            flags: 0,
        }
    }
}

fn make_chunk_tile_data_image(size: &UVec2, data: &[PackedTileData]) -> Image {
    Image {
        data: Some(bytemuck::cast_slice(data).to_vec()),
        data_order: TextureDataOrder::default(),
        texture_descriptor: TextureDescriptor {
            size: size.to_extents(),
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
        copy_on_resize: false,
    }
}
