//! Machine component definitions.

use bevy_ecs::prelude::*;
use bevy_math::Vec3;
use bevy_reflect::Reflect;
use bevy_transform::components::Transform;
use serde::{Deserialize, Serialize};

/// Type of construction machine.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect)]
pub enum MachineType {
    /// Excavator with rotating upper body and boom arm.
    #[default]
    Excavator,
    /// Bulldozer with front blade.
    Dozer,
    /// Wheel loader with front bucket.
    Loader,
    /// Dump truck for hauling material.
    DumpTruck,
}

impl MachineType {
    /// Returns the display name of this machine type.
    pub const fn name(&self) -> &'static str {
        match self {
            MachineType::Excavator => "Excavator",
            MachineType::Dozer => "Bulldozer",
            MachineType::Loader => "Wheel Loader",
            MachineType::DumpTruck => "Dump Truck",
        }
    }
}

/// Mobility characteristics of a machine.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, Reflect, Component)]
pub struct Mobility {
    /// Maximum travel speed in m/s.
    pub max_speed: f32,
    /// Rotation speed in radians/s.
    pub turn_rate: f32,
    /// Whether the machine can reverse.
    pub can_reverse: bool,
    /// Whether the machine uses tracks (vs wheels).
    pub tracked: bool,
}

impl Default for Mobility {
    fn default() -> Self {
        Self {
            max_speed: 5.0,
            turn_rate: 1.0,
            can_reverse: true,
            tracked: false,
        }
    }
}

/// Work envelope defining the operational reach of a machine.
#[derive(Clone, Debug, Serialize, Deserialize, Reflect, Component)]
pub enum WorkEnvelope {
    /// Toroidal (donut-shaped) envelope for excavators.
    /// Inner radius is minimum reach, outer radius is maximum reach.
    Toroidal {
        /// Inner radius (minimum reach).
        inner_radius: f32,
        /// Outer radius (maximum reach).
        outer_radius: f32,
        /// Minimum height relative to machine base.
        min_height: f32,
        /// Maximum height relative to machine base.
        max_height: f32,
    },
    /// Rectangular envelope for dozers.
    Rectangular {
        /// Width of the blade.
        width: f32,
        /// Distance in front of machine.
        depth: f32,
        /// Height of the blade.
        height: f32,
    },
    /// Arc-shaped envelope for loaders.
    Arc {
        /// Radius of reach.
        radius: f32,
        /// Arc angle in radians (centered on forward direction).
        angle: f32,
        /// Minimum height.
        min_height: f32,
        /// Maximum height.
        max_height: f32,
    },
}

impl Default for WorkEnvelope {
    fn default() -> Self {
        WorkEnvelope::Toroidal {
            inner_radius: 2.0,
            outer_radius: 8.0,
            min_height: -4.0,
            max_height: 6.0,
        }
    }
}

impl WorkEnvelope {
    /// Checks if a point (relative to machine position) is within the work envelope.
    pub fn contains(&self, point: Vec3) -> bool {
        match self {
            WorkEnvelope::Toroidal {
                inner_radius,
                outer_radius,
                min_height,
                max_height,
            } => {
                let horizontal_dist = (point.x * point.x + point.z * point.z).sqrt();
                horizontal_dist >= *inner_radius
                    && horizontal_dist <= *outer_radius
                    && point.y >= *min_height
                    && point.y <= *max_height
            }
            WorkEnvelope::Rectangular {
                width,
                depth,
                height,
            } => {
                point.x.abs() <= width / 2.0
                    && point.z >= 0.0
                    && point.z <= *depth
                    && point.y >= 0.0
                    && point.y <= *height
            }
            WorkEnvelope::Arc {
                radius,
                angle,
                min_height,
                max_height,
            } => {
                let horizontal_dist = (point.x * point.x + point.z * point.z).sqrt();
                if horizontal_dist > *radius || point.y < *min_height || point.y > *max_height {
                    return false;
                }
                // Check angle (point must be in front arc)
                if point.z <= 0.0 {
                    return false;
                }
                let point_angle = point.x.atan2(point.z).abs();
                point_angle <= angle / 2.0
            }
        }
    }
}

