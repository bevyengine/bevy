use crate::{Window, WindowCloseRequested, WindowFocused, WindowId, Windows};
use bevy_app::{AppExit, EventReader, EventWriter};
use bevy_ecs::prelude::*;
use bevy_input::{keyboard::KeyCode, Input};

pub fn exit_on_all_closed(mut app_exit_events: EventWriter<AppExit>, windows: Res<Windows>) {
    if windows.iter().count() == 0 {
        app_exit_events.send(AppExit);
    }
}

pub fn close_when_requested(
    mut windows: ResMut<Windows>,
    mut closed: EventReader<WindowCloseRequested>,
) {
    for event in closed.iter() {
        windows.get_mut(event.id).map(Window::close);
    }
}

pub fn close_on_esc(
    mut focused: Local<Option<WindowId>>,
    mut focused_events: EventReader<WindowFocused>,
    mut windows: ResMut<Windows>,
    input: Res<Input<KeyCode>>,
) {
    for event in focused_events.iter() {
        if event.focused {
            *focused = Some(event.id);
        }
    }

    if let Some(focused) = &*focused {
        if input.just_pressed(KeyCode::Escape) {
            if let Some(window) = windows.get_mut(*focused) {
                window.close();
            }
        }
    }
}
