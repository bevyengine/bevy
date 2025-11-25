//! Revolute joint type with rotational constraints.
//!
//! PhysicsRevoluteJoint defines a predefined revolute joint type where rotation
//! along the joint axis is permitted. It restricts all linear motion and rotation
//! around the other two axes.

use bevy_ecs_macros::Component;
use crate::axis::Axis;

/// Marks this entity as a revolute joint constraining rotation around a specified axis.
#[derive(Component)]
pub struct RevoluteJoint {
    /// Joint axis: X, Y, or Z.
    pub axis: Axis,

    /// Lower limit in degrees. -inf means not limited in negative direction.
    pub lower_limit: f32,

    /// Upper limit in degrees. inf means not limited in positive direction.
    pub upper_limit: f32,
}

impl Default for RevoluteJoint {
    fn default() -> Self {
        Self {
            axis: Axis::default(),
            lower_limit: f32::NEG_INFINITY,
            upper_limit: f32::INFINITY,
        }
    }
}
