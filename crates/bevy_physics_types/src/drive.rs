//! Joint drive and actuation.
//!

use core::f32;

#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriveType {
    /// Drive spring applies force at the joint.
    #[default]
    Force,
    /// Drive spring applies acceleration at the joint.
    Acceleration,
}

/// Shared drive configuration type for joint axis actuation.
/// Each drive is a force-limited damped spring:
/// Force or acceleration = stiffness * (targetPosition - position) + damping * (targetVelocity - velocity)
#[derive(Clone, Copy, Debug)]
pub struct DriveConfig {
    /// Drive type: force or acceleration.
    pub drive_type: DriveType,

    /// Maximum force that can be applied to drive. Units:
    /// if linear drive: mass*distance/second/second
    /// if angular drive: mass*distance*distance/second/second
    /// inf means not limited. Must be non-negative.
    pub max_force: f32,

    /// Target value for position. Units:
    /// if linear drive: distance
    /// if angular drive: degrees.
    pub target_position: f32,

    /// Target value for velocity. Units:
    /// if linear drive: distance/second
    /// if angular drive: degrees/second.
    pub target_velocity: f32,

    /// Damping of the drive. Units:
    /// if linear drive: mass/second
    /// if angular drive: mass*distance*distance/second/degrees.
    pub damping: f32,

    /// Stiffness of the drive. Units:
    /// if linear drive: mass/second/second
    /// if angular drive: mass*distance*distance/degrees/second/second.
    pub stiffness: f32,
}

impl DriveConfig {
    /// Default drive configuration (fully unconstrained, no actuation).
    pub const DEFAULT: Self = Self {
        drive_type: DriveType::Force,
        max_force: f32::INFINITY,
        target_position: 0.0,
        target_velocity: 0.0,
        damping: 0.0,
        stiffness: 0.0,
    };
}

impl Default for DriveConfig {
    fn default() -> Self {
        Self::DEFAULT
    }
}

usd_attribute! {
    /// Drive configuration for the X translation axis.
    DriveTransX(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:transX"
    displayName = "Drive Trans X"
}

usd_attribute! {
    /// Drive configuration for the Y translation axis.
    DriveTransY(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:transY"
    displayName = "Drive Trans Y"
}

usd_attribute! {
    /// Drive configuration for the Z translation axis.
    DriveTransZ(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:transZ"
    displayName = "Drive Trans Z"
}

usd_attribute! {
    /// Drive configuration for the X rotation axis.
    DriveRotX(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:rotX"
    displayName = "Drive Rot X"
}

usd_attribute! {
    /// Drive configuration for the Y rotation axis.
    DriveRotY(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:rotY"
    displayName = "Drive Rot Y"
}

usd_attribute! {
    /// Drive configuration for the Z rotation axis.
    DriveRotZ(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:rotZ"
    displayName = "Drive Rot Z"
}

usd_attribute! {
    /// Drive configuration for linear distance (prismatic joints).
    DriveLinear(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:linear"
    displayName = "Drive Linear"
}

usd_attribute! {
    /// Drive configuration for angular motion (revolute joints).
    DriveAngular(DriveConfig) = DriveConfig::DEFAULT;
    apiName = "drive:angular"
    displayName = "Drive Angular"
}
