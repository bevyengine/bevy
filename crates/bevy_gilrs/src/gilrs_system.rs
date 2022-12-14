use crate::converter::{convert_axis, convert_button, convert_gamepad_id};
use bevy_ecs::event::EventWriter;
use bevy_ecs::system::{NonSend, NonSendMut};
use bevy_input::gamepad::GamepadInfo;
use bevy_input::{
    gamepad::{
        GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
        RawGamepadButtonChangedEvent,
    },
    prelude::*,
};
use gilrs::{ev::filter::axis_dpad_to_button, EventType, Filter, Gilrs};

pub fn gilrs_event_startup_system(
    gilrs: NonSend<Gilrs>,
    mut connection_events: EventWriter<GamepadConnectionEvent>,
) {
    for (id, gamepad) in gilrs.gamepads() {
        let info = GamepadInfo {
            name: gamepad.name().into(),
        };

        connection_events.send(GamepadConnectionEvent {
            gamepad: convert_gamepad_id(id),
            connection: GamepadConnection::Connected(info),
        });
    }
}

pub fn gilrs_event_system(
    mut gilrs: NonSendMut<Gilrs>,
    mut axis_events: EventWriter<RawGamepadAxisChangedEvent>,
    mut button_events: EventWriter<RawGamepadButtonChangedEvent>,
    mut connection_events: EventWriter<GamepadConnectionEvent>,
) {
    while let Some(gilrs_event) = gilrs
        .next_event()
        .filter_ev(&axis_dpad_to_button, &mut gilrs)
    {
        gilrs.update(&gilrs_event);

        let gamepad = convert_gamepad_id(gilrs_event.id);
        match gilrs_event.event {
            EventType::Connected => {
                let pad = gilrs.gamepad(gilrs_event.id);
                let info = GamepadInfo {
                    name: pad.name().into(),
                };

                connection_events.send(GamepadConnectionEvent {
                    gamepad,
                    connection: GamepadConnection::Connected(info),
                });
            }
            EventType::Disconnected => connection_events.send(GamepadConnectionEvent {
                gamepad,
                connection: GamepadConnection::Disconnected,
            }),
            EventType::ButtonChanged(gilrs_button, value, _) => {
                if let Some(button_type) = convert_button(gilrs_button) {
                    button_events.send(RawGamepadButtonChangedEvent {
                        gamepad,
                        button: GamepadButton::new(gamepad, button_type),
                        unfiltered_value: value,
                    });
                }
            }
            EventType::AxisChanged(gilrs_axis, value, _) => {
                if let Some(axis_type) = convert_axis(gilrs_axis) {
                    axis_events.send(RawGamepadAxisChangedEvent {
                        gamepad,
                        axis: GamepadAxis::new(gamepad, axis_type),
                        unfiltered_value: value,
                    });
                }
            }
            _ => (),
        };
    }
    gilrs.inc();
}
