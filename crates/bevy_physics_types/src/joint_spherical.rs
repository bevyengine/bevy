//! Spherical joint type with cone limit constraints.
//!
//! PhysicsSphericalJoint defines a predefined spherical joint type that removes linear
//! degrees of freedom. A cone limit may restrict the motion in a given range. Two limit
//! values can be specified: when equal they create a circular cone, otherwise an elliptic
//! cone limit around the limit axis.

use bevy_ecs_macros::Component;
use crate::axis::Axis;

/// Marks this entity as a spherical joint with optional cone limit constraints.
#[derive(Component)]
pub struct SphericalJoint {
    /// Cone limit axis: X, Y, or Z.
    pub axis: Axis,

    /// Cone limit from the primary joint axis in the local0 frame toward the next axis.
    /// (Next axis of X is Y, and of Z is X.) A negative value means not limited.
    /// Units: degrees.
    pub cone_angle0_limit: f32,

    /// Cone limit from the primary joint axis in the local0 frame toward the second to next axis.
    /// A negative value means not limited. Units: degrees.
    pub cone_angle1_limit: f32,
}

impl Default for SphericalJoint {
    fn default() -> Self {
        Self {
            axis: Axis::default(),
            cone_angle0_limit: -1.0,
            cone_angle1_limit: -1.0,
        }
    }
}