/// Current activity state of a machine.
#[derive(Clone, Debug, Default, Serialize, Deserialize, Reflect, Component)]
pub enum MachineActivity {
    /// Machine is idle.
    #[default]
    Idle,
    /// Machine is traveling to a destination.
    Traveling {
        /// Target position.
        target: Vec3,
        /// Progress (0.0 to 1.0).
        progress: f32,
        /// Start position.
        start: Vec3,
    },
    /// Machine is excavating material.
    Excavating {
        /// Target voxel position.
        target: Vec3,
        /// Progress (0.0 to 1.0).
        progress: f32,
        /// Volume being excavated.
        volume: f32,
    },
    /// Machine is dumping material.
    Dumping {
        /// Target dump position.
        target: Vec3,
        /// Progress (0.0 to 1.0).
        progress: f32,
        /// Volume being dumped.
        volume: f32,
    },
    /// Machine is pushing material (dozer).
    Pushing {
        /// Direction of push.
        direction: Vec3,
        /// Progress (0.0 to 1.0).
        progress: f32,
    },
}

impl MachineActivity {
    /// Returns the progress of the current activity (0.0 to 1.0).
    pub fn progress(&self) -> f32 {
        match self {
            MachineActivity::Idle => 1.0,
            MachineActivity::Traveling { progress, .. } => *progress,
            MachineActivity::Excavating { progress, .. } => *progress,
            MachineActivity::Dumping { progress, .. } => *progress,
            MachineActivity::Pushing { progress, .. } => *progress,
        }
    }

    /// Returns true if the activity is complete.
    pub fn is_complete(&self) -> bool {
        self.progress() >= 1.0
    }
}

/// Core machine component.
#[derive(Clone, Debug, Serialize, Deserialize, Reflect, Component)]
pub struct Machine {
    /// Unique identifier for this machine.
    pub id: String,
    /// Type of machine.
    pub machine_type: MachineType,
    /// Current bucket/blade load (0.0 to capacity).
    pub current_load: f32,
    /// Maximum bucket/blade capacity in cubic meters.
    pub capacity: f32,
    /// Current fuel level (0.0 to 1.0).
    pub fuel: f32,
}

impl Default for Machine {
    fn default() -> Self {
        Self {
            id: String::new(),
            machine_type: MachineType::Excavator,
            current_load: 0.0,
            capacity: 1.0,
            fuel: 1.0,
        }
    }
}

/// Bundle for spawning a complete machine entity.
#[derive(Bundle, Default)]
pub struct MachineBundle {
    /// Core machine data.
    pub machine: Machine,
    /// Work envelope.
    pub envelope: WorkEnvelope,
    /// Mobility characteristics.
    pub mobility: Mobility,
    /// Current activity.
    pub activity: MachineActivity,
    /// Transform.
    pub transform: Transform,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toroidal_envelope_contains() {
        let envelope = WorkEnvelope::Toroidal {
            inner_radius: 2.0,
            outer_radius: 8.0,
            min_height: -4.0,
            max_height: 6.0,
        };

        // Point within envelope
        assert!(envelope.contains(Vec3::new(5.0, 0.0, 0.0)));

        // Point too close
        assert!(!envelope.contains(Vec3::new(1.0, 0.0, 0.0)));

        // Point too far
        assert!(!envelope.contains(Vec3::new(10.0, 0.0, 0.0)));

        // Point too high
        assert!(!envelope.contains(Vec3::new(5.0, 10.0, 0.0)));

        // Point too low
        assert!(!envelope.contains(Vec3::new(5.0, -5.0, 0.0)));
    }

    #[test]
    fn test_rectangular_envelope_contains() {
        let envelope = WorkEnvelope::Rectangular {
            width: 4.0,
            depth: 2.0,
            height: 1.0,
        };

        // Point within envelope
        assert!(envelope.contains(Vec3::new(0.0, 0.5, 1.0)));

        // Point outside width
        assert!(!envelope.contains(Vec3::new(3.0, 0.5, 1.0)));

        // Point behind machine
        assert!(!envelope.contains(Vec3::new(0.0, 0.5, -1.0)));
    }
}
