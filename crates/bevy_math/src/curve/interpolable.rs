//! The [`Interpolable`] trait for types that support interpolation between two values.

use crate::{Quat, VectorSpace};

/// A trait for types whose values can be intermediately interpolated between two given values
/// with an auxiliary parameter.
pub trait Interpolable: Clone {
    /// Interpolate between this value and the `other` given value using the parameter `t`.
    /// Note that the parameter `t` is not necessarily clamped to lie between `0` and `1`.
    fn interpolate(&self, other: &Self, t: f32) -> Self;
}

impl<S, T> Interpolable for (S, T)
where
    S: Interpolable,
    T: Interpolable,
{
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        (
            self.0.interpolate(&other.0, t),
            self.1.interpolate(&other.1, t),
        )
    }
}

impl<T> Interpolable for T
where
    T: VectorSpace,
{
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.lerp(*other, t)
    }
}

impl Interpolable for Quat {
    fn interpolate(&self, other: &Self, t: f32) -> Self {
        self.slerp(*other, t)
    }
}
