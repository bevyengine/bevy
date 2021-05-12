use crate::{Mat4, Vec3};

/// Generates a translation / rotation matrix that faces a given target
pub trait FaceToward {
    /// Generates a translation / rotation matrix that faces a given target
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
