use crate::bytes::AsBytes;
use std::{
    cmp::Ordering,
    hash::{Hash, Hasher},
    ops::Neg,
};

/// A wrapper type that enables ordering floats. This is a work around for the famous "rust float ordering" problem.
/// By using it, you acknowledge that sorting NaN is undefined according to spec. This implementation treats NaN as the
/// "smallest" float.
#[derive(Debug, Copy, Clone, PartialOrd)]
pub struct FloatOrd(pub f32);

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
        state.write(self.0.as_bytes());
    }
}

impl Neg for FloatOrd {
    type Output = FloatOrd;

    fn neg(self) -> Self::Output {
        FloatOrd(-self.0)
    }
}
