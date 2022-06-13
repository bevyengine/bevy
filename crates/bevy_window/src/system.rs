use crate::{
    PrimaryWindow, Window, WindowCloseRequested, WindowClosed, WindowCommandsExtension,
    WindowCurrentlyFocused, WindowDescriptor,
};

use bevy_app::AppExit;
use bevy_ecs::prelude::*;
use bevy_input::{keyboard::KeyCode, Input};

pub fn create_primary_window(mut commands: Commands, mut primary: ResMut<PrimaryWindow>) {

    bevy_utils::tracing::info!("Creating primary window");
    let entity = commands.spawn().id();

    commands
        .window(entity)
        .create_window(WindowDescriptor::default());

    // TODO: Maybe this should be controlled by window backend
    primary.window = Some(entity);
}

/// Exit the application when there are no open windows.
///
/// This system is added by the [`WindowPlugin`] in the default configuration.
/// To disable this behaviour, set `close_when_requested` (on the [`WindowPlugin`]) to `false`.
/// Ensure that you read the caveats documented on that field if doing so.
///
/// [`WindowPlugin`]: crate::WindowPlugin
pub fn exit_on_all_closed(mut app_exit_events: EventWriter<AppExit>, windows: Query<&Window>) {
    if windows.iter().count() == 0 {
        app_exit_events.send(AppExit);
    }
}

/// Exit the application when the primary window has been closed
///
/// This system is added by the [`WindowPlugin`]
// TODO: More docs
pub fn exit_on_primary_closed(
    mut app_exit_events: EventWriter<AppExit>,
    primary: Res<PrimaryWindow>,
    mut window_close: EventReader<WindowClosed>,
) {
    for window in window_close.iter() {
        if let Some(primary_window) = primary.window {
            if primary_window == window.entity {
                // Primary window has been closed
                app_exit_events.send(AppExit);
            }
        }
    }
}

/// Close windows in response to [`WindowCloseRequested`] (e.g.  when the close button is pressed).
///
/// This system is added by the [`WindowPlugin`] in the default configuration.
/// To disable this behaviour, set `close_when_requested` (on the [`WindowPlugin`]) to `false`.
/// Ensure that you read the caveats documented on that field if doing so.
///
/// [`WindowPlugin`]: crate::WindowPlugin
pub fn close_when_requested(mut commands: Commands, mut closed: EventReader<WindowCloseRequested>) {
    for event in closed.iter() {
        commands.window(event.entity).close();
    }
}

/// Close the focused window whenever the escape key (<kbd>Esc</kbd>) is pressed
///
/// This is useful for examples or prototyping.
pub fn close_on_esc(
    mut commands: Commands,
    focused_windows: Query<Entity, With<WindowCurrentlyFocused>>,
    // mut focused: Local<Option<WindowId>>,
    input: Res<Input<KeyCode>>,
) {
    // TODO: Not quite sure what this is about
    // TODO: Track this in e.g. a resource to ensure consistent behaviour across similar systems
    // for event in focused_events.iter() {
    //     *focused = event.focused.then(|| event.id);
    // }

    for focused_window in focused_windows.iter() {
        if input.just_pressed(KeyCode::Escape) {
            commands.window(focused_window).close();
        }
    }
}
