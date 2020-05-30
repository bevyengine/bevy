use bevy_app::{Events, GetEventReader};
use bevy_property::Properties;
use bevy_window::WindowResized;
use glam::Mat4;
use legion::{prelude::*, storage::Component};

#[derive(Debug, Clone, Properties)]
pub struct OrthographicCamera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

impl CameraProjection for OrthographicCamera {
    fn get_view_matrix(&self) -> Mat4 {
        let projection = Mat4::orthographic_rh_gl(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.near,
            self.far,
        );
        projection
    }
    fn update(&mut self, width: usize, height: usize) {
        self.right = width as f32;
        self.top = height as f32;
    }
}

impl Default for OrthographicCamera {
    fn default() -> Self {
        OrthographicCamera {
            left: 0.0,
            right: 0.0,
            bottom: 0.0,
            top: 0.0,
            near: 0.0,
            far: 1.0,
        }
    }
}

pub trait CameraProjection {
    fn get_view_matrix(&self) -> Mat4;
    fn update(&mut self, width: usize, height: usize);
}

#[derive(Debug, Clone, Properties)]
pub struct PerspectiveCamera {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl CameraProjection for PerspectiveCamera {
    fn get_view_matrix(&self) -> Mat4 {
        let projection = Mat4::perspective_rh_gl(self.fov, self.aspect_ratio, self.near, self.far);
        projection
    }
    fn update(&mut self, width: usize, height: usize) {
        self.aspect_ratio = width as f32 / height as f32;
    }
}

impl Default for PerspectiveCamera {
    fn default() -> Self {
        PerspectiveCamera {
            fov: std::f32::consts::PI / 4.0,
            near: 1.0,
            far: 1000.0,
            aspect_ratio: 1.0,
        }
    }
}

#[derive(Default, Debug, Properties)]
pub struct Camera {
    pub view_matrix: Mat4,
    pub name: Option<String>,
}

pub fn camera_system<T: CameraProjection + Component>(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut window_resized_event_reader = resources.get_event_reader::<WindowResized>();
    (move |world: &mut SubWorld,
           window_resized_events: Res<Events<WindowResized>>,
           query: &mut Query<(Write<Camera>, Write<T>)>| {
        let primary_window_resized_event = window_resized_event_reader
            .find_latest(&window_resized_events, |event| event.is_primary);
        if let Some(primary_window_resized_event) = primary_window_resized_event {
            for (mut camera, mut camera_projection) in query.iter_mut(world) {
                camera_projection.update(primary_window_resized_event.width, primary_window_resized_event.height);
                camera.view_matrix = camera_projection.get_view_matrix();
            }
        }
    })
    .system()
}