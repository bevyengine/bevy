use crate::{
    gamepad::{
        GamepadAxis, GamepadButton, GamepadEvent, GamepadEventRaw, GamepadEventType,
        GamepadSettings, Gamepads, ALL_AXIS_TYPES, ALL_BUTTON_TYPES,
    },
    Axis, Input,
};
use bevy_app::{EventReader, EventWriter};
use bevy_ecs::system::{Res, ResMut};
use bevy_utils::tracing::info;

/// Monitors gamepad connection and disconnection events and updates the [`Gamepads`] resource accordingly.
///
/// ## Note
///
/// Whenever a [`Gamepad`](crate::gamepad::Gamepad) connects or disconnects, an information gets printed to the console using the [`info!`] macro.
pub fn gamepad_connection_system(
    mut gamepads: ResMut<Gamepads>,
    mut gamepad_event: EventReader<GamepadEvent>,
) {
    for event in gamepad_event.iter() {
        match event.event_type {
            GamepadEventType::Connected => {
                gamepads.register(event.gamepad);
                info!("{:?} Connected", event.gamepad);
            }
            GamepadEventType::Disconnected => {
                gamepads.deregister(&event.gamepad);
                info!("{:?} Disconnected", event.gamepad);
            }
            _ => (),
        }
    }
}

/// Modifies the gamepad resources and sends out gamepad events.
///
/// The resources [`Input<GamepadButton>`], [`Axis<GamepadAxis>`], and [`Axis<GamepadButton>`] are updated
/// and the [`GamepadEvent`]s are sent according to the received [`GamepadEventRaw`]s.
///
/// ## Differences
///
/// The main difference between the events and the resources is that the latter allows you to check specifc
/// buttons or axes, rather than reading the events one at a time. This is done through convenient functions
/// like [`Input::pressed`], [`Input::just_pressed`], and [`Input::just_released`] or [`Axis::get`], and
/// [`Axis::set`].
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
        match event.event_type {
            GamepadEventType::Connected => {
                events.send(GamepadEvent::new(event.gamepad, event.event_type.clone()));
                for button_type in &ALL_BUTTON_TYPES {
                    let gamepad_button = GamepadButton::new(event.gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.set(gamepad_button, 0.0);
                }
                for axis_type in &ALL_AXIS_TYPES {
                    axis.set(GamepadAxis::new(event.gamepad, *axis_type), 0.0);
                }
            }
            GamepadEventType::Disconnected => {
                events.send(GamepadEvent::new(event.gamepad, event.event_type.clone()));
                for button_type in &ALL_BUTTON_TYPES {
                    let gamepad_button = GamepadButton::new(event.gamepad, *button_type);
                    button_input.reset(gamepad_button);
                    button_axis.remove(gamepad_button);
                }
                for axis_type in &ALL_AXIS_TYPES {
                    axis.remove(GamepadAxis::new(event.gamepad, *axis_type));
                }
            }
            GamepadEventType::AxisChanged(axis_type, value) => {
                let gamepad_axis = GamepadAxis::new(event.gamepad, axis_type);
                if let Some(filtered_value) = settings
                    .get_axis_settings(gamepad_axis)
                    .filter(value, axis.get(gamepad_axis))
                {
                    axis.set(gamepad_axis, filtered_value);
                    events.send(GamepadEvent::new(
                        event.gamepad,
                        GamepadEventType::AxisChanged(axis_type, filtered_value),
                    ));
                }
            }
            GamepadEventType::ButtonChanged(button_type, value) => {
                let gamepad_button = GamepadButton::new(event.gamepad, button_type);
                if let Some(filtered_value) = settings
                    .get_button_axis_settings(gamepad_button)
                    .filter(value, button_axis.get(gamepad_button))
                {
                    button_axis.set(gamepad_button, filtered_value);
                    events.send(GamepadEvent::new(
                        event.gamepad,
                        GamepadEventType::ButtonChanged(button_type, filtered_value),
                    ));
                }

                let button_property = settings.get_button_settings(gamepad_button);
                if button_input.pressed(gamepad_button) {
                    if button_property.is_released(value) {
                        button_input.release(gamepad_button);
                    }
                } else if button_property.is_pressed(value) {
                    button_input.press(gamepad_button);
                }
            }
        }
    }
}
