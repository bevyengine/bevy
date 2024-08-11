//! Provides sensible defaults for mouse picking inputs.

use bevy_ecs::prelude::*;
use bevy_input::{mouse::MouseButtonInput, prelude::*, ButtonState};
use bevy_math::Vec2;
use bevy_render::camera::RenderTarget;
use bevy_window::{CursorMoved, PrimaryWindow, Window, WindowRef};

use crate::{
    pointer::{InputMove, InputPress, Location, PointerButton, PointerId},
    PointerBundle,
};

/// Spawns the default mouse pointer.
pub fn spawn_mouse_pointer(mut commands: Commands) {
    commands.spawn((PointerBundle::new(PointerId::Mouse),));
}

/// Sends mouse pointer events to be processed by the core plugin
pub fn mouse_pick_events(
    // Input
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut cursor_moves: EventReader<CursorMoved>,
    mut cursor_last: Local<Vec2>,
    mut mouse_inputs: EventReader<MouseButtonInput>,
    // Output
    mut pointer_move: EventWriter<InputMove>,
    mut pointer_presses: EventWriter<InputPress>,
) {
    for event in cursor_moves.read() {
        pointer_move.send(InputMove::new(
            PointerId::Mouse,
            Location {
                target: RenderTarget::Window(WindowRef::Entity(event.window))
                    .normalize(Some(
                        match windows.get_single() {
                            Ok(w) => w,
                            Err(_) => continue,
                        }
                        .0,
                    ))
                    .unwrap(),
                position: event.position,
            },
            event.position - *cursor_last,
        ));
        *cursor_last = event.position;
    }

    for input in mouse_inputs.read() {
        let button = match input.button {
            MouseButton::Left => PointerButton::Primary,
            MouseButton::Right => PointerButton::Secondary,
            MouseButton::Middle => PointerButton::Middle,
            MouseButton::Other(_) | MouseButton::Back | MouseButton::Forward => continue,
        };

        match input.state {
            ButtonState::Pressed => {
                pointer_presses.send(InputPress::new_down(PointerId::Mouse, button));
            }
            ButtonState::Released => {
                pointer_presses.send(InputPress::new_up(PointerId::Mouse, button));
            }
        }
    }
}
