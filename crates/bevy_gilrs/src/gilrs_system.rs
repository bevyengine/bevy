use crate::converter::{convert_axis, convert_button, convert_gamepad_id};
use bevy_ecs::event::EventWriter;
use bevy_ecs::system::{NonSend, NonSendMut};
use bevy_input::gamepad::GamepadInfo;
use bevy_input::{gamepad::GamepadEventRaw, prelude::*};
use gilrs::{ev::filter::axis_dpad_to_button, EventType, Filter, Gilrs};

pub fn gilrs_event_startup_system(gilrs: NonSend<Gilrs>, mut events: EventWriter<GamepadEventRaw>) {
    for (id, gamepad) in gilrs.gamepads() {
        let info = GamepadInfo {
            name: gamepad.name().into(),
        };

        events.send(GamepadEventRaw::new(
            convert_gamepad_id(id),
            GamepadEventType::Connected(info),
        ));
    }
}

pub fn gilrs_event_system(mut gilrs: NonSendMut<Gilrs>, mut events: EventWriter<GamepadEventRaw>) {
    while let Some(gilrs_event) = gilrs
        .next_event()
        .filter_ev(&axis_dpad_to_button, &mut gilrs)
    {
        gilrs.update(&gilrs_event);

        let maybe_gamepad_event = match gilrs_event.event {
            EventType::Connected => {
                let pad = gilrs.gamepad(gilrs_event.id);
                let info = GamepadInfo {
                    name: pad.name().into(),
                };

                Some(GamepadEventType::Connected(info))
            }
            EventType::Disconnected => {
                Some(GamepadEventRaw::new(gampad, GamepadEventType::Disconnected));
            }
            EventType::ButtonChanged(gilrs_button, value, _) => {
                if let Some(button_type) = convert_button(gilrs_button) {
                    Some(GamepadEventType::ButtonChanged(button_type, value))
                } else {
                    None
                }
            }
            EventType::AxisChanged(gilrs_axis, value, _) => {
                if let Some(axis_type) = convert_axis(gilrs_axis) {
                    Some(GamepadEventType::AxisChanged(axis_type, value))
                } else {
                    None
                }
            }
            _ => None,
        };

        if let Some(gamepad_event) = maybe_gamepad_event {
            events.send(GamepadEventRaw {
                gamepad: convert_gamepad_id(gilrs_event.id),
                event_type: gamepad_event,
            });
        }
    }
    gilrs.inc();
}
