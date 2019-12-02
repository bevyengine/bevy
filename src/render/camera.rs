use crate::math;

pub enum CameraType {
    Projection {
        fov: f32,
        aspect_ratio: f32,
        near: f32,
        far: f32
    }
}

pub struct Camera {
    pub view_matrix: math::Mat4,
    pub camera_type: CameraType,
}

impl Camera {
    pub fn new(camera_type: CameraType) -> Self {
        Camera {
            view_matrix: math::identity(),
            camera_type,
        }
    }

    pub fn update(&mut self, width: u32, height: u32) {
        match &mut self.camera_type {
            CameraType::Projection { aspect_ratio, fov, near, far } => {
                *aspect_ratio = width as f32 / height as f32;
                self.view_matrix = get_projection_matrix(*fov, *aspect_ratio, *near, *far)
            }
        }
    }
}

pub fn get_projection_matrix(fov: f32, aspect_ratio: f32, near: f32, far: f32) -> math::Mat4 {
    let projection = math::perspective(aspect_ratio, fov, near, far);

    opengl_to_wgpu_matrix() * projection
}

pub fn opengl_to_wgpu_matrix() -> math::Mat4 {
    math::mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, -1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.0,
        0.0, 0.0, 0.5, 1.0,
    )
}