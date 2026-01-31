//! Joint limit configuration for degrees of freedom.
//!
//! The [`LimitRange`] type and limit components restrict movement along specific
//! joint axes. This is a multi-apply schema pattern where limits can be applied
//! to different degrees of freedom (DOFs).
//!
//! ## Available Degrees of Freedom
//!
//! - **Translation**: [`LimitTransX`], [`LimitTransY`], [`LimitTransZ`]
//! - **Rotation**: [`LimitRotX`], [`LimitRotY`], [`LimitRotZ`]
//! - **Special**: [`LimitLinear`] (prismatic), [`LimitAngular`] (revolute)
//!
//! ## Lock Semantics
//!
//! When `low > high` in a [`LimitRange`], motion along that axis is **locked**
//! (no movement allowed). This is a convenient way to fully constrain an axis
//! without needing a separate "locked" flag.
//!
//! ## Limit States
//!
//! A degree of freedom can be in one of three states:
//! 1. **Free**: `low = -∞` and `high = +∞` (unlimited motion)
//! 2. **Limited**: `low < high` (motion constrained to range [low, high])
//! 3. **Locked**: `low >= high` (no motion allowed)
//!
//! ## Units
//!
//! - Translation limits: distance units
//! - Rotation limits: radians
//!
//! ## D6 Joint Configuration
//!
//! For generic D6 joints, applying limit components to specific DOFs creates
//! custom joint configurations. Combined with [`DriveAPI`](crate::drive)
//! components, this allows building any joint type from primitives.

use crate::types::float;

/// Limit range configuration for joint axis constraints.
///
/// Defines the allowable range of motion for a degree of freedom.
/// When `low > high`, the axis is locked (no motion allowed).
#[derive(Default, Clone, Copy, Debug)]
pub struct LimitRange {
    /// Lower limit of motion.
    ///
    /// Use `float::NEG_INFINITY` for no lower bound.
    /// Units depend on DOF type (distance for translation, radians for rotation).
    pub low: float,

    /// Upper limit of motion.
    ///
    /// Use `float::INFINITY` for no upper bound.
    /// Units depend on DOF type (distance for translation, radians for rotation).
    pub high: float,
}

impl LimitRange {
    /// Fully unconstrained motion (free DOF).
    pub const UNLOCKED: Self = Self {
        low: float::NEG_INFINITY,
        high: float::INFINITY,
    };

    /// Create a new limit range with the given bounds.
    ///
    /// If `low > high`, the axis will be locked.
    pub fn new(low: float, high: float) -> Self {
        Self { low, high }
    }

    /// Create an unlocked limit range (fully unconstrained).
    pub fn unlocked() -> Self {
        Self::UNLOCKED
    }

    /// Create a locked limit range (no motion allowed).
    ///
    /// Sets `low = high = 0`, which satisfies `low >= high` and locks the axis.
    pub fn locked() -> Self {
        Self {
            low: 0.0,
            high: 0.0,
        }
    }

    /// Returns true if this range represents a locked axis.
    pub fn is_locked(&self) -> bool {
        self.low >= self.high
    }

    /// Returns true if this range is fully unconstrained.
    pub fn is_free(&self) -> bool {
        self.low == float::NEG_INFINITY && self.high == float::INFINITY
    }
}

make_attribute! {
    /// Limit for the X translation axis.
    ///
    /// Units: distance.
    LimitTransX(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:transX"
    displayName = "Limit Trans X"
}

make_attribute! {
    /// Limit for the Y translation axis.
    ///
    /// Units: distance.
    LimitTransY(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:transY"
    displayName = "Limit Trans Y"
}

make_attribute! {
    /// Limit for the Z translation axis.
    ///
    /// Units: distance.
    LimitTransZ(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:transZ"
    displayName = "Limit Trans Z"
}

make_attribute! {
    /// Limit for the X rotation axis.
    ///
    /// Units: radians.
    LimitRotX(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:rotX"
    displayName = "Limit Rot X"
}

make_attribute! {
    /// Limit for the Y rotation axis.
    ///
    /// Units: radians.
    LimitRotY(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:rotY"
    displayName = "Limit Rot Y"
}

make_attribute! {
    /// Limit for the Z rotation axis.
    ///
    /// Units: radians.
    LimitRotZ(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:rotZ"
    displayName = "Limit Rot Z"
}

make_attribute! {
    /// Limit for linear distance (used with prismatic joints).
    ///
    /// Units: distance.
    LimitLinear(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:linear"
    displayName = "Limit Linear"
}

make_attribute! {
    /// Limit for angular motion (used with revolute joints).
    ///
    /// Units: radians.
    LimitAngular(LimitRange) = LimitRange::UNLOCKED;
    apiName = "limit:angular"
    displayName = "Limit Angular"
}
