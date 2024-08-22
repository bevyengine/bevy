//! Provides sensible defaults for mouse picking inputs.

use bevy_ecs::prelude::*;
use bevy_input::{prelude::*, ButtonState};
use bevy_math::Vec2;
use bevy_render::camera::RenderTarget;
use bevy_window::{PrimaryWindow, WindowEvent, WindowRef};

use crate::{
    pointer::{Location, PointerAction, PointerButton, PointerId, PointerInput, PressDirection},
    PointerBundle,
};

/// Spawns the default mouse pointer.
pub fn spawn_mouse_pointer(mut commands: Commands) {
    commands.spawn((PointerBundle::new(PointerId::Mouse),));
}

/// Sends mouse pointer events to be processed by the core plugin
pub fn mouse_pick_events(
    // Input
    mut window_events: EventReader<WindowEvent>,
    primary_window: Query<Entity, With<PrimaryWindow>>,
    // Locals
    mut cursor_last: Local<Vec2>,
    // Output
    mut pointer_events: EventWriter<PointerInput>,
) {
    for window_event in window_events.read() {
        match window_event {
            // Handle the cursor entering the window
            WindowEvent::CursorEntered(event) => {
                let location = Location {
                    target: match RenderTarget::Window(WindowRef::Entity(event.window))
                        .normalize(primary_window.get_single().ok())
                    {
                        Some(target) => target,
                        None => continue,
                    },
                    position: *cursor_last, // Note, this is a hack until winit starts providing locations
                };
                pointer_events.send(PointerInput::new(
                    PointerId::Mouse,
                    location,
                    PointerAction::EnteredWindow,
                ));
            }
            // Handle curcor leaving the window
            WindowEvent::CursorLeft(event) => {
                let location = Location {
                    target: match RenderTarget::Window(WindowRef::Entity(event.window))
                        .normalize(primary_window.get_single().ok())
                    {
                        Some(target) => target,
                        None => continue,
                    },
                    position: *cursor_last, // Note, this is a hack until winit starts providing locations
                };
                pointer_events.send(PointerInput::new(
                    PointerId::Mouse,
                    location,
                    PointerAction::LeftWindow,
                ));
            }
            // Handle cursor movement events
            WindowEvent::CursorMoved(event) => {
                let location = Location {
                    target: match RenderTarget::Window(WindowRef::Entity(event.window))
                        .normalize(primary_window.get_single().ok())
                    {
                        Some(target) => target,
                        None => continue,
                    },
                    position: event.position,
                };
                pointer_events.send(PointerInput::new(
                    PointerId::Mouse,
                    location,
                    PointerAction::Moved {
                        delta: event.position - *cursor_last,
                    },
                ));
                *cursor_last = event.position;
            }
            // Handle mouse button press events
            WindowEvent::MouseButtonInput(input) => {
                let location = Location {
                    target: match RenderTarget::Window(WindowRef::Entity(input.window))
                        .normalize(primary_window.get_single().ok())
                    {
                        Some(target) => target,
                        None => continue,
                    },
                    position: *cursor_last,
                };
                let button = match input.button {
                    MouseButton::Left => PointerButton::Primary,
                    MouseButton::Right => PointerButton::Secondary,
                    MouseButton::Middle => PointerButton::Middle,
                    MouseButton::Other(_) | MouseButton::Back | MouseButton::Forward => continue,
                };
                let direction = match input.state {
                    ButtonState::Pressed => PressDirection::Down,
                    ButtonState::Released => PressDirection::Up,
                };
                pointer_events.send(PointerInput::new(
                    PointerId::Mouse,
                    location,
                    PointerAction::Pressed { direction, button },
                ));
            }
            _ => {}
        }
    }
}
