//! Bevy Earthworks - Volumetric voxel terrain and construction machine simulation
//!
//! This crate provides:
//! - Volumetric voxel terrain with efficient modification and rendering
//! - Construction machine simulation with work envelope constraints
//! - Plan execution and playback system for pre-computed operations
//! - Timeline UI for interactive visualization
//!
//! # Example
//!
//! ```no_run
//! use bevy::prelude::*;
//! use bevy_earthworks::prelude::*;
//!
//! fn main() {
//!     App::new()
//!         .add_plugins(DefaultPlugins)
//!         .add_plugins(EarthworksPlugin::default())
//!         .run();
//! }
//! ```

#![warn(missing_docs)]

pub mod camera;
pub mod effects;
pub mod export;
pub mod jobs;
pub mod machines;
pub mod models;
pub mod plan;
pub mod scoring;
pub mod terrain;
pub mod testing;
pub mod ui;
pub mod zyns;

mod config;
mod plugin;

pub use config::EarthworksConfig;
pub use plugin::EarthworksPlugin;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::camera::{CameraShake, CameraTraumaEvent, OrbitCamera};
    pub use crate::config::EarthworksConfig;
    pub use crate::jobs::{CurrentJob, Job, JobType, JobsPlugin};
    pub use crate::machines::{
        BladeState, BladeVisual, ControlResponse, DirectControlPlugin, DozerPushState, Machine,
        MachineActivity, MachineCatalog, MachineType, Mobility, PlayerControlled, WorkEnvelope,
    };
    pub use crate::models::{
        spawn_machine_with_model, MachineModelRegistry, MachineSpawner, ModelAssets, ModelLoadState,
    };
    pub use crate::plan::{ExecutionPlan, PlanPlayback, PlanStep, PlannedAction};
    pub use crate::plugin::EarthworksPlugin;
    pub use crate::scoring::SimulationScore;
    pub use crate::terrain::{
        get_terrain_height, get_terrain_height_interpolated, Chunk, ChunkCoord, ChunkLOD,
        MaterialId, TerrainModifiedEvent, Voxel, VoxelState, VoxelTerrain, CHUNK_SIZE,
    };
    pub use crate::testing::{AIAgent, AgentBehavior, AgentGoal, TestingPlugin};
    pub use crate::ui::TimelineState;
    pub use crate::zyns::{
        Achievement, AchievementTracker, AchievementUnlockedEvent, ZynsConfig, ZynsEarnedEvent,
        ZynsSource, ZynsUiState, ZynsWallet,
    };
}
