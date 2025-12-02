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
pub mod machines;
pub mod plan;
pub mod scoring;
pub mod terrain;
pub mod ui;

mod config;
mod plugin;

pub use config::EarthworksConfig;
pub use plugin::EarthworksPlugin;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::config::EarthworksConfig;
    pub use crate::machines::{
        Machine, MachineActivity, MachineCatalog, MachineType, Mobility, WorkEnvelope,
    };
    pub use crate::plan::{ExecutionPlan, PlanPlayback, PlanStep, PlannedAction};
    pub use crate::plugin::EarthworksPlugin;
    pub use crate::scoring::SimulationScore;
    pub use crate::terrain::{
        Chunk, ChunkCoord, MaterialId, TerrainModifiedEvent, Voxel, VoxelState, VoxelTerrain,
    };
}
