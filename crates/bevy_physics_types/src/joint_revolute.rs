//! Revolute (hinge) joint type.
//!
//! A [`RevoluteJoint`] allows rotation around a single axis while restricting
//! all other degrees of freedom. This is the most common joint type, used for
//! hinges, wheels, motors, and many robotic joints.
//!
//! ## Behavior
//!
//! - **Allowed motion**: Rotation around the specified axis
//! - **Restricted motion**: All translation, rotation around other axes
//!
//! ## Limits
//!
//! Angular limits can be specified in degrees:
//! - `lower_limit` / `upper_limit`: The allowable rotation range
//! - Set both to infinity for unlimited rotation
//! - A joint drive can be added via [`DriveAPI`](crate::drive) for motorization
//!
//! ## Axis Convention
//!
//! The axis follows USD conventions established in `UsdGeomCapsule` and
//! `UsdGeomCylinder`, specifying which local axis the rotation occurs around.
//!
//! ## Example Uses
//!
//! - Door hinges (limited rotation)
//! - Wheels (unlimited rotation, often with drive)
//! - Robot arm joints
//! - Propellers and fans

use crate::axis::Axis;
use bevy_ecs::component::Component;

/// A revolute (hinge) joint allowing rotation around one axis.
///
/// This joint type is one of the most common in physics simulations,
/// representing hinges, wheels, and similar mechanisms.
#[derive(Component)]
pub struct RevoluteJoint {
    /// The axis of rotation (X, Y, or Z in the joint frame).
    ///
    /// Rotation is permitted around this axis; all other DOFs are locked.
    pub axis: Axis,

    /// Lower angular limit in degrees.
    ///
    /// The minimum rotation angle allowed. Use `f32::NEG_INFINITY` for
    /// no lower limit (unlimited rotation in negative direction).
    ///
    /// Units: degrees.
    pub lower_limit: f32,

    /// Upper angular limit in degrees.
    ///
    /// The maximum rotation angle allowed. Use `f32::INFINITY` for
    /// no upper limit (unlimited rotation in positive direction).
    ///
    /// Units: degrees.
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
