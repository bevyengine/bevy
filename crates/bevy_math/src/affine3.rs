use glam::{Affine3, Affine3A, Vec3Swizzles, Vec4};

/// Extension trait for [`Affine3`]
pub trait Affine3Ext {
    /// Calculates the transpose of the affine 4x3 matrix to a 3x4 and formats it for packing into GPU buffers
    fn to_transpose(self) -> [Vec4; 3];
    /// Calculates the inverse transpose of the 3x3 matrix and formats it for packing into GPU buffers
    fn inverse_transpose_3x3(self) -> ([Vec4; 2], f32);
}

impl Affine3Ext for Affine3 {
    #[inline]
    fn to_transpose(self) -> [Vec4; 3] {
        let transpose_3x3 = self.matrix3.transpose();
        [
            transpose_3x3.x_axis.extend(self.translation.x),
            transpose_3x3.y_axis.extend(self.translation.y),
            transpose_3x3.z_axis.extend(self.translation.z),
        ]
    }

    #[inline]
    fn inverse_transpose_3x3(self) -> ([Vec4; 2], f32) {
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
