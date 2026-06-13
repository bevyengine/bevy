use glam::{vec3, Affine3, Affine3A, Mat3, Vec3Swizzles, Vec4, Vec4Swizzles};

/// Extension trait for [`Affine3`]
pub trait Affine3Ext {
    /// Generates an [`Affine3`] from a transposed 3x4 matrix.
    ///
    /// This is the inverse of [`Self::to_transpose`].
    fn from_transpose(transposed: [Vec4; 3]) -> Self;
    /// Calculates the transpose of the affine 4x3 matrix to a 3x4 and formats it for packing into GPU buffers
    fn to_transpose(self) -> [Vec4; 3];
    /// Calculates the inverse transpose of the 3x3 matrix and formats it for packing into GPU buffers
    fn inverse_transpose_3x3(self) -> ([Vec4; 2], f32);
}

impl Affine3Ext for Affine3 {
    fn from_transpose(transposed: [Vec4; 3]) -> Self {
        let transpose_3x3 = Mat3::from_cols(
            transposed[0].xyz(),
            transposed[1].xyz(),
            transposed[2].xyz(),
        );
        let translation = vec3(transposed[0].w, transposed[1].w, transposed[2].w);
        Affine3::from_mat3_translation(transpose_3x3.transpose(), translation)
    }

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
