//! Joint drive (motor/actuator) configuration.
//!
//! The [`DriveConfig`] type and drive components allow joints to be motorized
//! along specific degrees of freedom. Drives act as force-limited damped springs
//! that can target either a position or velocity.
//!
//! ## Drive Formula
//!
//! The resulting drive force or acceleration is proportional to:
//!
//! ```text
//! force = stiffness × (targetPosition - position) + damping × (targetVelocity - velocity)
//! ```
//!
//! Where:
//! - `position` is the current relative pose along the DOF (angle for revolute, distance for prismatic)
//! - `velocity` is the rate of change of this motion
//!
//! ## Drive Types
//!
//! - **Force drive** ([`DriveType::Force`]): The computed value is a force/torque.
//!   The resulting motion depends on the mass of connected bodies.
//! - **Acceleration drive** ([`DriveType::Acceleration`]): The computed value is
//!   an acceleration. Motion is independent of body mass—useful for precise
//!   motion control in robotics applications.
//!
//! ## Position vs Velocity Targeting
//!
//! - **Position targeting**: Set `stiffness > 0` and `target_position` to desired pose.
//!   The drive acts like a spring pulling toward the target.
//! - **Velocity targeting**: Set `damping > 0` and `target_velocity` to desired speed.
//!   The drive acts like a motor maintaining constant velocity.
//! - **Combined**: Both can be used together for spring-damper behavior.
//!
//! ## Available DOFs
//!
//! - **Translation**: [`DriveTransX`], [`DriveTransY`], [`DriveTransZ`]
//! - **Rotation**: [`DriveRotX`], [`DriveRotY`], [`DriveRotZ`]
//! - **Special**: [`DriveLinear`] (prismatic), [`DriveAngular`] (revolute)
//!
//! ## Units
//!
//! Units vary by drive type and DOF:
//!
//! | Property | Linear DOF | Angular DOF |
//! |----------|------------|-------------|
//! | `max_force` | mass×distance/second² | mass×distance²/second² |
//! | `target_position` | distance | radians |
//! | `target_velocity` | distance/second | radians/second |
//! | `stiffness` | mass/second² | mass×distance²/radians/second² |
//! | `damping` | mass/second | mass×distance²/second/radians |

use crate::types::float;

/// The type of drive (how the computed value is interpreted).
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriveType {
    /// Drive applies a force/torque.
    ///
    /// The resulting motion depends on the mass of connected bodies.
    /// Heavier bodies move more slowly for the same drive force.
    #[default]
    Force,

    /// Drive applies an acceleration.
    ///
    /// The resulting motion is independent of body mass. This is useful
    /// for robotics applications where precise motion control is needed
    /// regardless of payload mass.
    Acceleration,
}

/// Drive configuration for joint actuation.
///
/// A force-limited damped spring that can target position and/or velocity:
/// `force = stiffness × (targetPosition - position) + damping × (targetVelocity - velocity)`
#[derive(Clone, Copy, Debug)]
pub struct DriveConfig {
    /// Whether this drive applies force or acceleration.
    pub drive_type: DriveType,

    /// Maximum force/torque the drive can apply.
    ///
    /// Use `float::INFINITY` for unlimited force.
    /// Must be non-negative.
    ///
    /// Units (linear): mass × distance / second²
    /// Units (angular): mass × distance² / second²
    pub max_force: float,

    /// Target position for the drive.
    ///
    /// Only effective when `stiffness > 0`.
    ///
    /// Units (linear): distance
    /// Units (angular): radians
    pub target_position: float,

    /// Target velocity for the drive.
    ///
    /// Only effective when `damping > 0`.
    ///
    /// Units (linear): distance / second
    /// Units (angular): radians / second
    pub target_velocity: float,

    /// Damping coefficient (velocity term).
    ///
    /// Controls how strongly the drive resists deviation from target velocity.
    /// Set to 0 for pure position targeting.
    ///
    /// Units (linear): mass / second
    /// Units (angular): mass × distance² / second / radians
    pub damping: float,

    /// Stiffness coefficient (position term).
    ///
    /// Controls how strongly the drive pulls toward target position.
    /// Set to 0 for pure velocity targeting.
    ///
    /// Units (linear): mass / second²
    /// Units (angular): mass × distance² / radians / second²
    pub stiffness: float,
}

impl DriveConfig {
    /// Default drive configuration (no actuation).
    ///
    /// All targets are zero, no stiffness or damping, unlimited force.
    pub const DEFAULT: Self = Self {
        drive_type: DriveType::Force,
        max_force: float::INFINITY,
        target_position: 0.0,
        target_velocity: 0.0,
        damping: 0.0,
        stiffness: 0.0,
    };

    /// Returns true if this drive has no effect (zero stiffness and damping).
    pub fn is_inactive(&self) -> bool {
        self.stiffness == 0.0 && self.damping == 0.0
    }
}

impl Default for DriveConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

make_attribute! {
    /// Drive for the X translation axis.
    DriveTransX(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:transX"
    displayName = "Drive Trans X"
}

make_attribute! {
    /// Drive for the Y translation axis.
    DriveTransY(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:transY"
    displayName = "Drive Trans Y"
}

make_attribute! {
    /// Drive for the Z translation axis.
    DriveTransZ(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:transZ"
    displayName = "Drive Trans Z"
}

make_attribute! {
    /// Drive for the X rotation axis.
    DriveRotX(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:rotX"
    displayName = "Drive Rot X"
}

make_attribute! {
    /// Drive for the Y rotation axis.
    DriveRotY(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:rotY"
    displayName = "Drive Rot Y"
}

make_attribute! {
    /// Drive for the Z rotation axis.
    DriveRotZ(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:rotZ"
    displayName = "Drive Rot Z"
}

make_attribute! {
    /// Drive for linear distance (used with prismatic joints).
    DriveLinear(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:linear"
    displayName = "Drive Linear"
}

make_attribute! {
    /// Drive for angular motion (used with revolute joints).
    DriveAngular(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:angular"
    displayName = "Drive Angular"
}
