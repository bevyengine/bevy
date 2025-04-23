use crate::MeshMaterial2d;
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
use bevy_image::Image;
use bevy_math::{primitives::Rectangle, UVec2};
use bevy_platform::collections::HashMap;
use bevy_render::{
    mesh::{Mesh, Mesh2d},
    render_resource::{Extent3d, TextureDimension, TextureFormat},
};
use tracing::warn;

mod tilemap_chunk_material;

pub use tilemap_chunk_material::*;

/// Plugin that handles the initialization and updating of tilemap chunks.
/// Adds systems for processing newly added tilemap chunks and updating their indices.
pub struct TilemapChunkPlugin;

impl Plugin for TilemapChunkPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<TilemapChunkMeshMap>()
            .add_observer(on_add_tilemap_chunk)
            .add_systems(Update, update_tilemap_chunk_indices);
    }
}

/// A resource storing the meshes for each tilemap chunk size.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct TilemapChunkMeshMap(HashMap<UVec2, Handle<Mesh>>);

/// A component representing a chunk of a tilemap.
/// Each chunk is a rectangular section of tiles that is rendered as a single mesh.
#[derive(Component, Clone, Debug, Default)]
#[require(Mesh2d, MeshMaterial2d<TilemapChunkMaterial>, TilemapChunkIndices)]
pub struct TilemapChunk {
    /// The size of the chunk in tiles
    pub chunk_size: UVec2,
    /// The size to use for each tile, not to be confused with the size of a tile in the tileset image.
    /// The size of the tile in the tileset image is determined by the tileset image's dimensions.
    pub tile_display_size: UVec2,
    /// Handle to the tileset image containing all tile textures
    pub tileset: Handle<Image>,
}

/// Component storing the indices of tiles within a chunk.
/// Each index corresponds to a specific tile in the tileset.
#[derive(Component, Clone, Debug, Default, Deref, DerefMut)]
pub struct TilemapChunkIndices(pub Vec<Option<u32>>);

fn on_add_tilemap_chunk(
    trigger: Trigger<OnAdd, TilemapChunk>,
    tilemap_chunk_query: Query<(&TilemapChunk, &TilemapChunkIndices)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TilemapChunkMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut tilemap_chunk_meshes: ResMut<TilemapChunkMeshMap>,
) {
    let chunk_entity = trigger.target();
    let Ok((
        TilemapChunk {
            chunk_size,
            tile_display_size,
            tileset,
        },
        indices,
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

    let indices_image = Image::new(
        Extent3d {
            width: chunk_size.x,
            height: chunk_size.y,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        indices_image_data(indices.0.clone()),
        TextureFormat::R32Uint,
        RenderAssetUsages::default(),
    );

    let mesh_size = chunk_size * tile_display_size;

    let mesh = tilemap_chunk_meshes
        .entry(mesh_size)
        .or_insert_with(|| meshes.add(Rectangle::from_size(mesh_size.as_vec2())));

    commands.entity(chunk_entity).insert((
        Mesh2d(mesh.clone()),
        MeshMaterial2d(materials.add(TilemapChunkMaterial {
            tileset: tileset.clone(),
            indices: images.add(indices_image),
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
        indices_image.data = Some(indices_image_data(indices.0.clone()));
    }
}

fn indices_image_data(indices: Vec<Option<u32>>) -> Vec<u8> {
    bytemuck::cast_slice(
        &indices
            .into_iter()
            .map(|i| i.unwrap_or(u32::MAX))
            .collect::<Vec<u32>>(),
    )
    .to_owned()
}
