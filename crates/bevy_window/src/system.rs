use crate::{WindowCloseRequested, WindowId};
use bevy_app::{AppExit, EventReader, EventWriter};

pub fn exit_on_primary_window_close_system(
    mut app_exit_events: EventWriter<AppExit>,
    mut window_close_requested_events: EventReader<WindowCloseRequested>,
) {
    for WindowCloseRequested { id } in window_close_requested_events.iter() {
        if id.is_primary() {
            app_exit_events.send(AppExit);
        }

        // TODO: Remove window from Res<Windows>
    }
}

pub fn exit_on_last_window_close_system(
    mut app_exit_events: EventWriter<AppExit>,
    mut window_close_requested_events: EventReader<WindowCloseRequested>,
) {
    for WindowCloseRequested { id } in window_close_requested_events.iter() {
        // TODO: Use Res<Windows> to check if last window -> exit

        // TODO: Remove window from Res<Windows>
    }
}

pub fn exit_on_any_window_close_system(
    mut app_exit_events: EventWriter<AppExit>,
    mut window_close_requested_events: EventReader<WindowCloseRequested>,
) {
    if window_close_requested_events.iter().next().is_some() {
        app_exit_events.send(AppExit);
    }
}

pub fn exit_on_window_close_system(
    window_id: WindowId,
    mut app_exit_events: EventWriter<AppExit>,
    mut window_close_requested_events: EventReader<WindowCloseRequested>,
) {
    for WindowCloseRequested { id } in window_close_requested_events.iter() {
        if id == &window_id {
            app_exit_events.send(AppExit);
        }

        // TODO: Remove window from Res<Windows>
    }
}