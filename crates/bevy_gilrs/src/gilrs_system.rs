use crate::converter::{convert_axis, convert_button, convert_gamepad_id};
use bevy_app::Events;
use bevy_ecs::{Resources, World};
use bevy_input::{gamepad::GamepadEventRaw, prelude::*};
use gilrs::{EventType, Gilrs};

pub fn gilrs_event_startup_system(_world: &mut World, resources: &mut Resources) {
    let gilrs = resources.get_thread_local::<Gilrs>().unwrap();
    let mut event = resources.get_mut::<Events<GamepadEventRaw>>().unwrap();
    for (id, _) in gilrs.gamepads() {
        event.send(GamepadEventRaw(
            convert_gamepad_id(id),
            GamepadEventType::Connected,
        ));
    }
}

pub fn gilrs_event_system(_world: &mut World, resources: &mut Resources) {
    let mut gilrs = resources.get_thread_local_mut::<Gilrs>().unwrap();
    let mut event = resources.get_mut::<Events<GamepadEventRaw>>().unwrap();
    event.update();
    while let Some(gilrs_event) = gilrs.next_event() {
        match gilrs_event.event {
            EventType::Connected => {
                event.send(GamepadEventRaw(
                    convert_gamepad_id(gilrs_event.id),
                    GamepadEventType::Connected,
                ));
            }
            EventType::Disconnected => {
                event.send(GamepadEventRaw(
                    convert_gamepad_id(gilrs_event.id),
                    GamepadEventType::Disconnected,
                ));
            }
            EventType::ButtonChanged(gilrs_button, value, _) => {
                if let Some(button_type) = convert_button(gilrs_button) {
                    event.send(GamepadEventRaw(
                        convert_gamepad_id(gilrs_event.id),
                        GamepadEventType::ButtonChanged(button_type, value),
                    ));
                }
            }
            EventType::AxisChanged(gilrs_axis, value, _) => {
                if let Some(axis_type) = convert_axis(gilrs_axis) {
                    event.send(GamepadEventRaw(
                        convert_gamepad_id(gilrs_event.id),
                        GamepadEventType::AxisChanged(axis_type, value),
                    ));
                }
            }
            _ => (),
        };
    }
    gilrs.inc();
}
