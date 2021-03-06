use crate::WindowCloseRequested;
use bevy_app::{
    prelude::{EventReader, Events},
    AppExit,
};
use bevy_ecs::system::ResMut;

pub fn exit_on_window_close_system(
    mut app_exit_events: ResMut<Events<AppExit>>,
    mut window_close_requested_events: EventReader<WindowCloseRequested>,
) {
    if window_close_requested_events.iter().next().is_some() {
        app_exit_events.send(AppExit);
    }
}
