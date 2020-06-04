use crate::CameraProjection;
use bevy_app::{EventReader, Events};
use bevy_property::Properties;
use bevy_window::WindowResized;
use glam::Mat4;
use legion::{prelude::*, storage::Component};

#[derive(Default, Debug, Properties)]
pub struct Camera {
    pub view_matrix: Mat4,
    pub name: Option<String>,
}

pub fn camera_system<T: CameraProjection + Component>(
    _resources: &mut Resources,
) -> Box<dyn Schedulable> {
    let mut window_resized_event_reader = EventReader::<WindowResized>::default();
    (move |world: &mut SubWorld,
           window_resized_events: Res<Events<WindowResized>>,
           query: &mut Query<(Write<Camera>, Write<T>)>| {
        let primary_window_resized_event = window_resized_event_reader
            .find_latest(&window_resized_events, |event| event.is_primary);
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
