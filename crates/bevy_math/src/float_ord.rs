use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::Neg,
};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

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
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Hash)
)]
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
    use std::hash::DefaultHasher;

    use super::*;

    const NAN: FloatOrd = FloatOrd(f32::NAN);
    const ZERO: FloatOrd = FloatOrd(0.0);
    const ONE: FloatOrd = FloatOrd(1.0);

    #[test]
    fn float_ord_eq() {
        assert_eq!(NAN, NAN);

        assert_ne!(NAN, ZERO);
        assert_ne!(ZERO, NAN);

        assert_eq!(ZERO, ZERO);
    }

    #[test]
    fn float_ord_cmp() {
        assert_eq!(NAN.cmp(&NAN), Ordering::Equal);

        assert_eq!(NAN.cmp(&ZERO), Ordering::Less);
        assert_eq!(ZERO.cmp(&NAN), Ordering::Greater);

        assert_eq!(ZERO.cmp(&ZERO), Ordering::Equal);
        assert_eq!(ONE.cmp(&ZERO), Ordering::Greater);
        assert_eq!(ZERO.cmp(&ONE), Ordering::Less);
    }

    #[test]
    #[allow(clippy::nonminimal_bool)]
    fn float_ord_cmp_operators() {
        assert!(!(NAN < NAN));
        assert!(NAN < ZERO);
        assert!(!(ZERO < NAN));
        assert!(!(ZERO < ZERO));
        assert!(ZERO < ONE);
        assert!(!(ONE < ZERO));

        assert!(!(NAN > NAN));
        assert!(!(NAN > ZERO));
        assert!(ZERO > NAN);
        assert!(!(ZERO > ZERO));
        assert!(!(ZERO > ONE));
        assert!(ONE > ZERO);

        assert!(NAN <= NAN);
        assert!(NAN <= ZERO);
        assert!(!(ZERO <= NAN));
        assert!(ZERO <= ZERO);
        assert!(ZERO <= ONE);
        assert!(!(ONE <= ZERO));

        assert!(NAN >= NAN);
        assert!(!(NAN >= ZERO));
        assert!(ZERO >= NAN);
        assert!(ZERO >= ZERO);
        assert!(!(ZERO >= ONE));
        assert!(ONE >= ZERO);
    }

    #[test]
    fn float_ord_hash() {
        let hash = |num| {
            let mut h = DefaultHasher::new();
            FloatOrd(num).hash(&mut h);
            h.finish()
        };

        assert_ne!((-0.0f32).to_bits(), 0.0f32.to_bits());
        assert_eq!(hash(-0.0), hash(0.0));

        let nan_1 = f32::from_bits(0b0111_1111_1000_0000_0000_0000_0000_0001);
        assert!(nan_1.is_nan());
        let nan_2 = f32::from_bits(0b0111_1111_1000_0000_0000_0000_0000_0010);
        assert!(nan_2.is_nan());
        assert_ne!(nan_1.to_bits(), nan_2.to_bits());
        assert_eq!(hash(nan_1), hash(nan_2));
    }
}
