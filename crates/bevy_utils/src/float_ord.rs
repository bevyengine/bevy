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
#[derive(Debug, Copy, Clone)]
pub struct FloatOrd(pub f32);

impl PartialOrd for FloatOrd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }

    fn lt(&self, other: &Self) -> bool {
        !other.le(self)
    }
    // If `self` is NaN, it is equal to another NaN and less than all other floats, so return true.
    // If `self` isn't NaN and `other` is, the float comparison returns false, which match the `FloatOrd` ordering.
    // Otherwise, a standard float comparison happens.
    fn le(&self, other: &Self) -> bool {
        self.0.is_nan() || self.0 <= other.0
    }
    fn gt(&self, other: &Self) -> bool {
        !self.le(other)
    }
    fn ge(&self, other: &Self) -> bool {
        other.le(self)
    }
}

impl Ord for FloatOrd {
    #[allow(clippy::comparison_chain)]
    fn cmp(&self, other: &Self) -> Ordering {
        if self > other {
            Ordering::Greater
        } else if self < other {
            Ordering::Less
        } else {
            Ordering::Equal
        }
    }
}

impl PartialEq for FloatOrd {
    fn eq(&self, other: &Self) -> bool {
        if self.0.is_nan() {
            other.0.is_nan()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn float_ord_eq() {
        let nan = FloatOrd(f32::NAN);
        let num = FloatOrd(0.0);

        assert_eq!(nan, nan);

        assert_ne!(nan, num);
        assert_ne!(num, nan);

        assert_eq!(num, num);
    }

    #[test]
    fn float_ord_cmp() {
        let nan = FloatOrd(f32::NAN);
        let zero = FloatOrd(0.0);
        let one = FloatOrd(1.0);

        assert_eq!(nan.cmp(&nan), Ordering::Equal);

        assert_eq!(nan.cmp(&zero), Ordering::Less);
        assert_eq!(zero.cmp(&nan), Ordering::Greater);

        assert_eq!(zero.cmp(&zero), Ordering::Equal);
        assert_eq!(one.cmp(&zero), Ordering::Greater);
        assert_eq!(zero.cmp(&one), Ordering::Less);
    }
}
