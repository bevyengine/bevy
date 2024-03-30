use crate::{
    converter::{convert_axis, convert_button, convert_gamepad_id},
    Gilrs,
};
use bevy_ecs::event::EventWriter;
#[cfg(target_arch = "wasm32")]
use bevy_ecs::system::NonSendMut;
use bevy_ecs::system::ResMut;
use bevy_input::gamepad::{
    GamepadConnectionEvent, RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent,
    RawGamepadEvent, GamepadConnection, GamepadInfo
};
use gilrs::{ev::filter::axis_dpad_to_button, EventType, Filter};

pub fn gilrs_event_startup_system(
    #[cfg(target_arch = "wasm32")] mut gilrs: NonSendMut<Gilrs>,
    #[cfg(not(target_arch = "wasm32"))] mut gilrs: ResMut<Gilrs>,
    mut events: EventWriter<RawGamepadEvent>,
) {
    for (id, gamepad) in gilrs.0.get().gamepads() {
        let info = GamepadInfo {
            name: gamepad.name().into(),
        };

        events.send(
            GamepadConnectionEvent {
                gamepad: convert_gamepad_id(id),
                connection: GamepadConnection::Connected(info),
            }
            .into(),
        );
    }
}

pub fn gilrs_event_system(
    #[cfg(target_arch = "wasm32")] mut gilrs: NonSendMut<Gilrs>,
    #[cfg(not(target_arch = "wasm32"))] mut gilrs: ResMut<Gilrs>,
    mut events: EventWriter<RawGamepadEvent>,
) {
    let gilrs = gilrs.0.get();
    while let Some(gilrs_event) = gilrs.next_event().filter_ev(&axis_dpad_to_button, gilrs) {
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
            EventType::Disconnected => {
                events.send(
                    GamepadConnectionEvent::new(gamepad, GamepadConnection::Disconnected).into(),
                );
            }
            EventType::ButtonChanged(gilrs_button, raw_value, _) => {
                let Some(button) = convert_button(gilrs_button) else {
                    continue;
                };
                events.send(RawGamepadButtonChangedEvent::new(gamepad, button, raw_value).into());
            }
            EventType::AxisChanged(gilrs_axis, raw_value, _) => {
                let Some(axis) = convert_axis(gilrs_axis) else {
                    continue;
                };
                events.send(RawGamepadAxisChangedEvent::new(gamepad, axis, raw_value).into());
            }
            _ => (),
        };
    }
    gilrs.inc();
}
