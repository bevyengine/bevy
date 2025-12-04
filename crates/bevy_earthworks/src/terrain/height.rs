//! Terrain height query utilities.

use bevy_ecs::prelude::*;
use bevy_math::IVec3;

use super::chunk::{Chunk, CHUNK_SIZE};
use super::voxel::VoxelTerrain;

/// Query the terrain height at a world position.
/// Returns the Y coordinate of the top of the highest solid voxel, or None if no terrain.
pub fn get_terrain_height(
    terrain: &VoxelTerrain,
    chunks: &Query<&Chunk>,
    world_x: f32,
    world_z: f32,
) -> Option<f32> {
    let voxel_size = terrain.voxel_size();
    if voxel_size <= 0.0 {
        return None;
    }

    // Convert to voxel coordinates
    let voxel_x = (world_x / voxel_size).floor() as i32;
    let voxel_z = (world_z / voxel_size).floor() as i32;

    // Search from top of world downward
    // Assume max height of 64 voxels (4 chunks vertically)
    let max_y = 64i32;
    for y in (0..max_y).rev() {
        let voxel_pos = IVec3::new(voxel_x, y, voxel_z);
        let chunk_coord = terrain.voxel_to_chunk(voxel_pos);

        if let Some(entity) = terrain.get_chunk_entity(chunk_coord) {
            if let Ok(chunk) = chunks.get(entity) {
                let local = terrain.voxel_to_local(voxel_pos);
                // Ensure local coordinates are within bounds
                if local.x >= 0
                    && local.x < CHUNK_SIZE as i32
                    && local.y >= 0
                    && local.y < CHUNK_SIZE as i32
                    && local.z >= 0
                    && local.z < CHUNK_SIZE as i32
                {
                    let voxel = chunk.get(local.x as usize, local.y as usize, local.z as usize);
                    if voxel.is_solid() {
                        // Return top of this voxel
                        return Some((y + 1) as f32 * voxel_size);
                    }
                }
            }
        }
    }

    None
}

/// Query terrain height with bilinear interpolation for smoother results.
/// Samples 4 corners and interpolates between them.
pub fn get_terrain_height_interpolated(
    terrain: &VoxelTerrain,
    chunks: &Query<&Chunk>,
    world_x: f32,
    world_z: f32,
) -> Option<f32> {
    let voxel_size = terrain.voxel_size();
    if voxel_size <= 0.0 {
        return None;
    }

    // Get the 4 corners of the cell containing this point
    let x0 = (world_x / voxel_size).floor() * voxel_size;
    let z0 = (world_z / voxel_size).floor() * voxel_size;
    let x1 = x0 + voxel_size;
    let z1 = z0 + voxel_size;

    // Sample heights at corners
    let h00 = get_terrain_height(terrain, chunks, x0, z0)?;
    let h10 = get_terrain_height(terrain, chunks, x1, z0)?;
    let h01 = get_terrain_height(terrain, chunks, x0, z1)?;
    let h11 = get_terrain_height(terrain, chunks, x1, z1)?;

    // Bilinear interpolation
    let fx = (world_x - x0) / voxel_size;
    let fz = (world_z - z0) / voxel_size;

    let h0 = h00 * (1.0 - fx) + h10 * fx;
    let h1 = h01 * (1.0 - fx) + h11 * fx;

    Some(h0 * (1.0 - fz) + h1 * fz)
}

#[cfg(test)]
mod tests {
    // Tests would require setting up terrain and chunks which is complex
    // Manual testing is more practical for this module
}
