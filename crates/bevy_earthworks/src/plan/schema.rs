//! Schema definitions for execution plans.

use bevy_asset::Asset;
use bevy_math::Vec3;
use bevy_reflect::Reflect;
use serde::{Deserialize, Serialize};

use crate::machines::MachineType;
use crate::terrain::MaterialId;

/// An execution plan for earthworks operations.
#[derive(Asset, Clone, Debug, Serialize, Deserialize, Reflect)]
pub struct ExecutionPlan {
    /// Schema version.
    pub version: String,
    /// Plan metadata.
    pub metadata: PlanMetadata,
    /// Site definition.
    pub site: SiteDefinition,
    /// Machines in the plan.
    pub machines: Vec<MachineDef>,
    /// Execution steps.
    pub steps: Vec<PlanStep>,
    /// Expected results.
    #[serde(default)]
    pub expected_results: Option<ExpectedResults>,
}

/// Metadata about a plan.
#[derive(Clone, Debug, Serialize, Deserialize, Reflect)]
pub struct PlanMetadata {
    /// Plan name.
    pub name: String,
    /// Plan description.
    #[serde(default)]
    pub description: Option<String>,
    /// When the plan was created.
    #[serde(default)]
    pub created_at: Option<String>,
    /// Total duration in seconds.
    pub duration: f32,
    /// Total volume to be moved in cubic meters.
    pub total_volume: f32,
}

/// Site definition for a plan.
#[derive(Clone, Debug, Serialize, Deserialize, Reflect)]
pub struct SiteDefinition {
    /// Site bounds in voxels (x, y, z).
    pub bounds: [u32; 3],
    /// Voxel size in meters.
    pub voxel_size: f32,
    /// RLE-encoded terrain data (base64).
    #[serde(default)]
    pub terrain_data: Option<String>,
    /// Origin offset in world coordinates.
    #[serde(default)]
    pub origin: Option<Vec3>,
}

/// Machine definition in a plan.
#[derive(Clone, Debug, Serialize, Deserialize, Reflect)]
pub struct MachineDef {
    /// Unique machine identifier.
    pub id: String,
    /// Machine type.
    pub machine_type: MachineType,
    /// Initial position in world coordinates.
    pub initial_position: Vec3,
    /// Initial rotation (yaw in radians).
    #[serde(default)]
    pub initial_rotation: Option<f32>,
}

/// A single step in an execution plan.
#[derive(Clone, Debug, Serialize, Deserialize, Reflect)]
pub struct PlanStep {
    /// Timestamp when this step should execute (seconds from start).
    pub timestamp: f32,
    /// Machine ID that performs this step.
    pub machine_id: String,
    /// Action to perform.
    pub action: PlannedAction,
    /// Optional duration override.
    #[serde(default)]
    pub duration: Option<f32>,
}

/// Actions that can be performed in a plan step.
#[derive(Clone, Debug, Serialize, Deserialize, Reflect)]
#[serde(tag = "type")]
pub enum PlannedAction {
    /// Move to a position.
    MoveTo {
        /// Target position.
        target: Vec3,
    },
    /// Excavate at current position.
    Excavate {
        /// Target voxel position.
        target: Vec3,
        /// Volume to excavate.
        volume: f32,
        /// Material being excavated.
        #[serde(default)]
        material: Option<MaterialId>,
    },
    /// Dump carried material.
    Dump {
        /// Target dump position.
        target: Vec3,
        /// Volume to dump.
        volume: f32,
    },
    /// Push material (for dozers).
    Push {
        /// Direction to push.
        direction: Vec3,
        /// Distance to push.
        distance: f32,
    },
    /// Idle for a duration.
    Idle {
        /// Duration to idle.
        duration: f32,
    },
    /// Wait for another machine.
    WaitFor {
        /// Machine ID to wait for.
        machine_id: String,
        /// Step index to wait for.
        step_index: usize,
    },
}

/// Expected results for plan validation.
#[derive(Clone, Debug, Serialize, Deserialize, Reflect)]
pub struct ExpectedResults {
    /// Total volume excavated.
    pub volume_excavated: f32,
    /// Total volume filled.
    pub volume_filled: f32,
    /// Total distance traveled by all machines.
    pub total_distance: f32,
    /// Fuel consumed.
    pub fuel_consumed: f32,
    /// Final score.
    pub score: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_deserialization() {
        let json = r#"{
            "version": "1.0",
            "metadata": {
                "name": "Test Plan",
                "duration": 100.0,
                "total_volume": 500.0
            },
            "site": {
                "bounds": [80, 8, 80],
                "voxel_size": 0.3048
            },
            "machines": [
                {
                    "id": "excavator-1",
                    "machine_type": "Excavator",
                    "initial_position": [10.0, 0.0, 10.0]
                }
            ],
            "steps": [
                {
                    "timestamp": 0.0,
                    "machine_id": "excavator-1",
                    "action": {
                        "type": "MoveTo",
                        "target": [20.0, 0.0, 20.0]
                    }
                }
            ]
        }"#;

        let plan: ExecutionPlan = serde_json::from_str(json).unwrap();
        assert_eq!(plan.version, "1.0");
        assert_eq!(plan.metadata.name, "Test Plan");
        assert_eq!(plan.machines.len(), 1);
        assert_eq!(plan.steps.len(), 1);
    }
}
