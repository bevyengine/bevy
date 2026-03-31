use core::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::Neg,
};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// A wrapper for floats that implements [`Ord`], [`Eq`], and [`Hash`] traits.
///
/// This is a work around for the fact that the IEEE 754-2008 standard,
/// implemented by Rust's [`f32`] type, doesn't define an ordering for
/// [`NaN`](f32::NAN).
#[derive(Debug, Copy, Clone)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, PartialEq, Hash, Clone)
)]
pub struct FloatOrd(pub f32);

impl PartialOrd for FloatOrd {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FloatOrd {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl PartialEq for FloatOrd {
    fn eq(&self, other: &Self) -> bool {
        matches!(self.cmp(other), Ordering::Equal)
    }
}

impl Eq for FloatOrd {}

impl Hash for FloatOrd {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
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

        assert_ne!(NAN.cmp(&ZERO), Ordering::Equal);
        assert_ne!(ZERO.cmp(&NAN), Ordering::Equal);

        assert_eq!(ZERO.cmp(&ZERO), Ordering::Equal);
        assert_eq!(ONE.cmp(&ZERO), Ordering::Greater);
        assert_eq!(ZERO.cmp(&ONE), Ordering::Less);
    }

    #[test]
    #[expect(
        clippy::nonminimal_bool,
        reason = "This tests that all operators work as they should, and in the process requires some non-simplified boolean expressions."
    )]
    fn float_ord_cmp_operators() {
        assert!(!(NAN < NAN));
        assert_ne!(NAN, ZERO);
        assert!(!(ZERO < ZERO));
        assert!(ZERO < ONE);
        assert!(!(ONE < ZERO));

        assert!(!(NAN > NAN));
        assert!(!(ZERO > ZERO));
        assert!(!(ZERO > ONE));
        assert!(ONE > ZERO);

        assert!(NAN <= NAN);
        assert!(ZERO <= ZERO);
        assert!(ZERO <= ONE);
        assert!(!(ONE <= ZERO));

        assert!(NAN >= NAN);
        assert!(ZERO >= ZERO);
        assert!(!(ZERO >= ONE));
        assert!(ONE >= ZERO);
    }
}
