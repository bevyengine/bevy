use super::keyboard::ElementState;
use crate::{Axis, Input};
use bevy_app::prelude::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_math::Vec2;
use bevy_window::{AxisId, Cursor, CursorMoved, Motion};

/// A mouse button input event
#[derive(Debug, Clone)]
pub struct MouseButtonInput {
    pub button: MouseButton,
    pub state: ElementState,
}

/// A button on a mouse device
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

/// A mouse motion event
#[derive(Debug, Clone)]
pub struct MouseMotion {
    pub delta: Vec2,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Mouse {
    Vertical,
    Horizontal,
}
/// Unit of scroll
#[derive(Debug, Clone)]
pub enum MouseScrollUnit {
    Line,
    Pixel,
}

/// A mouse scroll wheel event, where x represents horizontal scroll and y represents vertical scroll.
#[derive(Debug, Clone)]
pub struct MouseWheel {
    pub unit: MouseScrollUnit,
    pub x: f32,
    pub y: f32,
}

/// State used by the mouse button input system
#[derive(Default)]
pub struct MouseButtonInputState {
    mouse_button_input_event_reader: EventReader<MouseButtonInput>,
}

#[derive(Default)]
pub struct MouseMovementState {
    mouse_movement_event_reader: EventReader<MouseMotion>,
}

#[derive(Default)]
pub struct CursorMovementState {
    cursor_movement_event_reader: EventReader<CursorMoved>,
}

#[derive(Default)]
pub struct AxisState {
    joystick_movement_event_reader: EventReader<Motion>,
}

/// Updates the Input<MouseButton> resource with the latest MouseButtonInput events
pub fn mouse_button_input_system(
    mut state: Local<MouseButtonInputState>,
    mut mouse_button_input: ResMut<Input<MouseButton>>,
    mouse_button_input_events: Res<Events<MouseButtonInput>>,
) {
    mouse_button_input.update();
    for event in state
        .mouse_button_input_event_reader
        .iter(&mouse_button_input_events)
    {
        match event.state {
            ElementState::Pressed => mouse_button_input.press(event.button),
            ElementState::Released => mouse_button_input.release(event.button),
        }
    }
}

/// Updates the Axis<Mouse::Horizontal> and Axis<Mouse::Vertical> with raw data
/// Can be different on other platforms
/// Should be used for in-game movement with mouse
pub fn mouse_axis_system(
    mut state: Local<MouseMovementState>,
    mut mouse_axis: ResMut<Axis<Mouse>>,
    mouse_movement_events: Res<Events<MouseMotion>>,
) {
    for event in state
        .mouse_movement_event_reader
        .iter(&mouse_movement_events)
    {
        mouse_axis.add(Mouse::Horizontal, event.delta.x());
        mouse_axis.add(Mouse::Vertical, event.delta.y());
    }
}

/// Updates the Axis<Cursor::Horizontal(WindowId)> and Axis<Cursor::Vertical(WindowId)> with data
/// Is platform independent, always returns correct possition
/// Unless cursor is outside of that `Window` then it is the last position within
pub fn cursor_system(
    mut state: Local<CursorMovementState>,
    mut cursor_axis: ResMut<Axis<Cursor>>,
    cursor_movement_events: Res<Events<CursorMoved>>,
) {
    for event in state
        .cursor_movement_event_reader
        .iter(&cursor_movement_events)
    {
        cursor_axis.add(event.id, event.position);
    }
}

/// Updates all Axis<AxisId>
pub fn axis_system(
    mut state: Local<AxisState>,
    mut axis: ResMut<Axis<AxisId>>,
    axis_events: Res<Events<Motion>>,
) {
    for event in state.joystick_movement_event_reader.iter(&axis_events) {
        axis.add(event.axis, event.value);
    }
}
