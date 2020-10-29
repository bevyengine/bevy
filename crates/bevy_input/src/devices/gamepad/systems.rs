use super::*;
use crate::core::*;
use bevy_app::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};

#[allow(clippy::float_cmp)]
pub fn gamepad_event_system(
    mut event_reader: Local<EventReader<GamepadEventRaw>>,
    mut button_input: ResMut<BinaryInput<GamepadButton>>,
    mut axis: ResMut<Axis<GamepadAxis>>,
    mut button_axis: ResMut<Axis<GamepadButton>>,
    raw_events: Res<Events<GamepadEventRaw>>,
    mut events: ResMut<Events<GamepadEvent>>,
    settings: Res<GamepadSettings>,
) {
    button_input.update();
    for event in event_reader.iter(&raw_events) {
        let (gamepad, event) = (event.0, &event.1);
        match event {
            GamepadEventType::Connected => {
                events.send(GamepadEvent(gamepad, event.clone()));
                for button_type in ALL_BUTTON_TYPES.iter() {
                    let gamepad_button = GamepadButton(gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.set(gamepad_button, 0.0);
                }
                for axis_type in ALL_AXIS_TYPES.iter() {
                    axis.set(GamepadAxis(gamepad, *axis_type), 0.0);
                }
            }
            GamepadEventType::Disconnected => {
                events.send(GamepadEvent(gamepad, event.clone()));
                for button_type in ALL_BUTTON_TYPES.iter() {
                    let gamepad_button = GamepadButton(gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.remove(gamepad_button);
                }
                for axis_type in ALL_AXIS_TYPES.iter() {
                    axis.remove(GamepadAxis(gamepad, *axis_type));
                }
            }
            GamepadEventType::AxisChanged(axis_type, value) => {
                let gamepad_axis = GamepadAxis(gamepad, *axis_type);
                let old_value = axis.get(gamepad_axis);
                let filtered_value = settings
                    .get_axis_settings(gamepad_axis)
                    .filter(*value, old_value);
                axis.set(gamepad_axis, filtered_value);

                // only send event if axis has changed after going through filters
                if let Some(old_value) = old_value {
                    if old_value == filtered_value {
                        return;
                    }
                } else if filtered_value == 0.0 {
                    return;
                }

                events.send(GamepadEvent(
                    gamepad,
                    GamepadEventType::AxisChanged(*axis_type, filtered_value),
                ))
            }
            GamepadEventType::ButtonChanged(button_type, value) => {
                let gamepad_button = GamepadButton(gamepad, *button_type);
                let old_value = button_axis.get(gamepad_button);
                let filtered_value = settings
                    .get_button_axis_settings(gamepad_button)
                    .filter(*value, old_value);
                button_axis.set(gamepad_button, filtered_value);

                let button_property = settings.get_button_settings(gamepad_button);
                if button_input.pressed(gamepad_button) {
                    if button_property.is_released(*value) {
                        button_input.release(gamepad_button);
                    }
                } else if button_property.is_pressed(*value) {
                    button_input.press(gamepad_button);
                }

                // only send event if axis has changed after going through filters
                if let Some(old_value) = old_value {
                    if old_value == filtered_value {
                        return;
                    }
                } else if filtered_value == 0.0 {
                    return;
                }

                events.send(GamepadEvent(
                    gamepad,
                    GamepadEventType::ButtonChanged(*button_type, filtered_value),
                ))
            }
        }
    }
}
