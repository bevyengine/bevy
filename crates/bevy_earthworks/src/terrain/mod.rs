//! Voxel terrain system for volumetric earth simulation.
//!
//! This module provides:
//! - [`Voxel`] - Individual voxel data with state and material
//! - [`Chunk`] - 16x16x16 voxel container for efficient storage
//! - [`VoxelTerrain`] - World-level terrain resource managing chunks
//! - Terrain operations: excavate, fill, query
//! - Mesh generation for rendering

mod chunk;
mod materials;
mod meshing;
pub mod operations;
mod voxel;

pub use chunk::{Chunk, ChunkCoord, DirtyChunk};
pub use materials::MaterialId;
pub use meshing::{generate_chunk_mesh, ChunkMesh};
pub use operations::TerrainModifiedEvent;
pub use voxel::{Voxel, VoxelState, VoxelTerrain};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

/// Plugin for terrain systems.
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelTerrain>()
            .add_message::<TerrainModifiedEvent>()
            .add_systems(Update, (chunk_dirty_system, mesh_generation_system).chain());
    }
}

/// System that marks chunks as dirty when their voxel data changes.
fn chunk_dirty_system(
    mut commands: Commands,
    changed_chunks: Query<Entity, (Changed<Chunk>, Without<DirtyChunk>)>,
) {
    for entity in changed_chunks.iter() {
        commands.entity(entity).insert(DirtyChunk);
    }
}

/// System that generates meshes for dirty chunks.
fn mesh_generation_system(
    mut commands: Commands,
    config: Res<crate::config::EarthworksConfig>,
    mut dirty_chunks: Query<(Entity, &Chunk, &ChunkCoord), With<DirtyChunk>>,
    mut meshes: ResMut<bevy_asset::Assets<bevy_mesh::Mesh>>,
    mut materials: ResMut<bevy_asset::Assets<bevy_pbr::StandardMaterial>>,
) {
    let mut processed = 0;
    for (entity, chunk, coord) in dirty_chunks.iter_mut() {
        if processed >= config.max_meshes_per_frame {
            break;
        }

        // Generate mesh for this chunk
        if let Some(mesh) = generate_chunk_mesh(chunk, config.voxel_size) {
            let mesh_handle = meshes.add(mesh);
            let material_handle = materials.add(bevy_pbr::StandardMaterial {
                base_color: bevy_color::Color::srgb(0.5, 0.4, 0.3),
                ..Default::default()
            });

            let transform = bevy_transform::components::Transform::from_translation(
                bevy_math::Vec3::new(
                    coord.x as f32 * config.chunk_size as f32 * config.voxel_size,
                    coord.y as f32 * config.chunk_size as f32 * config.voxel_size,
                    coord.z as f32 * config.chunk_size as f32 * config.voxel_size,
                ),
            );

            commands
                .entity(entity)
                .insert((
                    bevy_mesh::Mesh3d(mesh_handle),
                    bevy_pbr::MeshMaterial3d(material_handle),
                    transform,
                ))
                .remove::<DirtyChunk>();
        } else {
            // Empty chunk, just remove dirty marker
            commands.entity(entity).remove::<DirtyChunk>();
        }

        processed += 1;
    }
}
