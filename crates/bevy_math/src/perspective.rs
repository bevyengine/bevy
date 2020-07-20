use crate::Mat4;
use glam::Vec4;

pub trait PerspectiveRh {
    fn perspective_rh(fov_y_radians: f32, aspect_ratio: f32, z_near: f32, z_far: f32) -> Self;
}

impl PerspectiveRh for Mat4 {
    fn perspective_rh(fov_y_radians: f32, aspect_ratio: f32, z_near: f32, z_far: f32) -> Self {
        let (sin_fov, cos_fov) = (0.5 * fov_y_radians).sin_cos();
        let h = cos_fov / sin_fov;
        let w = h /aspect_ratio;
        let r = z_far / (z_near - z_far);
        Mat4::from_cols(
            Vec4::new(w, 0.0, 0.0, 0.0),
            Vec4::new(0.0, h, 0.0, 0.0),
            Vec4::new(0.0, 0.0, r, -1.0),
            Vec4::new(0.0, 0.0, r * z_near, 0.0),
        )
    }
}
