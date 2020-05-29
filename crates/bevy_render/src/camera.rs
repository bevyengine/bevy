use bevy_app::{Events, GetEventReader};
use bevy_property::{Properties, Property};
use bevy_window::WindowResized;
use glam::Mat4;
use legion::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Default, Properties)]
pub struct ActiveCamera;

#[derive(Default, Properties)]
pub struct ActiveCamera2d;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrthographicCamera {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
    pub near: f32,
    pub far: f32,
}

impl OrthographicCamera {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerspectiveCamera {
    pub fov: f32,
    pub aspect_ratio: f32,
    pub near: f32,
    pub far: f32,
}

impl PerspectiveCamera {
    pub fn get_view_matrix(&self) -> Mat4 {
        let projection = Mat4::perspective_rh_gl(self.fov, self.aspect_ratio, self.near, self.far);
        projection
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

#[derive(Debug, Clone, Serialize, Deserialize, Property)]
pub enum CameraType {
    Perspective(PerspectiveCamera),
    Orthographic(OrthographicCamera),
}

impl CameraType {
    pub fn default_perspective() -> CameraType {
        CameraType::Perspective(PerspectiveCamera::default())
    }

    pub fn default_orthographic() -> CameraType {
        CameraType::Orthographic(OrthographicCamera::default())
    }
}

impl Default for CameraType {
    fn default() -> Self {
        CameraType::default_perspective()
    }
}

#[derive(Default, Debug, Properties)]
pub struct Camera {
    pub view_matrix: Mat4,
    pub camera_type: CameraType,
}

impl Camera {
    pub fn new(camera_type: CameraType) -> Self {
        Camera {
            view_matrix: Mat4::identity(),
            camera_type,
        }
    }

    pub fn update(&mut self, width: u32, height: u32) {
        self.view_matrix = match &mut self.camera_type {
            CameraType::Perspective(projection) => {
                projection.aspect_ratio = width as f32 / height as f32;
                projection.get_view_matrix()
            }
            CameraType::Orthographic(orthographic) => {
                orthographic.right = width as f32;
                orthographic.top = height as f32;
                orthographic.get_view_matrix()
            }
        }
    }
}

pub fn camera_update_system(resources: &mut Resources) -> Box<dyn Schedulable> {
    let mut window_resized_event_reader = resources.get_event_reader::<WindowResized>();
    (move |world: &mut SubWorld,
           window_resized_events: Res<Events<WindowResized>>,
           query: &mut Query<Write<Camera>>| {
        let primary_window_resized_event = window_resized_event_reader
            .find_latest(&window_resized_events, |event| event.is_primary);
        if let Some(primary_window_resized_event) = primary_window_resized_event {
            for mut camera in query.iter_mut(world) {
                camera.update(
                    primary_window_resized_event.width,
                    primary_window_resized_event.height,
                );
            }
        }
    })
    .system()
}
