use crate::{WindowCloseRequested, WindowFocused, Windows};
use bevy_app::{
    prelude::{EventReader, Events},
    AppExit,
};
use bevy_ecs::ResMut;

pub fn exit_on_window_close_system(
    mut app_exit_events: ResMut<Events<AppExit>>,
    mut window_close_requested_events: EventReader<WindowCloseRequested>,
) {
    if window_close_requested_events.iter().next().is_some() {
        app_exit_events.send(AppExit);
    }
}

pub fn window_focus_update_system(
    mut window_focus_events: EventReader<WindowFocused>,
    mut windows: ResMut<Windows>,
) {
    for event in window_focus_events.iter() {
        windows.get_mut(event.id).unwrap().focused = event.focused;
    }
}