use glam::{Affine3A, Mat3, Vec3};

/// Reduced-size version of `glam::Affine3A` for use when storage has
/// significant performance impact. Convert to `glam::Affine3A` to do
/// non-trivial calculations.
pub struct Affine3 {
    /// Scaling, rotation, shears, and other non-translation affine transforms
    pub matrix3: Mat3,
    /// Translation
    pub translation: Vec3,
}

impl From<&Affine3A> for Affine3 {
    fn from(affine: &Affine3A) -> Self {
        Self {
            matrix3: affine.matrix3.into(),
            translation: affine.translation.into(),
        }
    }
}

impl From<&Affine3> for Affine3A {
    fn from(affine3: &Affine3) -> Self {
        Self {
            matrix3: affine3.matrix3.into(),
            translation: affine3.translation.into(),
        }
    }
}
