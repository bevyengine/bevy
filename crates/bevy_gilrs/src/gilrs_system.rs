use crate::converter::{convert_axis, convert_button, convert_gamepad_id};
use bevy_app::Events;
use bevy_ecs::world::World;
use bevy_input::{gamepad::GamepadEventRaw, prelude::*};
use gilrs::{EventType, Gilrs};

pub fn gilrs_event_startup_system(world: &mut World) {
    let world = world.cell();
    let gilrs = world.get_non_send::<Gilrs>().unwrap();
    let mut event = world.get_resource_mut::<Events<GamepadEventRaw>>().unwrap();
    for (id, _) in gilrs.gamepads() {
        event.send(GamepadEventRaw(
            convert_gamepad_id(id),
            GamepadEventType::Connected,
        ));
    }
}

pub fn gilrs_event_system(world: &mut World) {
    let world = world.cell();
    let mut gilrs = world.get_non_send_mut::<Gilrs>().unwrap();
    let mut event = world.get_resource_mut::<Events<GamepadEventRaw>>().unwrap();
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
