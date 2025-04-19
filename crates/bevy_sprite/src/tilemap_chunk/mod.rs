use crate::MeshMaterial2d;
use bevy_app::{App, Plugin, Update};
use bevy_asset::{Assets, Handle, RenderAssetUsages};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    observer::Trigger,
    query::Changed,
    system::{Commands, Query, ResMut},
    world::OnAdd,
};
use bevy_image::Image;
use bevy_math::{primitives::Rectangle, UVec2};
use bevy_render::{
    mesh::{Mesh, Mesh2d},
    render_resource::{Extent3d, TextureDimension, TextureFormat},
};

mod tilemap_chunk_material;

pub use tilemap_chunk_material::*;

/// Plugin that handles the initialization and updating of tilemap chunks.
/// Adds systems for processing newly added tilemap chunks and updating their indices.
pub struct TilemapChunkPlugin;

impl Plugin for TilemapChunkPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(on_add_tilemap_chunk)
            .add_systems(Update, update_tilemap_chunk_indices);
    }
}

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
    mut commands: Commands,
    mut query: Query<(&TilemapChunk, &mut TilemapChunkIndices)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TilemapChunkMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    let (
        TilemapChunk {
            chunk_size,
            tile_display_size,
            tileset,
        },
        mut indices,
    ) = query.get_mut(trigger.target()).unwrap();

    // Ensure the indices vec is the same size as the chunk
    indices.resize((chunk_size.x * chunk_size.y) as usize, None);

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

    commands.entity(trigger.target()).insert((
        Mesh2d(meshes.add(Rectangle::from_size(
            chunk_size.as_vec2() * tile_display_size.as_vec2(),
        ))),
        MeshMaterial2d(materials.add(TilemapChunkMaterial {
            tileset: tileset.clone(),
            indices: images.add(indices_image),
        })),
    ));
}

fn update_tilemap_chunk_indices(
    mut query: Query<
        (
            &TilemapChunk,
            &mut TilemapChunkIndices,
            &MeshMaterial2d<TilemapChunkMaterial>,
        ),
        Changed<TilemapChunkIndices>,
    >,
    mut materials: ResMut<Assets<TilemapChunkMaterial>>,
    mut images: ResMut<Assets<Image>>,
) {
    for (chunk, mut indices, material) in &mut query {
        indices.resize((chunk.chunk_size.x * chunk.chunk_size.y) as usize, None);
        let material = materials.get_mut(material.id()).unwrap();
        let indices_image = images.get_mut(&material.indices).unwrap();
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
