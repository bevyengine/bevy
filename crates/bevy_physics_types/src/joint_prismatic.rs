//! Prismatic (slider) joint type.
//!
//! A [`PrismaticJoint`] allows translation along a single axis while restricting
//! all other degrees of freedom. This represents sliding mechanisms like pistons,
//! drawer slides, and linear actuators.
//!
//! ## Behavior
//!
//! - **Allowed motion**: Translation along the specified axis
//! - **Restricted motion**: All rotation, translation along other axes
//!
//! ## Limits
//!
//! Linear limits can be specified in distance units:
//! - `lower_limit` / `upper_limit`: The allowable translation range
//! - Set both to infinity for unlimited translation
//! - A joint drive can be added via [`DriveAPI`](crate::drive) for motorization
//!
//! ## Axis Convention
//!
//! The axis specifies which local axis the translation occurs along.
//!
//! ## Example Uses
//!
//! - Hydraulic pistons
//! - Drawer slides
//! - Car suspensions (simplified)
//! - Industrial linear actuators
//! - Elevator mechanisms

use crate::axis::Axis;
use crate::types::float;
use bevy_ecs::component::Component;

/// A prismatic (slider) joint allowing translation along one axis.
///
/// This joint type represents sliding mechanisms where one body moves
/// linearly relative to another without rotation.
#[derive(Component)]
pub struct PrismaticJoint {
    /// The axis of translation (X, Y, or Z in the joint frame).
    ///
    /// Translation is permitted along this axis; all other DOFs are locked.
    pub axis: Axis,

    /// Lower translation limit.
    ///
    /// The minimum translation distance allowed. Use `float::NEG_INFINITY` for
    /// no lower limit.
    ///
    /// Units: distance.
    pub lower_limit: float,

    /// Upper translation limit.
    ///
    /// The maximum translation distance allowed. Use `float::INFINITY` for
    /// no upper limit.
    ///
    /// Units: distance.
    pub upper_limit: float,
}

impl Default for PrismaticJoint {
    fn default() -> Self {
        Self {
            axis: Axis::default(),
            lower_limit: float::NEG_INFINITY,
            upper_limit: float::INFINITY,
        }
    }
}
