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

impl Into<Affine3A> for &Affine3 {
    fn into(self) -> Affine3A {
        Affine3A {
            matrix3: self.matrix3.into(),
            translation: self.translation.into(),
        }
    }
}
