use crate::converter::{convert_axis, convert_button, convert_gamepad_id};
use bevy_app::Events;
use bevy_ecs::{Resources, World};
use bevy_input::prelude::*;
use gilrs::{Button, EventType, Gilrs};

pub fn gilrs_startup_system(_world: &mut World, resources: &mut Resources) {
    let gilrs = resources.get_thread_local::<Gilrs>().unwrap();
    let mut gamepad_event = resources.get_mut::<Events<GamepadEvent>>().unwrap();
    let mut inputs = resources.get_mut::<Input<GamepadButton>>().unwrap();
    let mut axes = resources.get_mut::<Axis<GamepadAxis>>().unwrap();
    let mut button_axes = resources.get_mut::<Axis<GamepadButton>>().unwrap();
    gamepad_event.update();
    inputs.update();
    for (gilrs_id, gilrs_gamepad) in gilrs.gamepads() {
        connect_gamepad(
            gilrs_gamepad,
            convert_gamepad_id(gilrs_id),
            &mut gamepad_event,
            &mut inputs,
            &mut axes,
            &mut button_axes,
        );
    }
}

pub fn gilrs_update_system(_world: &mut World, resources: &mut Resources) {
    let mut gilrs = resources.get_thread_local_mut::<Gilrs>().unwrap();
    let mut gamepad_event = resources.get_mut::<Events<GamepadEvent>>().unwrap();
    let mut inputs = resources.get_mut::<Input<GamepadButton>>().unwrap();
    let mut axes = resources.get_mut::<Axis<GamepadAxis>>().unwrap();
    let mut button_axes = resources.get_mut::<Axis<GamepadButton>>().unwrap();

    gamepad_event.update();
    inputs.update();
    while let Some(gilrs_event) = gilrs.next_event() {
        match gilrs_event.event {
            EventType::Connected => {
                connect_gamepad(
                    gilrs.gamepad(gilrs_event.id),
                    convert_gamepad_id(gilrs_event.id),
                    &mut gamepad_event,
                    &mut inputs,
                    &mut axes,
                    &mut button_axes,
                );
            }
            EventType::Disconnected => {
                disconnect_gamepad(
                    convert_gamepad_id(gilrs_event.id),
                    &mut gamepad_event,
                    &mut inputs,
                    &mut axes,
                    &mut button_axes,
                );
            }
            EventType::ButtonPressed(gilrs_button, _) => {
                if let Some(button_type) = convert_button(gilrs_button) {
                    inputs.press(GamepadButton(
                        convert_gamepad_id(gilrs_event.id),
                        button_type,
                    ));
                }
            }
            EventType::ButtonReleased(gilrs_button, _) => {
                if let Some(button_type) = convert_button(gilrs_button) {
                    inputs.release(GamepadButton(
                        convert_gamepad_id(gilrs_event.id),
                        button_type,
                    ));
                }
            }
            EventType::ButtonChanged(gilrs_button, value, _) => {
                if let Some(button_type) = convert_button(gilrs_button) {
                    button_axes.set(
                        GamepadButton(convert_gamepad_id(gilrs_event.id), button_type),
                        value,
                    );
                }
            }
            EventType::AxisChanged(gilrs_axis, value, _) => {
                if let Some(axis_type) = convert_axis(gilrs_axis) {
                    axes.set(
                        GamepadAxis(convert_gamepad_id(gilrs_event.id), axis_type),
                        value,
                    );
                }
            }
            _ => (),
        };
    }
    gilrs.inc();
}

const ALL_GILRS_BUTTONS: [Button; 19] = [
    Button::South,
    Button::East,
    Button::North,
    Button::West,
    Button::C,
    Button::Z,
    Button::LeftTrigger,
    Button::LeftTrigger2,
    Button::RightTrigger,
    Button::RightTrigger2,
    Button::Select,
    Button::Start,
    Button::Mode,
    Button::LeftThumb,
    Button::RightThumb,
    Button::DPadUp,
    Button::DPadDown,
    Button::DPadLeft,
    Button::DPadRight,
];

const ALL_GILRS_AXES: [gilrs::Axis; 8] = [
    gilrs::Axis::LeftStickX,
    gilrs::Axis::LeftStickY,
    gilrs::Axis::LeftZ,
    gilrs::Axis::RightStickX,
    gilrs::Axis::RightStickY,
    gilrs::Axis::RightZ,
    gilrs::Axis::DPadX,
    gilrs::Axis::DPadY,
];

fn connect_gamepad(
    gilrs_gamepad: gilrs::Gamepad,
    gamepad: Gamepad,
    events: &mut Events<GamepadEvent>,
    inputs: &mut Input<GamepadButton>,
    axes: &mut Axis<GamepadAxis>,
    button_axes: &mut Axis<GamepadButton>,
) {
    for gilrs_button in ALL_GILRS_BUTTONS.iter() {
        if let Some(button_type) = convert_button(*gilrs_button) {
            if let Some(button_data) = gilrs_gamepad.button_data(*gilrs_button) {
                let gamepad_button = GamepadButton(gamepad, button_type);
                inputs.reset(gamepad_button);
                if button_data.is_pressed() {
                    inputs.press(gamepad_button);
                }
                button_axes.set(gamepad_button, button_data.value());
            }
        }
    }
    for gilrs_axis in ALL_GILRS_AXES.iter() {
        if let Some(axis_type) = convert_axis(*gilrs_axis) {
            let gamepad_axis = GamepadAxis(gamepad, axis_type);
            axes.set(gamepad_axis, gilrs_gamepad.value(*gilrs_axis));
        }
    }
    events.send(GamepadEvent(gamepad, GamepadEventType::Connected));
}

fn disconnect_gamepad(
    gamepad: Gamepad,
    events: &mut Events<GamepadEvent>,
    inputs: &mut Input<GamepadButton>,
    axes: &mut Axis<GamepadAxis>,
    button_axes: &mut Axis<GamepadButton>,
) {
    for gilrs_button in ALL_GILRS_BUTTONS.iter() {
        if let Some(button_type) = convert_button(*gilrs_button) {
            let gamepad_button = GamepadButton(gamepad, button_type);
            inputs.reset(gamepad_button);
            button_axes.remove(&gamepad_button);
        }
    }
    for gilrs_axis in ALL_GILRS_AXES.iter() {
        if let Some(axis_type) = convert_axis(*gilrs_axis) {
            let gamepad_axis = GamepadAxis(gamepad, axis_type);
            axes.remove(&gamepad_axis);
        }
    }
    events.send(GamepadEvent(gamepad, GamepadEventType::Disconnected));
}
