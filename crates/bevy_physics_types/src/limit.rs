//! Joint axis and limit configuration.
//!
//! PhysicsLimitAPI restricts movement along an axis. It is a multipleApply schema
//! that can be applied to "transX", "transY", "transZ", "rotX", "rotY", "rotZ",
//! or "distance" to define different degrees of freedom. When the low limit is
//! higher than the high limit, motion along that axis is locked.

use core::f32;

/// Shared limit range type for joint axis constraints.
/// When low > high, motion along that axis is locked.
#[derive(Default, Clone, Copy, Debug)]
pub struct LimitRange {
    /// Lower limit. -inf means not limited in negative direction.
    pub low: f32,
    /// Upper limit. inf means not limited in positive direction.
    pub high: f32,
}

impl LimitRange {
    /// Unlocked limit range (fully unconstrained motion).
    pub const UNLOCKED: Self = Self {
        low: f32::NEG_INFINITY,
        high: f32::INFINITY,
    };

    /// Create a new limit range with the given bounds.
    pub fn new(low: f32, high: f32) -> Self {
        Self { low, high }
    }

    /// Create an unlocked limit range (fully unconstrained).
    pub fn unlocked() -> Self {
        Self::UNLOCKED
    }

    /// Create a locked limit range (no motion allowed).
    pub fn locked() -> Self {
        Self {
            low: 0.0,
            high: 0.0,
        }
    }
}

usd_attribute! {
    /// Limit configuration for the X translation axis.
    LimitTransX(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:transX"
    displayName = "Limit Trans X"
}

usd_attribute! {
    /// Limit configuration for the Y translation axis.
    LimitTransY(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:transY"
    displayName = "Limit Trans Y"
}

usd_attribute! {
    /// Limit configuration for the Z translation axis.
    LimitTransZ(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:transZ"
    displayName = "Limit Trans Z"
}

usd_attribute! {
    /// Limit configuration for the X rotation axis.
    LimitRotX(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:rotX"
    displayName = "Limit Rot X"
}

usd_attribute! {
    /// Limit configuration for the Y rotation axis.
    LimitRotY(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:rotY"
    displayName = "Limit Rot Y"
}

usd_attribute! {
    /// Limit configuration for the Z rotation axis.
    LimitRotZ(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:rotZ"
    displayName = "Limit Rot Z"
}

usd_attribute! {
    /// Limit configuration for linear distance (prismatic joints).
    LimitLinear(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:linear"
    displayName = "Limit Linear"
}

usd_attribute! {
    /// Limit configuration for angular motion (revolute joints).
    LimitAngular(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:angular"
    displayName = "Limit Angular"
}
