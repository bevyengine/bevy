use glam::{Affine3A, Mat3, Vec3, Vec3Swizzles, Vec4};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

/// Reduced-size version of `glam::Affine3A` for use when storage has
/// significant performance impact. Convert to `glam::Affine3A` to do
/// non-trivial calculations.
#[cfg_attr(feature = "bevy_reflect", derive(Reflect))]
pub struct Affine3 {
    /// Scaling, rotation, shears, and other non-translation affine transforms
    pub matrix3: Mat3,
    /// Translation
    pub translation: Vec3,
}

impl Affine3 {
    /// Calculates the transpose of the affine 4x3 matrix to a 3x4 and formats it for packing into GPU buffers
    #[inline]
    pub fn to_transpose(&self) -> [Vec4; 3] {
        let transpose_3x3 = self.matrix3.transpose();
        [
            transpose_3x3.x_axis.extend(self.translation.x),
            transpose_3x3.y_axis.extend(self.translation.y),
            transpose_3x3.z_axis.extend(self.translation.z),
        ]
    }

    /// Calculates the inverse transpose of the 3x3 matrix and formats it for packing into GPU buffers
    #[inline]
    pub fn inverse_transpose_3x3(&self) -> ([Vec4; 2], f32) {
        let inverse_transpose_3x3 = Affine3A::from(self).inverse().matrix3.transpose();
        (
            [
                (inverse_transpose_3x3.x_axis, inverse_transpose_3x3.y_axis.x).into(),
                (
                    inverse_transpose_3x3.y_axis.yz(),
                    inverse_transpose_3x3.z_axis.xy(),
                )
                    .into(),
            ],
            inverse_transpose_3x3.z_axis.z,
        )
    }
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
