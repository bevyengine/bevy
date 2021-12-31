use crate::{Mat4, Vec3};

/// Implements a function to generate a translation / rotation matrix that faces a given target.
///
/// # Example
///
/// ```rust
/// # use bevy_math::{FaceToward, Vec3, Vec4};
/// # #[allow(dead_code)]
/// struct Matrix {
///     x_axis: Vec4,
///     y_axis: Vec4,
///     z_axis: Vec4,
///     w_axis: Vec4,
/// }
///
/// impl FaceToward for Matrix {
///     fn face_toward(eye: Vec3, center: Vec3, up: Vec3) -> Self {
///         let forward = (eye - center).normalize();
///         let right = up.cross(forward).normalize();
///         let up = forward.cross(right);
///
///         Matrix {
///             x_axis: right.extend(0.0),
///             y_axis: up.extend(0.0),
///             z_axis: forward.extend(0.0),
///             w_axis: eye.extend(1.0),
///         }
///     }
/// }
///
/// fn main() {
///     let matrix = Matrix::face_toward(
///         Vec3::new(50.0, 60.0, 0.0),
///         Vec3::new(0.0, 0.0, 0.0),
///         Vec3::new(0.0, 1.0, 0.0),
///     );
///
///     assert_eq!(matrix.x_axis, Vec4::new(0.0, 0.0, -1.0, -0.0));
///     assert_eq!(matrix.y_axis, Vec4::new(-0.7682213, 0.6401844, 0.0, 0.0));
///     assert_eq!(matrix.z_axis, Vec4::new(0.6401844, 0.7682213, 0.0, 0.0));
///     assert_eq!(matrix.w_axis, Vec4::new(50.0, 60.0, 0.0, 1.0));
/// }
/// ```
pub trait FaceToward {
    /// Generates a translation / rotation matrix that faces a given target.
    ///
    /// For an example on how to use this function see [`FaceToward`].
    fn face_toward(eye: Vec3, center: Vec3, up: Vec3) -> Self;
}

impl FaceToward for Mat4 {
    fn face_toward(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        let forward = (eye - center).normalize();
        let right = up.cross(forward).normalize();
        let up = forward.cross(right);
        Mat4::from_cols(
            right.extend(0.0),
            up.extend(0.0),
            forward.extend(0.0),
            eye.extend(1.0),
        )
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn face_toward_mat4() {
        use crate::{FaceToward, Mat4, Vec3, Vec4};

        // Completely arbitrary arguments
        let matrix = Mat4::face_toward(
            Vec3::new(50.0, 60.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        );

        assert_eq!(matrix.x_axis, Vec4::new(0.0, 0.0, -1.0, -0.0));
        assert_eq!(matrix.y_axis, Vec4::new(-0.7682213, 0.6401844, 0.0, 0.0));
        assert_eq!(matrix.z_axis, Vec4::new(0.6401844, 0.7682213, 0.0, 0.0));
        assert_eq!(matrix.w_axis, Vec4::new(50.0, 60.0, 0.0, 1.0));
    }
}
