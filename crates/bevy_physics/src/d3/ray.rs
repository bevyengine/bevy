use bevy_render::camera::Camera;
use bevy_transform::components::GlobalTransform;
use bevy_window::Window;
use glam::{Vec2, Vec3};

pub struct Ray {
    origin: Vec3,
    direction: Vec3,
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3) -> Self {
        direction.normalize();
        Self { origin, direction }
    }

    pub fn from_window(
        window: &Window,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Self {
        Self::from_mouse_position(window.cursor_position(), camera, camera_transform)
    }

    pub fn from_mouse_position(
        mouse_position: &Vec2,
        window: &Window,
        camera: &Camera,
        camera_transform: &GlobalTransform,
    ) -> Self {
        if window.id() != camera.window {
            panic!("Generating Ray from Camera with wrong Window");
        }

        let x = 2.0 * (mouse_position.x / window.width() as f32) - 1.0;
        let y = 2.0 * (mouse_position.y / window.height() as f32) - 1.0;

        let camera_inverse_matrix =
            camera_transform.compute_matrix() * camera.projection_matrix.inverse();
        let near = camera_inverse_matrix * Vec3::new(x, y, 0.0).extend(1.0);
        let far = camera_inverse_matrix * Vec3::new(x, y, 1.0).extend(1.0);

        let near = near.truncate() / near.w;
        let far = far.truncate() / far.w;

        let direction: Vec3 = (far - near).into();
        let origin: Vec3 = near.into();

        return Self { origin, direction };
    }

    pub fn origin(&self) -> &Vec3 {
        &self.origin
    }

    pub fn origin_mut(&mut self) -> &mut Vec3 {
        &mut self.origin
    }

    pub fn direction(&self) -> &Vec3 {
        &self.direction
    }

    pub fn set_direction(&mut self, direction: Vec3) {
        direction.normalize();
        self.direction = direction;
    }
}
