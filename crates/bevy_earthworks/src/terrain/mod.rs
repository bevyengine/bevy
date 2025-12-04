//! Voxel terrain system for volumetric earth simulation.
//!
//! This module provides:
//! - [`Voxel`] - Individual voxel data with state and material
//! - [`Chunk`] - 16x16x16 voxel container for efficient storage
//! - [`VoxelTerrain`] - World-level terrain resource managing chunks
//! - Terrain operations: excavate, fill, query
//! - Mesh generation for rendering
//! - Async mesh generation for non-blocking performance
//! - LOD system for distant chunk optimization

mod chunk;
mod height;
mod materials;
mod meshing;
pub mod operations;
mod textures;
mod voxel;

pub use chunk::{Chunk, ChunkCoord, ChunkLOD, DirtyChunk, MeshTask, CHUNK_SIZE};
pub use height::{get_terrain_height, get_terrain_height_interpolated};
pub use materials::MaterialId;
pub use meshing::{
    generate_chunk_mesh, generate_chunk_mesh_greedy, generate_chunk_mesh_greedy_with_atlas,
    AtlasUvConfig, ChunkMesh, MaterialAtlasUv,
};
pub use operations::{excavate, fill, Aabb, TerrainModifiedEvent};
pub use textures::{AtlasRegion, TerrainTextureAtlas, TerrainMaterialTexture, create_terrain_material};
pub use voxel::{Voxel, VoxelState, VoxelTerrain};

use bevy_app::prelude::*;
use bevy_camera::prelude::*;
use bevy_ecs::prelude::*;
use bevy_tasks::{futures_lite::future, AsyncComputeTaskPool};

/// Plugin for terrain systems.
pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<VoxelTerrain>()
            .register_type::<ChunkLOD>()
            .add_message::<TerrainModifiedEvent>()
            .add_systems(
                Update,
                (
                    chunk_dirty_system,
                    update_chunk_lod,
                    queue_async_mesh_generation,
                    poll_mesh_tasks,
                )
                    .chain(),
            );
    }
}

/// System that marks chunks as dirty when their voxel data changes.
fn chunk_dirty_system(
    mut commands: Commands,
    changed_chunks: Query<Entity, (Changed<Chunk>, Without<DirtyChunk>, Without<MeshTask>)>,
) {
    for entity in changed_chunks.iter() {
        commands.entity(entity).insert(DirtyChunk);
    }
}

/// Update LOD levels for chunks based on camera distance.
fn update_chunk_lod(
    camera: Query<&bevy_transform::components::Transform, With<Camera3d>>,
    mut chunks: Query<(&bevy_transform::components::Transform, &mut ChunkLOD), With<Chunk>>,
) {
    let Ok(cam_transform) = camera.single() else {
        return;
    };
    let cam_pos = cam_transform.translation;

    for (chunk_transform, mut lod) in chunks.iter_mut() {
        let distance = chunk_transform.translation.distance(cam_pos);
        let new_lod = ChunkLOD::from_distance(distance);

        // Only update if LOD level changed significantly
        if new_lod.level != lod.level {
            *lod = new_lod;
        } else {
            lod.distance = distance;
        }
    }
}

/// Queue async mesh generation for dirty chunks.
fn queue_async_mesh_generation(
    mut commands: Commands,
    config: Res<crate::config::EarthworksConfig>,
    dirty_chunks: Query<(Entity, &Chunk), (With<DirtyChunk>, Without<MeshTask>)>,
    texture_atlas: Option<Res<TerrainTextureAtlas>>,
) {
    let task_pool = AsyncComputeTaskPool::get();
    let voxel_size = config.voxel_size;

    // Get atlas UV config if available
    let atlas_config = texture_atlas
        .as_ref()
        .filter(|a| a.ready)
        .map(|a| a.to_uv_config());

    // Limit how many we queue per frame to prevent hitches
    let max_queue = config.max_meshes_per_frame as usize;
    let mut queued = 0;

    for (entity, chunk) in dirty_chunks.iter() {
        if queued >= max_queue {
            break;
        }

        // Clone voxel data for async processing
        let voxels = chunk.voxels().clone();
        let is_empty = chunk.is_empty();
        let atlas_config_clone = atlas_config.clone();

        // Spawn async task
        let task = task_pool.spawn(async move {
            if is_empty {
                return None;
            }

            // Reconstruct chunk from voxels for mesh generation
            let mut temp_chunk = Chunk::new();
            for (i, voxel) in voxels.iter().enumerate() {
                let (x, y, z) = Chunk::index_to_coord(i);
                temp_chunk.set(x, y, z, *voxel);
            }

            generate_chunk_mesh_greedy_with_atlas(
                &temp_chunk,
                voxel_size,
                atlas_config_clone.as_ref(),
            )
        });

        commands
            .entity(entity)
            .insert(MeshTask(task))
            .remove::<DirtyChunk>();

        queued += 1;
    }
}

