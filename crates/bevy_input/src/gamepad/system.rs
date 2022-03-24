use crate::gamepad::{
    GamepadAxis, GamepadButton, GamepadEvent, GamepadEventRaw, GamepadEventType, GamepadSettings,
    Gamepads, ALL_AXIS_TYPES, ALL_BUTTON_TYPES,
};
use crate::{Axis, Input};
use bevy_ecs::{
    event::{EventReader, EventWriter},
    system::{Res, ResMut},
};
use bevy_utils::tracing::info;

pub fn gamepad_event_system(
    mut button_input: ResMut<Input<GamepadButton>>,
    mut axis: ResMut<Axis<GamepadAxis>>,
    mut button_axis: ResMut<Axis<GamepadButton>>,
    mut raw_events: EventReader<GamepadEventRaw>,
    mut events: EventWriter<GamepadEvent>,
    settings: Res<GamepadSettings>,
) {
    button_input.clear();
    for event in raw_events.iter() {
        let (gamepad, event) = (event.0, &event.1);
        match event {
            GamepadEventType::Connected => {
                events.send(GamepadEvent(gamepad, event.clone()));
                for button_type in &ALL_BUTTON_TYPES {
                    let gamepad_button = GamepadButton(gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.set(gamepad_button, 0.0);
                }
                for axis_type in &ALL_AXIS_TYPES {
                    axis.set(GamepadAxis(gamepad, *axis_type), 0.0);
                }
            }
            GamepadEventType::Disconnected => {
                events.send(GamepadEvent(gamepad, event.clone()));
                for button_type in &ALL_BUTTON_TYPES {
                    let gamepad_button = GamepadButton(gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.remove(gamepad_button);
                }
                for axis_type in &ALL_AXIS_TYPES {
                    axis.remove(GamepadAxis(gamepad, *axis_type));
                }
            }
            GamepadEventType::AxisChanged(axis_type, value) => {
                let gamepad_axis = GamepadAxis(gamepad, *axis_type);
                if let Some(filtered_value) = settings
                    .get_axis_settings(gamepad_axis)
                    .filter(*value, axis.get(gamepad_axis))
                {
                    axis.set(gamepad_axis, filtered_value);
                    events.send(GamepadEvent(
                        gamepad,
                        GamepadEventType::AxisChanged(*axis_type, filtered_value),
                    ));
                }
            }
            GamepadEventType::ButtonChanged(button_type, value) => {
                let gamepad_button = GamepadButton(gamepad, *button_type);
                if let Some(filtered_value) = settings
                    .get_button_axis_settings(gamepad_button)
                    .filter(*value, button_axis.get(gamepad_button))
                {
                    button_axis.set(gamepad_button, filtered_value);
                    events.send(GamepadEvent(
                        gamepad,
                        GamepadEventType::ButtonChanged(*button_type, filtered_value),
                    ));
                }

                let button_property = settings.get_button_settings(gamepad_button);
                if button_input.pressed(gamepad_button) {
                    if button_property.is_released(*value) {
                        button_input.release(gamepad_button);
                    }
                } else if button_property.is_pressed(*value) {
                    button_input.press(gamepad_button);
                }
            }
        }
    }
}

/// Monitors gamepad connection and disconnection events, updating the [`Gamepads`] resource accordingly
///
/// By default, runs during `CoreStage::PreUpdate` when added via [`InputPlugin`](crate::InputPlugin).
pub fn gamepad_connection_system(
    mut gamepads: ResMut<Gamepads>,
    mut gamepad_event: EventReader<GamepadEvent>,
) {
    for event in gamepad_event.iter() {
        match &event {
            GamepadEvent(gamepad, GamepadEventType::Connected) => {
                gamepads.register(*gamepad);
                info!("{:?} Connected", gamepad);
            }
            GamepadEvent(gamepad, GamepadEventType::Disconnected) => {
                gamepads.deregister(gamepad);
                info!("{:?} Disconnected", gamepad);
            }
            _ => (),
        }
    }
}
