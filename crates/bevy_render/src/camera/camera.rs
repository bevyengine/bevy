use crate::CameraProjection;
use bevy_app::{EventReader, Events};
use bevy_ecs::{Component, Local, Query, Res};
use bevy_property::Properties;
use bevy_window::{WindowCreated, WindowReference, WindowResized, Windows};
use bevy_math::Mat4;

#[derive(Default, Debug, Properties)]
pub struct Camera {
    pub projection_matrix: Mat4,
    pub name: Option<String>,
    #[property(ignore)]
    pub window: WindowReference,
}

#[derive(Default)]
pub struct CameraSystemState {
    window_resized_event_reader: EventReader<WindowResized>,
    window_created_event_reader: EventReader<WindowCreated>,
}

pub fn camera_system<T: CameraProjection + Component>(
    mut state: Local<CameraSystemState>,
    window_resized_events: Res<Events<WindowResized>>,
    window_created_events: Res<Events<WindowCreated>>,
    windows: Res<Windows>,
    mut query: Query<(&mut Camera, &mut T)>,
) {
    let mut changed_window_ids = Vec::new();
    let mut changed_primary_window_id = None;
    // handle resize events. latest events are handled first because we only want to resize each window once
    for event in state
        .window_resized_event_reader
        .iter(&window_resized_events)
        .rev()
    {
        if changed_window_ids.contains(&event.id) {
            continue;
        }

        if event.is_primary {
            changed_primary_window_id = Some(event.id);
        } else {
            changed_window_ids.push(event.id);
        }
    }

    // handle resize events. latest events are handled first because we only want to resize each window once
    for event in state
        .window_created_event_reader
        .iter(&window_created_events)
        .rev()
    {
        if changed_window_ids.contains(&event.id) {
            continue;
        }

        if event.is_primary {
            changed_primary_window_id = Some(event.id);
        } else {
            changed_window_ids.push(event.id);
        }
    }

    for (mut camera, camera_projection) in &mut query.iter() {
        if let Some(window) = match camera.window {
            WindowReference::Id(id) => {
                if changed_window_ids.contains(&id) {
                    windows.get(id)
                } else {
                    None
                }
            }
            WindowReference::Primary => {
                if let Some(id) = changed_primary_window_id {
                    windows.get(id)
                } else {
                    None
                }
            }
        } {
            camera_projection.update(window.width as usize, window.height as usize);
            camera.projection_matrix = camera_projection.get_projection_matrix();
        }
    }
}
