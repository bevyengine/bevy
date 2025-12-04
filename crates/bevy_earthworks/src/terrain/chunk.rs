//! Chunk data structure for storing voxels.

use bevy_ecs::prelude::*;
use bevy_math::IVec3;
use bevy_mesh::Mesh;
use bevy_reflect::Reflect;
use bevy_tasks::Task;
use serde::{Deserialize, Serialize};

use super::materials::MaterialId;
use super::voxel::Voxel;

/// The size of a chunk in voxels per dimension.
pub const CHUNK_SIZE: usize = 16;

/// Total number of voxels in a chunk.
pub const CHUNK_VOLUME: usize = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;

/// Coordinate of a chunk in the world.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Component, Serialize, Deserialize, Reflect,
)]
pub struct ChunkCoord {
    /// X coordinate of the chunk.
    pub x: i32,
    /// Y coordinate of the chunk.
    pub y: i32,
    /// Z coordinate of the chunk.
    pub z: i32,
}

impl ChunkCoord {
    /// Creates a new chunk coordinate.
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    /// Returns the chunk coordinate as an IVec3.
    pub fn as_ivec3(&self) -> IVec3 {
        IVec3::new(self.x, self.y, self.z)
    }
}

impl From<IVec3> for ChunkCoord {
    fn from(v: IVec3) -> Self {
        Self::new(v.x, v.y, v.z)
    }
}

impl From<ChunkCoord> for IVec3 {
    fn from(c: ChunkCoord) -> Self {
        c.as_ivec3()
    }
}

/// Marker component indicating a chunk needs its mesh regenerated.
#[derive(Component, Default, Reflect)]
pub struct DirtyChunk;

/// Component holding an async mesh generation task.
#[derive(Component)]
pub struct MeshTask(pub Task<Option<Mesh>>);

/// Level of detail for a chunk.
#[derive(Component, Default, Clone, Copy, Debug, Reflect)]
pub struct ChunkLOD {
    /// Current LOD level (0 = full detail, 3 = lowest).
    pub level: u8,
    /// Distance to camera.
    pub distance: f32,
}

impl ChunkLOD {
    /// LOD distance thresholds.
    pub const LOD_DISTANCES: [f32; 4] = [32.0, 64.0, 128.0, f32::MAX];

    /// Calculate LOD level from distance.
    pub fn from_distance(distance: f32) -> Self {
        let level = if distance < Self::LOD_DISTANCES[0] {
            0
        } else if distance < Self::LOD_DISTANCES[1] {
            1
        } else if distance < Self::LOD_DISTANCES[2] {
            2
        } else {
            3
        };
        Self { level, distance }
    }
}

/// A 16x16x16 container of voxels.
#[derive(Clone, Component)]
pub struct Chunk {
    /// Voxel data stored in a flat array.
    /// Index = x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
    voxels: Box<[Voxel; CHUNK_VOLUME]>,
    /// Number of solid voxels in this chunk (for quick empty checks).
    solid_count: u32,
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl Chunk {
    /// Creates a new empty chunk.
    pub fn new() -> Self {
        Self {
            voxels: Box::new([Voxel::empty(); CHUNK_VOLUME]),
            solid_count: 0,
        }
    }

    /// Creates a chunk filled with the given material.
    pub fn filled(material: MaterialId) -> Self {
        let voxel = Voxel::solid(material);
        Self {
            voxels: Box::new([voxel; CHUNK_VOLUME]),
            solid_count: CHUNK_VOLUME as u32,
        }
    }

    /// Converts a local 3D coordinate to a flat array index.
    #[inline]
    pub fn coord_to_index(x: usize, y: usize, z: usize) -> usize {
        debug_assert!(x < CHUNK_SIZE && y < CHUNK_SIZE && z < CHUNK_SIZE);
        x + y * CHUNK_SIZE + z * CHUNK_SIZE * CHUNK_SIZE
    }

    /// Converts a flat array index to a local 3D coordinate.
    #[inline]
    pub fn index_to_coord(index: usize) -> (usize, usize, usize) {
        debug_assert!(index < CHUNK_VOLUME);
        let x = index % CHUNK_SIZE;
        let y = (index / CHUNK_SIZE) % CHUNK_SIZE;
        let z = index / (CHUNK_SIZE * CHUNK_SIZE);
        (x, y, z)
    }

    /// Gets the voxel at the given local coordinate.
    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> Voxel {
        self.voxels[Self::coord_to_index(x, y, z)]
    }

    /// Gets the voxel at the given local coordinate (IVec3 version).
    #[inline]
    pub fn get_ivec3(&self, pos: IVec3) -> Voxel {
        self.get(pos.x as usize, pos.y as usize, pos.z as usize)
    }

    /// Sets the voxel at the given local coordinate, returning the old voxel.
    #[inline]
    pub fn set(&mut self, x: usize, y: usize, z: usize, voxel: Voxel) -> Voxel {
        let index = Self::coord_to_index(x, y, z);
        let old = self.voxels[index];

        // Update solid count
        if old.is_solid() && !voxel.is_solid() {
            self.solid_count = self.solid_count.saturating_sub(1);
        } else if !old.is_solid() && voxel.is_solid() {
            self.solid_count += 1;
        }

        self.voxels[index] = voxel;
        old
    }

    /// Sets the voxel at the given local coordinate (IVec3 version).
    #[inline]
    pub fn set_ivec3(&mut self, pos: IVec3, voxel: Voxel) -> Voxel {
        self.set(pos.x as usize, pos.y as usize, pos.z as usize, voxel)
    }

    /// Returns true if this chunk is completely empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.solid_count == 0
    }

