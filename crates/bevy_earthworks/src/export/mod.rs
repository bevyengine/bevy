//! Export functionality for earthworks data.
//!
//! This module provides functionality to export:
//! - Terrain state as voxel data
//! - Simulation results and metrics
//! - Machine paths and operations

use bevy_ecs::prelude::*;
use serde::{Deserialize, Serialize};

/// Exported simulation results.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SimulationExport {
    /// Plan metadata.
    pub plan_name: String,
    /// Total simulation time.
    pub duration: f32,
    /// Volume metrics.
    pub volumes: VolumeMetrics,
    /// Machine metrics.
    pub machines: Vec<MachineMetrics>,
    /// Final score.
    pub score: f32,
}

/// Volume metrics for export.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct VolumeMetrics {
    /// Total volume excavated.
    pub excavated: f32,
    /// Total volume filled.
    pub filled: f32,
    /// Net volume change.
    pub net_change: f32,
}

/// Per-machine metrics for export.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MachineMetrics {
    /// Machine ID.
    pub id: String,
    /// Total distance traveled.
    pub distance: f32,
    /// Total fuel consumed.
    pub fuel: f32,
    /// Operations completed.
    pub operations: u32,
    /// Time spent idle.
    pub idle_time: f32,
}

/// Exports the current simulation state.
pub fn export_simulation(
    score: &crate::scoring::SimulationScore,
    plan_name: &str,
) -> SimulationExport {
    SimulationExport {
        plan_name: plan_name.to_string(),
        duration: score.elapsed_time,
        volumes: VolumeMetrics {
            excavated: score.volume_excavated,
            filled: score.volume_filled,
            net_change: score.volume_filled - score.volume_excavated,
        },
        machines: Vec::new(), // Would need to aggregate per-machine data
        score: score.calculate_score(),
    }
}
