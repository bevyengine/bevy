use crate::WindowCloseRequested;
use bevy_app::{
    prelude::{EventReader, Events},
    AppExit,
};
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
    if state
        .event_reader
        .iter(&window_close_requested_events)
        .next()
        .is_some()
    {
        app_exit_events.send(AppExit);
    }
}
