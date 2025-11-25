//! Distance joint type with minimum/maximum distance constraints.
//!
//! PhysicsDistanceJoint defines a predefined distance joint type where the distance
//! between rigid bodies may be limited to a given minimum or maximum distance.

use bevy_ecs_macros::Component;

/// Marks this entity as a distance joint constraining the distance between bodies.
#[derive(Component)]
pub struct DistanceJoint {
    /// Minimum distance. If negative, the joint is not limited.
    /// Units: distance.
    pub min_distance: f32,

    /// Maximum distance. If negative, the joint is not limited.
    /// Units: distance.
    pub max_distance: f32,
}

impl Default for DistanceJoint {
    fn default() -> Self {
        Self {
            min_distance: -1.0,
            max_distance: -1.0,
        }
    }
}
