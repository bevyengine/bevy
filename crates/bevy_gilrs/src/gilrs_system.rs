use crate::converter::{convert_axis, convert_button, convert_gamepad_id};
use bevy_ecs::event::EventWriter;
use bevy_ecs::system::{NonSend, NonSendMut, Res};
use bevy_input::gamepad::{
    GamepadAxisChangedEvent, GamepadButtonChangedEvent, GamepadConnection, GamepadConnectionEvent,
    GamepadSettings,
};
use bevy_input::gamepad::{GamepadEvent, GamepadInfo};
use bevy_input::prelude::{GamepadAxis, GamepadButton};
use bevy_input::Axis;
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
    mut events: EventWriter<GamepadEvent>,
    gamepad_axis: Res<Axis<GamepadAxis>>,
    gamepad_buttons: Res<Axis<GamepadButton>>,
    gamepad_settings: Res<GamepadSettings>,
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

                events.send(
                    GamepadConnectionEvent::new(gamepad, GamepadConnection::Connected(info)).into(),
                );
            }
            EventType::Disconnected => events
                .send(GamepadConnectionEvent::new(gamepad, GamepadConnection::Disconnected).into()),
            EventType::ButtonChanged(gilrs_button, raw_value, _) => {
                if let Some(button_type) = convert_button(gilrs_button) {
                    let button = GamepadButton::new(gamepad, button_type);
                    let old_value = gamepad_buttons.get(button);
                    let button_settings = gamepad_settings.get_button_axis_settings(button);

                    // Only send events that pass the user-defined change threshold
                    if let Some(filtered_value) = button_settings.filter(raw_value, old_value) {
                        events.send(
                            GamepadButtonChangedEvent::new(gamepad, button_type, filtered_value)
                                .into(),
                        );
                    }
                }
            }
            EventType::AxisChanged(gilrs_axis, raw_value, _) => {
                if let Some(axis_type) = convert_axis(gilrs_axis) {
                    let axis = GamepadAxis::new(gamepad, axis_type);
                    let old_value = gamepad_axis.get(axis);
                    let axis_settings = gamepad_settings.get_axis_settings(axis);

                    // Only send events that pass the user-defined change threshold
                    if let Some(filtered_value) = axis_settings.filter(raw_value, old_value) {
                        events.send(
                            GamepadAxisChangedEvent::new(gamepad, axis_type, filtered_value).into(),
                        );
                    }
                }
            }
            _ => (),
        };
    }
    gilrs.inc();
}
