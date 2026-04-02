//! Spherical (ball-and-socket) joint type.
//!
//! A [`SphericalJoint`] allows all rotational freedom while constraining all
//! translation. This represents ball-and-socket joints like hip joints,
//! shoulder joints, and trailer hitches.
//!
//! ## Behavior
//!
//! - **Allowed motion**: Rotation around all three axes
//! - **Restricted motion**: All translation
//!
//! ## Cone Limits
//!
//! Rotational freedom can be restricted using cone limits:
//! - `cone_angle0_limit`: Angle from the primary axis toward the "next" axis
//! - `cone_angle1_limit`: Angle from the primary axis toward the "second next" axis
//!
//! The axis cycling is: X → Y → Z → X
//!
//! When both cone angles are equal, the limit forms a **circular cone**.
//! When different, it forms an **elliptical cone**, useful for modeling
//! joints with asymmetric range of motion.
//!
//! ## Negative Values
//!
//! A negative cone angle means unlimited rotation in that direction
//! (sentinel value indicating no limit).
//!
//! ## Example Uses
//!
//! - Human hip and shoulder joints (with cone limits)
//! - Trailer hitch ball joints
//! - Camera gimbals
//! - Ragdoll character joints

use crate::axis::Axis;
use crate::types::angle;
use bevy_ecs::component::Component;

/// A spherical (ball-and-socket) joint allowing rotation around all axes.
///
/// This joint type represents ball joints where one body can rotate freely
/// relative to another but cannot translate.
#[derive(Component)]
pub struct SphericalJoint {
    /// The primary cone limit axis (X, Y, or Z).
    ///
    /// The cone limits are defined relative to this axis.
    pub axis: Axis,

    /// First cone angle limit in radians.
    ///
    /// Limits rotation from the primary axis toward the next axis in the
    /// cycle (X→Y, Y→Z, Z→X). A negative value means no limit.
    ///
    /// When equal to `cone_angle1_limit`, creates a circular cone.
    /// When different, creates an elliptical cone.
    ///
    /// Units: radians. Negative = unlimited.
    pub cone_angle0_limit: angle,

    /// Second cone angle limit in radians.
    ///
    /// Limits rotation from the primary axis toward the second-next axis
    /// in the cycle (X→Z, Y→X, Z→Y). A negative value means no limit.
    ///
    /// Units: radians. Negative = unlimited.
    pub cone_angle1_limit: angle,
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
