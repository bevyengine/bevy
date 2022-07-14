use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::Neg,
};

/// A wrapper for floats that implements [`Ord`], [`Eq`], and [`Hash`] traits.
///
/// This is a work around for the fact that the IEEE 754-2008 standard,
/// implemented by Rust's [`f32`] type,
/// doesn't define an ordering for [`NaN`](f32::NAN),
/// and `NaN` is not considered equal to any other `NaN`.
///
/// Wrapping a float with `FloatOrd` breaks conformance with the standard
/// by sorting `NaN` as less than all other numbers and equal to any other `NaN`.
#[derive(Debug, Copy, Clone, PartialOrd)]
pub struct FloatOrd(pub f32);

#[allow(clippy::derive_ord_xor_partial_ord)]
impl Ord for FloatOrd {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or_else(|| {
            if self.0.is_nan() && !other.0.is_nan() {
                Ordering::Less
            } else if !self.0.is_nan() && other.0.is_nan() {
                Ordering::Greater
            } else {
                Ordering::Equal
            }
        })
    }
}

impl PartialEq for FloatOrd {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() && other.0.is_nan() {
            true
        } else {
            self.0 == other.0
        }
    }
}

impl Eq for FloatOrd {}

impl Hash for FloatOrd {
    fn hash<H: Hasher>(&self, state: &mut H) {
        if self.0.is_nan() {
            // Ensure all NaN representations hash to the same value
            state.write(&f32::to_ne_bytes(f32::NAN));
        } else if self.0 == 0.0 {
            // Ensure both zeroes hash to the same value
            state.write(&f32::to_ne_bytes(0.0f32));
        } else {
            state.write(&f32::to_ne_bytes(self.0));
        }
    }
}

impl Neg for FloatOrd {
    type Output = FloatOrd;

    fn neg(self) -> Self::Output {
        FloatOrd(-self.0)
    }
}
