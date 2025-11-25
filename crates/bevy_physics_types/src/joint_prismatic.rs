//! Prismatic joint type with translational constraints.
//!
//! PhysicsPrismaticJoint defines a predefined prismatic joint type where translation
//! along the joint axis is permitted. It restricts all other linear motion and all
//! rotational motion.

use bevy_ecs_macros::Component;
use crate::axis::Axis;

/// Marks this entity as a prismatic joint constraining translation along a specified axis.
#[derive(Component)]
pub struct PrismaticJoint {
    /// Joint axis: X, Y, or Z.
    pub axis: Axis,

    /// Lower limit in distance. -inf means not limited in negative direction.
    pub lower_limit: f32,

    /// Upper limit in distance. inf means not limited in positive direction.
    pub upper_limit: f32,
}

impl Default for PrismaticJoint {
    fn default() -> Self {
        Self {
            axis: Axis::default(),
            lower_limit: f32::NEG_INFINITY,
            upper_limit: f32::INFINITY,
        }
    }
}
