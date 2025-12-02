//! Scoring and metrics tracking for earthworks simulations.

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;

use crate::machines::MachineEvent;
use crate::plan::PlanStepEvent;
use crate::terrain::TerrainModifiedEvent;

/// Plugin for scoring systems.
pub struct ScoringPlugin;

impl Plugin for ScoringPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SimulationScore>()
            .add_systems(Update, score_tracker_system);
    }
}

/// Resource tracking simulation scores and metrics.
#[derive(Resource, Default, Clone, Debug, Reflect)]
pub struct SimulationScore {
    /// Total volume excavated in cubic meters.
    pub volume_excavated: f32,
    /// Total volume filled/dumped in cubic meters.
    pub volume_filled: f32,
    /// Total distance traveled by all machines in meters.
    pub total_distance: f32,
    /// Total fuel consumed (arbitrary units).
    pub fuel_consumed: f32,
    /// Number of operations completed.
    pub operations_completed: u32,
    /// Current efficiency score (0.0 to 1.0).
    pub efficiency: f32,
    /// Time elapsed in simulation.
    pub elapsed_time: f32,
    /// Idle time accumulated.
    pub idle_time: f32,
}

impl SimulationScore {
    /// Calculates the overall score.
    pub fn calculate_score(&self) -> f32 {
        if self.elapsed_time <= 0.0 {
            return 0.0;
        }

        // Score based on volume moved per unit time and fuel
        let volume_rate = (self.volume_excavated + self.volume_filled) / self.elapsed_time;
        let fuel_efficiency = if self.fuel_consumed > 0.0 {
            (self.volume_excavated + self.volume_filled) / self.fuel_consumed
        } else {
            0.0
        };

        // Penalize idle time
        let activity_ratio = 1.0 - (self.idle_time / self.elapsed_time).min(1.0);

        // Combined score
        (volume_rate * 10.0 + fuel_efficiency * 5.0) * activity_ratio * self.efficiency
    }

    /// Resets all scores.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// System that tracks scores based on events.
fn score_tracker_system(
    time: Res<bevy_time::Time>,
    mut score: ResMut<SimulationScore>,
    mut terrain_events: MessageReader<TerrainModifiedEvent>,
    mut machine_events: MessageReader<MachineEvent>,
    mut step_events: MessageReader<PlanStepEvent>,
) {
    // Update elapsed time
    score.elapsed_time += time.delta_secs();

    // Process terrain modification events
    for event in terrain_events.read() {
        let volume = event.volume_changed.abs() as f32 * 0.0283; // Convert voxels to cubic meters (rough estimate)
        match event.operation {
            crate::terrain::operations::TerrainOperation::Excavate => {
                score.volume_excavated += volume;
            }
            crate::terrain::operations::TerrainOperation::Fill => {
                score.volume_filled += volume;
            }
        }
        score.operations_completed += 1;
    }

    // Process machine events
    for event in machine_events.read() {
        match event {
            MachineEvent::CompletedExcavating { volume, .. } => {
                score.fuel_consumed += volume * 0.1;
            }
            MachineEvent::CompletedDumping { volume, .. } => {
                score.fuel_consumed += volume * 0.05;
            }
            MachineEvent::ReachedDestination { .. } => {
                // Could track distance here if we had start position
            }
            _ => {}
        }
    }

    // Process step events for efficiency calculation
    let successful_steps = step_events
        .read()
        .filter(|e| matches!(e.result, crate::plan::StepResult::Success))
        .count();

    if successful_steps > 0 {
        // Update efficiency based on success rate
        score.efficiency = score.efficiency * 0.9 + 0.1; // Smoothed update
    }
}
