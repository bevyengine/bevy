use crate::{ClosingWindow, PrimaryWindow, Window, WindowCloseRequested};

use bevy_app::AppExit;
use bevy_ecs::prelude::*;

/// Exit the application when there are no open windows.
///
/// This system is added by the [`WindowPlugin`] in the default configuration.
/// To disable this behavior, set `close_when_requested` (on the [`WindowPlugin`]) to `false`.
/// Ensure that you read the caveats documented on that field if doing so.
///
/// [`WindowPlugin`]: crate::WindowPlugin
pub fn exit_on_all_closed(mut app_exit_events: EventWriter<AppExit>, windows: Query<&Window>) {
    if windows.is_empty() {
        bevy_utils::tracing::info!("No windows are open, exiting");
        app_exit_events.send(AppExit::Success);
    }
}

/// Exit the application when the primary window has been closed
///
/// This system is added by the [`WindowPlugin`]
///
/// [`WindowPlugin`]: crate::WindowPlugin
pub fn exit_on_primary_closed(
    mut app_exit_events: EventWriter<AppExit>,
    windows: Query<(), (With<Window>, With<PrimaryWindow>)>,
) {
    if windows.is_empty() {
        bevy_utils::tracing::info!("Primary window was closed, exiting");
        app_exit_events.send(AppExit::Success);
    }
}

/// Close windows in response to [`WindowCloseRequested`] (e.g.  when the close button is pressed).
///
/// This system is added by the [`WindowPlugin`] in the default configuration.
/// To disable this behavior, set `close_when_requested` (on the [`WindowPlugin`]) to `false`.
/// Ensure that you read the caveats documented on that field if doing so.
///
/// [`WindowPlugin`]: crate::WindowPlugin
pub fn close_when_requested(
    mut commands: Commands,
    mut closed: EventReader<WindowCloseRequested>,
    closing: Query<Entity, With<ClosingWindow>>,
) {
    // This was inserted by us on the last frame so now we can despawn the window
    for window in closing.iter() {
        commands.entity(window).despawn();
    }
    // Mark the window as closing so we can despawn it on the next frame
    for event in closed.read() {
        // When spamming the window close button on windows (other platforms too probably)
        // we may receive a `WindowCloseRequested` for a window we've just despawned in the above
        // loop.
        commands.entity(event.window).try_insert(ClosingWindow);
    }
}