    /// Returns the number of solid voxels in this chunk.
    #[inline]
    pub fn solid_count(&self) -> u32 {
        self.solid_count
    }

    /// Returns an iterator over all voxels with their local coordinates.
    pub fn iter(&self) -> impl Iterator<Item = (usize, usize, usize, &Voxel)> {
        self.voxels.iter().enumerate().map(|(i, v)| {
            let (x, y, z) = Self::index_to_coord(i);
            (x, y, z, v)
        })
    }

    /// Returns a mutable iterator over all voxels with their local coordinates.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, usize, usize, &mut Voxel)> {
        self.voxels.iter_mut().enumerate().map(|(i, v)| {
            let (x, y, z) = Self::index_to_coord(i);
            (x, y, z, v)
        })
    }

    /// Gets direct access to the voxel array for bulk operations.
    pub fn voxels(&self) -> &[Voxel; CHUNK_VOLUME] {
        &self.voxels
    }

    /// Gets mutable access to the voxel array for bulk operations.
    /// Note: This bypasses solid_count tracking!
    pub fn voxels_mut(&mut self) -> &mut [Voxel; CHUNK_VOLUME] {
        &mut self.voxels
    }

    /// Recalculates the solid count from the voxel data.
    pub fn recalculate_solid_count(&mut self) {
        self.solid_count = self.voxels.iter().filter(|v| v.is_solid()).count() as u32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_coord_to_index() {
        assert_eq!(Chunk::coord_to_index(0, 0, 0), 0);
        assert_eq!(Chunk::coord_to_index(1, 0, 0), 1);
        assert_eq!(Chunk::coord_to_index(0, 1, 0), CHUNK_SIZE);
        assert_eq!(Chunk::coord_to_index(0, 0, 1), CHUNK_SIZE * CHUNK_SIZE);
        assert_eq!(
            Chunk::coord_to_index(CHUNK_SIZE - 1, CHUNK_SIZE - 1, CHUNK_SIZE - 1),
            CHUNK_VOLUME - 1
        );
    }

    #[test]
    fn test_chunk_index_to_coord() {
        assert_eq!(Chunk::index_to_coord(0), (0, 0, 0));
        assert_eq!(Chunk::index_to_coord(1), (1, 0, 0));
        assert_eq!(Chunk::index_to_coord(CHUNK_SIZE), (0, 1, 0));
        assert_eq!(Chunk::index_to_coord(CHUNK_SIZE * CHUNK_SIZE), (0, 0, 1));
    }

    #[test]
    fn test_chunk_get_set() {
        let mut chunk = Chunk::new();
        assert!(chunk.is_empty());
        assert_eq!(chunk.solid_count(), 0);

        let voxel = Voxel::solid(MaterialId::Dirt);
        chunk.set(5, 5, 5, voxel);

        assert!(!chunk.is_empty());
        assert_eq!(chunk.solid_count(), 1);
        assert_eq!(chunk.get(5, 5, 5), voxel);
    }

    #[test]
    fn test_chunk_filled() {
        let chunk = Chunk::filled(MaterialId::Dirt);
        assert!(!chunk.is_empty());
        assert_eq!(chunk.solid_count(), CHUNK_VOLUME as u32);
    }
}
