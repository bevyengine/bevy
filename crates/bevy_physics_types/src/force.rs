use bevy_ecs::prelude::Component;
use bevy_math::Vec3;

/// Constant forces and torques that persist across time steps.
///
/// These are applied to the entity they attach to and are intended to be processed each physics step.
/// World-space variants are interpreted in "world" physics coordinates.
/// Local-space variants are interpreted in the attaching entity's local coordinate system.
///
/// Units:
/// - Force/Torque: mass * distance / time^2 for force, mass * distance^2 / time^2 for torque
/// - Acceleration: distance / time^2 or radians (for angular) / time^2
/// Applies a constant force in world space. Units: distance * mass / time^2.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ConstantForce(pub Vec3);

impl Default for ConstantForce {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

/// Applies a constant torque in world space.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ConstantTorque(pub Vec3);

impl Default for ConstantTorque {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

/// Applies a constant linear acceleration to the body in world space. Units: distance/time^2.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ConstantLinearAcceleration(pub Vec3);

impl Default for ConstantLinearAcceleration {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

/// Applies a constant angular acceleration (Euler rates) in world space. Units: radians/time^2.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ConstantAngularAcceleration(pub Vec3);

impl Default for ConstantAngularAcceleration {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

/// Local-space equivalent of `ConstantForce` - this vector is local to the entity and should
/// be transformed to world space before application.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ConstantLocalForce(pub Vec3);

impl Default for ConstantLocalForce {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

/// Local-space equivalent of `ConstantTorque`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ConstantLocalTorque(pub Vec3);

impl Default for ConstantLocalTorque {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

/// Local-space equivalent of `ConstantLinearAcceleration`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ConstantLocalLinearAcceleration(pub Vec3);

impl Default for ConstantLocalLinearAcceleration {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}

/// Local-space equivalent of `ConstantAngularAcceleration`.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct ConstantLocalAngularAcceleration(pub Vec3);

impl Default for ConstantLocalAngularAcceleration {
    fn default() -> Self {
        Self(Vec3::ZERO)
    }
}
