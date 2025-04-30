use crate::{AlphaMode2d, Anchor, MeshMaterial2d};
use bevy_app::{App, Plugin, Update};
use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    observer::Trigger,
    query::Changed,
    resource::Resource,
    system::{Commands, Query, ResMut},
    world::OnAdd,
};
use bevy_image::{Image, ImageSampler};
use bevy_math::{FloatOrd, UVec2, Vec2, Vec3};
use bevy_platform::collections::HashMap;
use bevy_render::{
    mesh::{Indices, Mesh, Mesh2d, PrimitiveTopology},
    render_resource::{
        Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    },
};
use tracing::warn;

mod tilemap_chunk_material;

pub use tilemap_chunk_material::*;

/// Plugin that handles the initialization and updating of tilemap chunks.
/// Adds systems for processing newly added tilemap chunks and updating their indices.
pub struct TilemapChunkPlugin;

impl Plugin for TilemapChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TilemapChunkMeshCache>()
            .add_observer(on_add_tilemap_chunk)
            .add_systems(Update, update_tilemap_chunk_indices);
    }
}

type TilemapChunkMeshCacheKey = (UVec2, FloatOrd, FloatOrd, FloatOrd, FloatOrd);

/// A resource storing the meshes for each tilemap chunk size.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct TilemapChunkMeshCache(HashMap<TilemapChunkMeshCacheKey, Handle<Mesh>>);

/// A component representing a chunk of a tilemap.
/// Each chunk is a rectangular section of tiles that is rendered as a single mesh.
#[derive(Component, Clone, Debug, Default)]
#[require(Mesh2d, MeshMaterial2d<TilemapChunkMaterial>, TilemapChunkIndices, Anchor)]
pub struct TilemapChunk {
    /// The size of the chunk in tiles
    pub chunk_size: UVec2,
    /// The size to use for each tile, not to be confused with the size of a tile in the tileset image.
    /// The size of the tile in the tileset image is determined by the tileset image's dimensions.
    pub tile_display_size: UVec2,
    /// Handle to the tileset image containing all tile textures
    pub tileset: Handle<Image>,
    /// The alpha mode to use for the tilemap chunk
    pub alpha_mode: AlphaMode2d,
}

/// Component storing the indices of tiles within a chunk.
/// Each index corresponds to a specific tile in the tileset.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut)]
pub struct TilemapChunkIndices(pub Vec<Option<u16>>);

fn on_add_tilemap_chunk(
    trigger: Trigger<OnAdd, TilemapChunk>,
    tilemap_chunk_query: Query<(&TilemapChunk, &TilemapChunkIndices, &Anchor)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TilemapChunkMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut tilemap_chunk_mesh_cache: ResMut<TilemapChunkMeshCache>,
) {
    let chunk_entity = trigger.target();
    let Ok((
        TilemapChunk {
            chunk_size,
            tile_display_size,
            tileset,
            alpha_mode,
        },
        indices,
        anchor,
    )) = tilemap_chunk_query.get(chunk_entity)
    else {
        warn!("Tilemap chunk {} not found", chunk_entity);
        return;
    };

    let expected_indices_length = chunk_size.element_product() as usize;
    if indices.len() != expected_indices_length {
        warn!(
            "Invalid indices length for tilemap chunk {} of size {}. Expected {}, got {}",
            chunk_entity,
            chunk_size,
            indices.len(),
            expected_indices_length
        );
        return;
    }

    let indices_image = make_chunk_image(chunk_size, &indices.0);

    let display_size = (chunk_size * tile_display_size).as_vec2();

    let mesh_key: TilemapChunkMeshCacheKey = (
        *chunk_size,
        FloatOrd(display_size.x),
        FloatOrd(display_size.y),
        FloatOrd(anchor.as_vec().x),
        FloatOrd(anchor.as_vec().y),
    );

    let mesh = tilemap_chunk_mesh_cache
        .entry(mesh_key)
        .or_insert_with(|| meshes.add(make_chunk_mesh(chunk_size, &display_size, anchor)));

    commands.entity(chunk_entity).insert((
        Mesh2d(mesh.clone()),
        MeshMaterial2d(materials.add(TilemapChunkMaterial {
            tileset: tileset.clone(),
            indices: images.add(indices_image),
            alpha_mode: *alpha_mode,
        })),
    ));
}

fn update_tilemap_chunk_indices(
    query: Query<
        (
            Entity,
            &TilemapChunk,
            &TilemapChunkIndices,
            &MeshMaterial2d<TilemapChunkMaterial>,
        ),
        Changed<TilemapChunkIndices>,
    >,
    mut materials: ResMut<Assets<TilemapChunkMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (chunk_entity, TilemapChunk { chunk_size, .. }, indices, material) in query {
        let expected_indices_length = chunk_size.element_product() as usize;
        if indices.len() != expected_indices_length {
            warn!(
                "Invalid TilemapChunkIndices length for tilemap chunk {} of size {}. Expected {}, got {}",
                chunk_entity,
                chunk_size,
                indices.len(),
                expected_indices_length
            );
            continue;
        }

        let Some(material) = materials.get_mut(material.id()) else {
            warn!(
                "TilemapChunkMaterial not found for tilemap chunk {}",
                chunk_entity
            );
            continue;
        };
        let Some(indices_image) = images.get_mut(&material.indices) else {
            warn!(
                "TilemapChunkMaterial indices image not found for tilemap chunk {}",
                chunk_entity
            );
            continue;
        };
        let Some(data) = indices_image.data.as_mut() else {
            warn!(
                "TilemapChunkMaterial indices image data not found for tilemap chunk {}",
                chunk_entity
            );
            continue;
        };
        data.clear();
        data.extend(
            indices
                .iter()
                .copied()
                .flat_map(|i| u16::to_ne_bytes(i.unwrap_or(u16::MAX))),
        );
    }
}

fn make_chunk_image(size: &UVec2, indices: &[Option<u16>]) -> Image {
    Image {
        data: Some(
            indices
                .iter()
                .copied()
                .flat_map(|i| u16::to_ne_bytes(i.unwrap_or(u16::MAX)))
                .collect(),
        ),
        texture_descriptor: TextureDescriptor {
            size: Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            dimension: TextureDimension::D2,
            format: TextureFormat::R16Uint,
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

fn make_chunk_mesh(size: &UVec2, display_size: &Vec2, anchor: &Anchor) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );

    let offset = display_size * (Vec2::splat(-0.5) - anchor.as_vec());

    let num_quads = size.element_product() as usize;
    let quad_size = display_size / size.as_vec2();

    let mut positions = Vec::with_capacity(4 * num_quads);
    let mut uvs = Vec::with_capacity(4 * num_quads);
    let mut indices = Vec::with_capacity(6 * num_quads);

    for y in 0..size.y {
        for x in 0..size.x {
            let i = positions.len() as u32;

            let p0 = offset + quad_size * UVec2::new(x, y).as_vec2();
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