/// Shared material handle for terrain chunks (vertex-colored).
#[derive(Resource)]
struct TerrainMaterial(bevy_asset::Handle<bevy_pbr::StandardMaterial>);

/// Shared material handle for textured terrain chunks.
#[derive(Resource)]
struct TexturedTerrainMaterial(bevy_asset::Handle<bevy_pbr::StandardMaterial>);

/// Poll completed mesh tasks and apply results.
fn poll_mesh_tasks(
    mut commands: Commands,
    config: Res<crate::config::EarthworksConfig>,
    mut mesh_tasks: Query<(Entity, &mut MeshTask, &ChunkCoord)>,
    mut meshes: ResMut<bevy_asset::Assets<bevy_mesh::Mesh>>,
    mut materials: ResMut<bevy_asset::Assets<bevy_pbr::StandardMaterial>>,
    terrain_material: Option<Res<TerrainMaterial>>,
    textured_material: Option<Res<TexturedTerrainMaterial>>,
    texture_atlas: Option<Res<TerrainTextureAtlas>>,
) {
    // Check if we should use textured material
    let use_textured = texture_atlas
        .as_ref()
        .map(|a| a.ready && a.albedo_atlas.is_some())
        .unwrap_or(false);

    // Get or create the appropriate material
    let material_handle = if use_textured {
        // Use textured material with atlas
        if let Some(mat) = textured_material {
            mat.0.clone()
        } else {
            // Create textured material from atlas
            let atlas = texture_atlas.as_ref().unwrap();
            let handle = create_terrain_material(&mut materials, atlas);
            commands.insert_resource(TexturedTerrainMaterial(handle.clone()));
            handle
        }
    } else if let Some(mat) = terrain_material {
        mat.0.clone()
    } else {
        // Create a material that uses vertex colors with improved shading
        let handle = materials.add(bevy_pbr::StandardMaterial {
            base_color: bevy_color::Color::WHITE, // White base so vertex colors show through
            perceptual_roughness: 0.92, // Very rough for natural earth look
            metallic: 0.0,
            reflectance: 0.3, // Low reflectance for dirt/earth
            diffuse_transmission: 0.0,
            specular_transmission: 0.0,
            ..Default::default()
        });
        commands.insert_resource(TerrainMaterial(handle.clone()));
        handle
    };

    for (entity, mut task, coord) in mesh_tasks.iter_mut() {
        // Check if task is complete (non-blocking)
        if let Some(mesh_result) = future::block_on(future::poll_once(&mut task.0)) {
            // Task complete, apply result
            if let Some(mesh) = mesh_result {
                let mesh_handle = meshes.add(mesh);

                let transform = bevy_transform::components::Transform::from_translation(
                    bevy_math::Vec3::new(
                        coord.x as f32 * config.chunk_size as f32 * config.voxel_size,
                        coord.y as f32 * config.chunk_size as f32 * config.voxel_size,
                        coord.z as f32 * config.chunk_size as f32 * config.voxel_size,
                    ),
                );

                commands.entity(entity).insert((
                    bevy_mesh::Mesh3d(mesh_handle),
                    bevy_pbr::MeshMaterial3d(material_handle.clone()),
                    transform,
                    ChunkLOD::default(),
                ));
            }

            // Remove task component
            commands.entity(entity).remove::<MeshTask>();
        }
    }
}
