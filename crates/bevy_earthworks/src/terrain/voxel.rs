//! Voxel and VoxelTerrain definitions.

use bevy_ecs::prelude::*;
use bevy_math::{IVec3, Vec3};
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

use super::chunk::{Chunk, ChunkCoord};
use super::materials::MaterialId;
use crate::config::EarthworksConfig;

/// The state of a single voxel.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum VoxelState {
    /// Empty space (air).
    #[default]
    Empty,
    /// Solid material that has not been disturbed.
    Solid,
    /// Material that has been excavated and redeposited.
    Disturbed,
}

impl VoxelState {
    /// Returns true if this voxel is solid (either natural or disturbed).
    pub fn is_solid(&self) -> bool {
        matches!(self, VoxelState::Solid | VoxelState::Disturbed)
    }

    /// Returns true if this voxel is empty.
    pub fn is_empty(&self) -> bool {
        matches!(self, VoxelState::Empty)
    }
}

/// A single voxel in the terrain.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Voxel {
    /// The state of this voxel.
    pub state: VoxelState,
    /// The material of this voxel (only relevant if solid).
    pub material: MaterialId,
}

impl Voxel {
    /// Creates a new empty voxel.
    pub const fn empty() -> Self {
        Self {
            state: VoxelState::Empty,
            material: MaterialId::Air,
        }
    }

    /// Creates a new solid voxel with the given material.
    pub const fn solid(material: MaterialId) -> Self {
        Self {
            state: VoxelState::Solid,
            material,
        }
    }

    /// Creates a new disturbed voxel with the given material.
    pub const fn disturbed(material: MaterialId) -> Self {
        Self {
            state: VoxelState::Disturbed,
            material,
        }
    }

    /// Returns true if this voxel is solid.
    pub fn is_solid(&self) -> bool {
        self.state.is_solid()
    }

    /// Returns true if this voxel is empty.
    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }
}

/// The world-level terrain resource managing all chunks.
#[derive(Resource, Default, Reflect)]
pub struct VoxelTerrain {
    /// Storage for all chunks, keyed by chunk coordinate.
    #[reflect(ignore)]
    chunks: bevy_platform::collections::HashMap<ChunkCoord, Entity>,
    /// Cached chunk size from config.
    chunk_size: u32,
    /// Cached voxel size from config.
    voxel_size: f32,
}

impl VoxelTerrain {
    /// Creates a new empty terrain with the given configuration.
    pub fn new(config: &EarthworksConfig) -> Self {
        Self {
            chunks: bevy_platform::collections::HashMap::default(),
            chunk_size: config.chunk_size,
            voxel_size: config.voxel_size,
        }
    }

    /// Gets the chunk size.
    pub fn chunk_size(&self) -> u32 {
        self.chunk_size
    }

    /// Gets the voxel size.
    pub fn voxel_size(&self) -> f32 {
        self.voxel_size
    }

    /// Converts a world position to a chunk coordinate.
    pub fn world_to_chunk(&self, world_pos: Vec3) -> ChunkCoord {
        let chunk_world_size = self.chunk_size as f32 * self.voxel_size;
        ChunkCoord {
            x: (world_pos.x / chunk_world_size).floor() as i32,
            y: (world_pos.y / chunk_world_size).floor() as i32,
            z: (world_pos.z / chunk_world_size).floor() as i32,
        }
    }

    /// Converts a world position to a voxel coordinate (global).
    pub fn world_to_voxel(&self, world_pos: Vec3) -> IVec3 {
        IVec3::new(
            (world_pos.x / self.voxel_size).floor() as i32,
            (world_pos.y / self.voxel_size).floor() as i32,
            (world_pos.z / self.voxel_size).floor() as i32,
        )
    }

    /// Converts a global voxel coordinate to a chunk coordinate.
    pub fn voxel_to_chunk(&self, voxel: IVec3) -> ChunkCoord {
        let size = self.chunk_size as i32;
        ChunkCoord {
            x: voxel.x.div_euclid(size),
            y: voxel.y.div_euclid(size),
            z: voxel.z.div_euclid(size),
        }
    }

    /// Converts a global voxel coordinate to a local coordinate within a chunk.
    pub fn voxel_to_local(&self, voxel: IVec3) -> IVec3 {
        let size = self.chunk_size as i32;
        IVec3::new(
            voxel.x.rem_euclid(size),
            voxel.y.rem_euclid(size),
            voxel.z.rem_euclid(size),
        )
    }

    /// Converts a chunk coordinate to a world position (chunk origin).
    pub fn chunk_to_world(&self, coord: ChunkCoord) -> Vec3 {
        let chunk_world_size = self.chunk_size as f32 * self.voxel_size;
        Vec3::new(
            coord.x as f32 * chunk_world_size,
            coord.y as f32 * chunk_world_size,
            coord.z as f32 * chunk_world_size,
        )
    }

    /// Gets the entity for a chunk at the given coordinate.
    pub fn get_chunk_entity(&self, coord: ChunkCoord) -> Option<Entity> {
        self.chunks.get(&coord).copied()
    }

    /// Sets the entity for a chunk at the given coordinate.
    pub fn set_chunk_entity(&mut self, coord: ChunkCoord, entity: Entity) {
        self.chunks.insert(coord, entity);
    }

    /// Removes the chunk entity at the given coordinate.
    pub fn remove_chunk_entity(&mut self, coord: ChunkCoord) -> Option<Entity> {
        self.chunks.remove(&coord)
    }

    /// Returns an iterator over all chunk coordinates.
    pub fn chunk_coords(&self) -> impl Iterator<Item = &ChunkCoord> {
        self.chunks.keys()
    }

    /// Returns the number of chunks.
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
    }

    /// Clears all chunks.
    pub fn clear(&mut self) {
        self.chunks.clear();
    }
}
