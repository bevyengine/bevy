use crate::{Window, WindowCloseRequested, WindowFocused, WindowId, Windows};

use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use bevy_input::{keyboard::KeyCode, Input};

/// Exit the application when there are no open windows.
///
/// This system is added by the [`WindowPlugin`] in the default configuration.
/// To disable this behaviour, set `close_when_requested` (on the [`WindowPlugin`]) to `false`.
/// Ensure that you read the caveats documented on that field if doing so.
///
/// [`WindowPlugin`]: crate::WindowPlugin
pub fn exit_on_all_closed(mut app_exit_events: EventWriter<AppExit>, windows: Res<Windows>) {
    if windows.iter().count() == 0 {
        app_exit_events.send(AppExit);
    }
}

/// Close windows in response to [`WindowCloseRequested`] (e.g.  when the close button is pressed).
///
/// This system is added by the [`WindowPlugin`] in the default configuration.
/// To disable this behaviour, set `close_when_requested` (on the [`WindowPlugin`]) to `false`.
/// Ensure that you read the caveats documented on that field if doing so.
///
/// [`WindowPlugin`]: crate::WindowPlugin
pub fn close_when_requested(
    mut windows: ResMut<Windows>,
    mut closed: EventReader<WindowCloseRequested>,
) {
    for event in closed.iter() {
        windows.get_mut(event.id).map(Window::close);
    }
}

/// Close the focused window whenever the escape key (<kbd>Esc</kbd>) is pressed
///
/// This is useful for examples or prototyping.
pub fn close_on_esc(
    mut focused: Local<Option<WindowId>>,
    mut focused_events: EventReader<WindowFocused>,
    mut windows: ResMut<Windows>,
    input: Res<Input<KeyCode>>,
) {
    // TODO: Track this in e.g. a resource to ensure consistent behaviour across similar systems
    for event in focused_events.iter() {
        *focused = event.focused.then_some(event.id);
    }

    if let Some(focused) = &*focused {
        if input.just_pressed(KeyCode::Escape) {
            if let Some(window) = windows.get_mut(*focused) {
                window.close();
            }
        }
    }
}
