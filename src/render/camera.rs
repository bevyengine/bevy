use crate::math;

pub fn get_projection_view_matrix(eye: &math::Vec3, fov: f32, aspect_ratio: f32, near: f32, far: f32) -> math::Mat4 {
    let projection = math::perspective(aspect_ratio, fov, near, far);

    let view = math::look_at_rh::<f32>(
        &eye,
        &math::vec3(0.0, 0.0, 0.0),
        &math::vec3(0.0, 0.0, 1.0),
    );

    opengl_to_wgpu_matrix() * projection * view
}

pub fn opengl_to_wgpu_matrix() -> math::Mat4 {
    math::mat4(
        1.0, 0.0, 0.0, 0.0,
        0.0, -1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.0,
        0.0, 0.0, 0.5, 1.0,
    )
}