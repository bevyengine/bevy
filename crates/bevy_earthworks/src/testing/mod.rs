//! Testing infrastructure for bevy_earthworks.
//!
//! This module provides:
//! - Headless test framework for CI/automated testing
//! - AI gameplay agent that can autonomously play the game
//! - Test utilities for setting up test scenarios

mod ai_agent;

pub use ai_agent::{AIAgent, AgentBehavior, AgentGoal, AIAgentPlugin};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

use crate::terrain::{Chunk, ChunkCoord, VoxelTerrain};

/// Plugin for testing systems.
pub struct TestingPlugin;

impl Plugin for TestingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(AIAgentPlugin);
    }
}

/// Configuration for headless testing.
#[derive(Resource, Clone, Debug)]
pub struct HeadlessTestConfig {
    /// Maximum frames to run before timing out.
    pub max_frames: u32,
    /// Current frame count.
    pub frame_count: u32,
    /// Whether to enable verbose logging.
    pub verbose: bool,
}

impl Default for HeadlessTestConfig {
    fn default() -> Self {
        Self {
            max_frames: 300, // 5 seconds at 60fps
            frame_count: 0,
            verbose: false,
        }
    }
}

/// Test result data.
#[derive(Clone, Debug)]
pub struct TestResult {
    /// Whether the test passed.
    pub passed: bool,
    /// Number of frames run.
    pub frames_run: u32,
    /// Optional error message.
    pub error: Option<String>,
    /// Additional metrics.
    pub metrics: TestMetrics,
}

/// Metrics collected during testing.
#[derive(Clone, Debug, Default)]
pub struct TestMetrics {
    /// Total voxels excavated.
    pub voxels_excavated: u32,
    /// Total voxels filled.
    pub voxels_filled: u32,
    /// Distance traveled by machine.
    pub distance_traveled: f32,
    /// Jobs completed.
    pub jobs_completed: u32,
    /// Zyns earned.
    pub zyns_earned: u32,
}

/// Helper function to get terrain height at a voxel position.
/// Returns the height in voxel units (integer).
pub fn get_height_at(terrain: &VoxelTerrain, chunks: &Query<&Chunk>, voxel_x: i32, voxel_z: i32) -> Option<i32> {
    use crate::terrain::get_terrain_height;
    let voxel_size = terrain.voxel_size();
    let world_x = voxel_x as f32 * voxel_size;
    let world_z = voxel_z as f32 * voxel_size;
    get_terrain_height(terrain, chunks, world_x, world_z)
        .map(|h| (h / voxel_size) as i32)
}

/// Helper to check if an area is leveled to target height.
pub fn is_area_leveled(
    terrain: &VoxelTerrain,
    chunks: &Query<&Chunk>,
    min_x: i32,
    max_x: i32,
    min_z: i32,
    max_z: i32,
    target_height: i32,
    tolerance: i32,
) -> bool {
    for x in min_x..=max_x {
        for z in min_z..=max_z {
            if let Some(height) = get_height_at(terrain, chunks, x, z) {
                if (height - target_height).abs() > tolerance {
                    return false;
                }
            }
        }
    }
    true
}

/// Helper to count solid voxels in an area.
pub fn count_solid_voxels(
    terrain: &VoxelTerrain,
    chunks: &Query<&Chunk>,
    min_x: i32,
    max_x: i32,
    min_y: i32,
    max_y: i32,
    min_z: i32,
    max_z: i32,
) -> u32 {
    use bevy_math::IVec3;

    let mut count = 0;
    for x in min_x..=max_x {
        for y in min_y..=max_y {
            for z in min_z..=max_z {
                let pos = IVec3::new(x, y, z);
                let chunk_coord = terrain.voxel_to_chunk(pos);
                if let Some(entity) = terrain.get_chunk_entity(chunk_coord) {
                    if let Ok(chunk) = chunks.get(entity) {
                        let local = terrain.voxel_to_local(pos);
                        if local.x >= 0 && (local.x as usize) < 16
                            && local.y >= 0 && (local.y as usize) < 16
                            && local.z >= 0 && (local.z as usize) < 16
                        {
                            let voxel = chunk.get(
                                local.x as usize,
                                local.y as usize,
                                local.z as usize,
                            );
                            if voxel.is_solid() {
                                count += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    count
}
