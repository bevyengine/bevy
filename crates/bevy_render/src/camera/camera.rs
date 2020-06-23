use crate::CameraProjection;
use bevy_app::{EventReader, Events};
use bevy_property::Properties;
use bevy_window::{WindowCreated, WindowResized, Windows};
use glam::Mat4;
use legion::{prelude::*, storage::Component};

#[derive(Default, Debug, Properties)]
pub struct Camera {
    pub view_matrix: Mat4,
    pub name: Option<String>,
}

pub fn camera_system<T: CameraProjection + Component>() -> Box<dyn Schedulable> {
    let mut window_resized_event_reader = EventReader::<WindowResized>::default();
    let mut window_created_event_reader = EventReader::<WindowCreated>::default();
    (move |world: &mut SubWorld,
           window_resized_events: Res<Events<WindowResized>>,
           window_created_events: Res<Events<WindowCreated>>,
           windows: Res<Windows>,
           query: &mut Query<(Write<Camera>, Write<T>)>| {
        let primary_window_resized_event = window_resized_event_reader
            .find_latest(&window_resized_events, |event| event.is_primary);

        for event in window_created_event_reader.iter(&window_created_events) {
            if !event.is_primary {
                continue;
            }
            if let Some(window) = windows.get(event.id) {
                for (mut camera, mut camera_projection) in query.iter_mut(world) {
                    camera_projection.update(window.width as usize, window.height as usize);
                    camera.view_matrix = camera_projection.get_view_matrix();
                }
            }
        }

        if let Some(primary_window_resized_event) = primary_window_resized_event {
            for (mut camera, mut camera_projection) in query.iter_mut(world) {
                camera_projection.update(
                    primary_window_resized_event.width,
                    primary_window_resized_event.height,
                );
                camera.view_matrix = camera_projection.get_view_matrix();
            }
        }
    })
    .system()
}
