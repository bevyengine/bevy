//! Plan execution system for earthworks operations.
//!
//! This module provides:
//! - [`ExecutionPlan`] - Asset type for pre-computed earthworks plans
//! - [`PlanPlayback`] - Resource for controlling plan execution
//! - [`PlanStep`] - Individual steps in an execution plan
//! - Plan loading from JSON files

mod executor;
mod loader;
mod playback;
mod schema;

pub use executor::plan_executor_system;
pub use loader::PlanLoader;
pub use playback::PlanPlayback;
pub use schema::{ExecutionPlan, PlanMetadata, PlanStep, PlannedAction, SiteDefinition};

use bevy_app::prelude::*;
use bevy_asset::AssetApp;
use bevy_ecs::prelude::*;

/// Plugin for plan execution systems.
pub struct PlanPlugin;

impl Plugin for PlanPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<ExecutionPlan>()
            .init_asset_loader::<PlanLoader>()
            .init_resource::<PlanPlayback>()
            .add_message::<PlanStepEvent>()
            .add_systems(Update, (plan_executor_system, plan_loader_system));
    }
}

/// Event emitted when a plan step is triggered.
#[derive(Message, Clone, Debug)]
pub struct PlanStepEvent {
    /// Index of the step in the plan.
    pub step_index: usize,
    /// The step that was triggered.
    pub step: PlanStep,
    /// Result of the step execution.
    pub result: StepResult,
}

/// Result of a plan step execution.
#[derive(Clone, Debug)]
pub enum StepResult {
    /// Step executed successfully.
    Success,
    /// Step was skipped.
    Skipped,
    /// Step failed with an error.
    Failed(String),
}

/// System that handles loading a new plan.
fn plan_loader_system(
    mut commands: Commands,
    plans: Res<bevy_asset::Assets<ExecutionPlan>>,
    mut playback: ResMut<PlanPlayback>,
    mut terrain: ResMut<crate::terrain::VoxelTerrain>,
    config: Res<crate::config::EarthworksConfig>,
    existing_chunks: Query<Entity, With<crate::terrain::Chunk>>,
    existing_machines: Query<Entity, With<crate::machines::Machine>>,
) {
    // Check if we have a new plan to load
    let Some(plan_handle) = playback.plan_handle() else {
        return;
    };

    if !playback.needs_reload() {
        return;
    }

    let Some(plan) = plans.get(plan_handle) else {
        return;
    };

    // Clear existing entities
    for entity in existing_chunks.iter() {
        commands.entity(entity).despawn();
    }
    for entity in existing_machines.iter() {
        commands.entity(entity).despawn();
    }

    // Initialize terrain from plan
    terrain.clear();
    *terrain = crate::terrain::VoxelTerrain::new(&config);

    // Decode and populate terrain data if present
    if let Some(ref terrain_data) = plan.site.terrain_data {
        if let Ok(decoded) = decode_terrain_data(terrain_data) {
            populate_terrain(&mut commands, &mut terrain, &decoded, &config);
        }
    }

    // Spawn machines from plan
    let catalog = crate::machines::MachineCatalog::default();
    for machine_def in &plan.machines {
        let mut bundle = catalog.create_machine(machine_def.machine_type, machine_def.id.clone());
        bundle.transform.translation = machine_def.initial_position;
        if let Some(rotation) = machine_def.initial_rotation {
            bundle.transform.rotation = bevy_math::Quat::from_rotation_y(rotation);
        }
        commands.spawn(bundle);
    }

    // Mark reload as complete
    playback.mark_loaded();
}

/// Decodes RLE-compressed terrain data.
fn decode_terrain_data(data: &str) -> Result<Vec<u8>, base64::DecodeError> {
    use base64::Engine;
    let compressed = base64::engine::general_purpose::STANDARD.decode(data)?;

    // Simple RLE decoding: pairs of (count, value)
    let mut result = Vec::new();
    let mut i = 0;
    while i + 1 < compressed.len() {
        let count = compressed[i] as usize;
        let value = compressed[i + 1];
        result.extend(std::iter::repeat(value).take(count));
        i += 2;
    }

    Ok(result)
}

/// Populates terrain from decoded voxel data.
fn populate_terrain(
    commands: &mut Commands,
    terrain: &mut crate::terrain::VoxelTerrain,
    data: &[u8],
    config: &crate::config::EarthworksConfig,
) {
    use crate::terrain::{Chunk, ChunkCoord, DirtyChunk, MaterialId, Voxel};

    // Assuming data is a flat array in XYZ order
    // This is a simplified implementation - real implementation would need bounds info
    let chunk_size = config.chunk_size as usize;

    // Group voxels by chunk
    let mut chunks: bevy_platform::collections::HashMap<ChunkCoord, Chunk> =
        bevy_platform::collections::HashMap::default();

    for (i, &material_byte) in data.iter().enumerate() {
        if material_byte == 0 {
            continue; // Skip air
        }

        // Convert flat index to 3D coordinates (assuming some default bounds)
        let bounds_x = 80usize;
        let bounds_y = 8usize;
        let x = i % bounds_x;
        let y = (i / bounds_x) % bounds_y;
        let z = i / (bounds_x * bounds_y);

        let chunk_coord = ChunkCoord::new(
            (x / chunk_size) as i32,
            (y / chunk_size) as i32,
            (z / chunk_size) as i32,
        );
        let local_x = x % chunk_size;
        let local_y = y % chunk_size;
        let local_z = z % chunk_size;

        let chunk = chunks.entry(chunk_coord).or_insert_with(Chunk::new);
        let material = MaterialId::from_u8(material_byte).unwrap_or(MaterialId::Dirt);
        chunk.set(local_x, local_y, local_z, Voxel::solid(material));
    }

    // Spawn chunk entities
    for (coord, chunk) in chunks {
        let entity = commands.spawn((chunk, coord, DirtyChunk)).id();
        terrain.set_chunk_entity(coord, entity);
    }
}
