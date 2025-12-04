//! Terrain modification operations.

use bevy_ecs::prelude::*;
use bevy_math::{IVec3, Vec3};
use bevy_reflect::Reflect;

use super::chunk::{Chunk, ChunkCoord, DirtyChunk, CHUNK_SIZE};
use super::materials::MaterialId;
use super::voxel::{Voxel, VoxelState, VoxelTerrain};

/// Event emitted when terrain is modified.
#[derive(Message, Clone, Debug, Reflect)]
pub struct TerrainModifiedEvent {
    /// The chunk that was modified.
    pub chunk_coord: ChunkCoord,
    /// Type of operation performed.
    pub operation: TerrainOperation,
    /// Volume changed (in voxels, positive for fill, negative for excavate).
    pub volume_changed: i32,
}

/// Type of terrain modification operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Reflect)]
pub enum TerrainOperation {
    /// Material was removed.
    Excavate,
    /// Material was added.
    Fill,
}

/// An axis-aligned bounding box for terrain operations.
#[derive(Clone, Copy, Debug)]
pub struct Aabb {
    /// Minimum corner (inclusive).
    pub min: IVec3,
    /// Maximum corner (inclusive).
    pub max: IVec3,
}

impl Aabb {
    /// Creates a new AABB from min and max corners.
    pub fn new(min: IVec3, max: IVec3) -> Self {
        Self {
            min: min.min(max),
            max: min.max(max),
        }
    }

    /// Creates an AABB from a center point and half-extents.
    pub fn from_center_half_extents(center: IVec3, half_extents: IVec3) -> Self {
        Self {
            min: center - half_extents,
            max: center + half_extents,
        }
    }

    /// Returns the volume of this AABB in voxels.
    pub fn volume(&self) -> i32 {
        let size = self.max - self.min + IVec3::ONE;
        size.x * size.y * size.z
    }

    /// Returns true if this AABB contains the given point.
    pub fn contains(&self, point: IVec3) -> bool {
        point.x >= self.min.x
            && point.x <= self.max.x
            && point.y >= self.min.y
            && point.y <= self.max.y
            && point.z >= self.min.z
            && point.z <= self.max.z
    }

    /// Iterates over all voxel coordinates within this AABB.
    pub fn iter(&self) -> impl Iterator<Item = IVec3> {
        let min = self.min;
        let max = self.max;
        (min.z..=max.z).flat_map(move |z| {
            (min.y..=max.y).flat_map(move |y| (min.x..=max.x).map(move |x| IVec3::new(x, y, z)))
        })
    }
}

/// Excavates voxels within the given AABB.
///
/// Returns the number of voxels that were actually excavated.
pub fn excavate(
    commands: &mut Commands,
    terrain: &mut VoxelTerrain,
    chunks: &mut Query<&mut Chunk>,
    bounds: Aabb,
    events: &mut MessageWriter<TerrainModifiedEvent>,
) -> i32 {
    let mut total_excavated = 0i32;
    let mut affected_chunks: bevy_platform::collections::HashMap<ChunkCoord, i32> =
        bevy_platform::collections::HashMap::default();

    for voxel_pos in bounds.iter() {
        let chunk_coord = terrain.voxel_to_chunk(voxel_pos);
        let local_pos = terrain.voxel_to_local(voxel_pos);

        if let Some(entity) = terrain.get_chunk_entity(chunk_coord) {
            if let Ok(mut chunk) = chunks.get_mut(entity) {
                let old = chunk.get_ivec3(local_pos);
                if old.is_solid() {
                    chunk.set_ivec3(local_pos, Voxel::empty());
                    total_excavated += 1;
                    *affected_chunks.entry(chunk_coord).or_default() += 1;

                    // Mark chunk as dirty
                    commands.entity(entity).insert(DirtyChunk);
                }
            }
        }
    }

    // Emit events for affected chunks
    for (chunk_coord, volume) in affected_chunks {
        events.write(TerrainModifiedEvent {
            chunk_coord,
            operation: TerrainOperation::Excavate,
            volume_changed: -volume,
        });
    }

    total_excavated
}

/// Fills voxels within the given AABB with the specified material.
///
/// Returns the number of voxels that were actually filled.
pub fn fill(
    commands: &mut Commands,
    terrain: &mut VoxelTerrain,
    chunks: &mut Query<&mut Chunk>,
    bounds: Aabb,
    material: MaterialId,
    disturbed: bool,
    events: &mut MessageWriter<TerrainModifiedEvent>,
) -> i32 {
    let mut total_filled = 0i32;
    let mut affected_chunks: bevy_platform::collections::HashMap<ChunkCoord, i32> =
        bevy_platform::collections::HashMap::default();

    let voxel = if disturbed {
        Voxel::disturbed(material)
    } else {
        Voxel::solid(material)
    };

    for voxel_pos in bounds.iter() {
        let chunk_coord = terrain.voxel_to_chunk(voxel_pos);
        let local_pos = terrain.voxel_to_local(voxel_pos);

        // Get or create chunk
        let entity = if let Some(entity) = terrain.get_chunk_entity(chunk_coord) {
            entity
        } else {
            // Create new chunk
            let new_entity = commands.spawn((Chunk::new(), chunk_coord, DirtyChunk)).id();
            terrain.set_chunk_entity(chunk_coord, new_entity);
            new_entity
        };

        if let Ok(mut chunk) = chunks.get_mut(entity) {
            let old = chunk.get_ivec3(local_pos);
            if old.is_empty() {
                chunk.set_ivec3(local_pos, voxel);
                total_filled += 1;
                *affected_chunks.entry(chunk_coord).or_default() += 1;

                // Mark chunk as dirty
                commands.entity(entity).insert(DirtyChunk);
            }
        }
    }

    // Emit events for affected chunks
    for (chunk_coord, volume) in affected_chunks {
        events.write(TerrainModifiedEvent {
            chunk_coord,
            operation: TerrainOperation::Fill,
            volume_changed: volume,
        });
    }

    total_filled
}

/// Returns the set of chunk coordinates that would be affected by an operation on the given AABB.
pub fn get_affected_chunks(terrain: &VoxelTerrain, bounds: &Aabb) -> Vec<ChunkCoord> {
    let chunk_size = terrain.chunk_size() as i32;

    let min_chunk = terrain.voxel_to_chunk(bounds.min);
    let max_chunk = terrain.voxel_to_chunk(bounds.max);

    let mut chunks = Vec::new();
    for z in min_chunk.z..=max_chunk.z {
        for y in min_chunk.y..=max_chunk.y {
            for x in min_chunk.x..=max_chunk.x {
                chunks.push(ChunkCoord::new(x, y, z));
            }
        }
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aabb_volume() {
        let aabb = Aabb::new(IVec3::ZERO, IVec3::new(2, 2, 2));
        assert_eq!(aabb.volume(), 27); // 3x3x3
    }

    #[test]
    fn test_aabb_contains() {
        let aabb = Aabb::new(IVec3::ZERO, IVec3::new(10, 10, 10));
        assert!(aabb.contains(IVec3::new(5, 5, 5)));
        assert!(aabb.contains(IVec3::ZERO));
        assert!(aabb.contains(IVec3::new(10, 10, 10)));
        assert!(!aabb.contains(IVec3::new(11, 5, 5)));
        assert!(!aabb.contains(IVec3::new(-1, 5, 5)));
    }

    #[test]
    fn test_aabb_iter() {
        let aabb = Aabb::new(IVec3::ZERO, IVec3::new(1, 1, 1));
        let points: Vec<_> = aabb.iter().collect();
        assert_eq!(points.len(), 8); // 2x2x2
    }
}
