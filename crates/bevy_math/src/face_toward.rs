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
