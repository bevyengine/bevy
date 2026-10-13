use crate::{
    ClosingWindow, CursorMoved, PrimaryWindow, RawCursorMoved, Window, WindowCloseRequested,
    WindowEvent,
};

use alloc::vec::Vec;
use bevy_app::AppExit;
use bevy_ecs::prelude::*;

/// A [`SystemSet`] for the system that exits the application.
/// Which can be either [`exit_on_all_closed`] or [`exit_on_primary_closed`].
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExitSystems;

/// A [`SystemSet`] for the system translating [`WindowEvent`] into different messages.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct WindowEventSystems;

/// Splits each [`WindowEvent`] into the matching typed message.
pub fn send_typed_window_events(
    mut window_events: MessageReader<WindowEvent>,
    mut commands: Commands,
) {
    if window_events.is_empty() {
        return;
    }
    let window_events: Vec<WindowEvent> = window_events.read().cloned().collect();

    // Calling directly queue instead of individual `commands.write_message` to queue a single command for all messages
    commands.queue(move |world: &mut World| {
        for window_event in window_events {
            match window_event {
                WindowEvent::AppLifecycle(e) => {
                    world.write_message(e);
                }
                WindowEvent::CursorEntered(e) => {
                    world.write_message(e);
                }
                WindowEvent::CursorLeft(e) => {
                    if let Some(mut window) = world.get_mut::<Window>(e.window) {
                        window.internal.physical_cursor_position = None;
                    }
                    world.write_message(e);
                }
                WindowEvent::CursorMoved(RawCursorMoved {
                    window,
                    physical_position,
                }) => {
                    let Some(mut window_component) = world.get_mut::<Window>(window) else {
                        continue;
                    };
                    let scale_factor = window_component.scale_factor();
                    let last_position = window_component.physical_cursor_position();
                    window_component.internal.physical_cursor_position = Some(physical_position);
                    world.write_message(CursorMoved {
                        window,
                        position: (physical_position / scale_factor as f64).as_vec2(),
                        delta: last_position.map(|last_position| {
                            (physical_position.as_vec2() - last_position) / scale_factor
                        }),
                    });
                }
                WindowEvent::FileDragAndDrop(e) => {
                    world.write_message(e);
                }
                WindowEvent::Ime(e) => {
                    world.write_message(e);
                }
                WindowEvent::RequestRedraw(e) => {
                    world.write_message(e);
                }
                WindowEvent::WindowBackendScaleFactorChanged(e) => {
                    world.write_message(e);
                }
                WindowEvent::WindowCloseRequested(e) => {
                    world.write_message(e);
                }
                WindowEvent::WindowCreated(e) => {
                    world.write_message(e);
                }
                WindowEvent::WindowDestroyed(e) => {
                    world.write_message(e);
                }
                WindowEvent::WindowFocused(e) => {
                    world.write_message(e);
                }
                WindowEvent::WindowMoved(e) => {
                    world.write_message(e);
                }
                WindowEvent::WindowOccluded(e) => {
                    world.write_message(e);
                }
                WindowEvent::WindowResized(e) => {
                    world.write_message(e);
                }
                WindowEvent::WindowScaleFactorChanged(e) => {
                    world.write_message(e);
                }
                WindowEvent::WindowThemeChanged(e) => {
                    world.write_message(e);
                }
                WindowEvent::MouseButtonInput(e) => {
                    world.write_message(e);
                }
                WindowEvent::MouseMotion(e) => {
                    world.write_message(e);
                }
                WindowEvent::MouseWheel(e) => {
                    world.write_message(e);
                }
                WindowEvent::PinchGesture(e) => {
                    world.write_message(e);
                }
                WindowEvent::RotationGesture(e) => {
                    world.write_message(e);
                }
                WindowEvent::DoubleTapGesture(e) => {
                    world.write_message(e);
                }
                WindowEvent::PanGesture(e) => {
                    world.write_message(e);
                }
                WindowEvent::TouchInput(e) => {
                    world.write_message(e);
                }
                WindowEvent::KeyboardInput(e) => {
                    world.write_message(e);
                }
                WindowEvent::KeyboardFocusLost(e) => {
                    world.write_message(e);
                }
            }
        }
    });
}

/// Exit the application when there are no open windows.
///
/// This system is added by the [`WindowPlugin`] in the default configuration.
/// To disable this behavior, set `close_when_requested` (on the [`WindowPlugin`]) to `false`.
/// Ensure that you read the caveats documented on that field if doing so.
///
/// [`WindowPlugin`]: crate::WindowPlugin
pub fn exit_on_all_closed(
    mut app_exit_writer: MessageWriter<AppExit>,
    windows: Query<(), With<Window>>,
) {
    if windows.is_empty() {
        log::info!("No windows are open, exiting");
        app_exit_writer.write(AppExit::Success);
    }
}

/// Exit the application when the primary window has been closed
///
/// This system is added by the [`WindowPlugin`]
///
/// [`WindowPlugin`]: crate::WindowPlugin
pub fn exit_on_primary_closed(
    mut app_exit_writer: MessageWriter<AppExit>,
    windows: Query<(), (With<Window>, With<PrimaryWindow>)>,
) {
    if windows.is_empty() {
        log::info!("Primary window was closed, exiting");
        app_exit_writer.write(AppExit::Success);
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
    mut closed: MessageReader<WindowCloseRequested>,
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
