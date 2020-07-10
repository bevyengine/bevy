use crate::WindowCloseRequested;
use bevy_app::{AppExit, EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};

#[derive(Default)]
pub struct ExitOnWindowCloseState {
    event_reader: EventReader<WindowCloseRequested>,
}

pub fn exit_on_window_close_system(
    mut state: Local<ExitOnWindowCloseState>,
    mut app_exit_events: ResMut<Events<AppExit>>,
    window_close_requested_events: Res<Events<WindowCloseRequested>>,
) {
    for _ in state.event_reader.iter(&window_close_requested_events) {
        app_exit_events.send(AppExit);
        break;
    }
}
