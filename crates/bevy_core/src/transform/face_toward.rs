use glam::{Vec3, Mat4};

pub trait FaceToward {
    fn face_toward(eye: Vec3, center: Vec3, up: Vec3) -> Self;
}

impl FaceToward for Mat4 {
    fn face_toward(eye: Vec3, center: Vec3, up: Vec3) -> Self {
        let forward = (center - eye).normalize();
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
