use crate::converter::{convert_axis, convert_button, convert_gamepad_id};
use bevy_ecs::event::EventWriter;
use bevy_ecs::non_send_resources::MainThreadExecutor;
use bevy_ecs::system::Res;
use bevy_input::gamepad::GamepadInfo;
use bevy_input::{gamepad::GamepadEventRaw, prelude::*};
use gilrs::{ev::filter::axis_dpad_to_button, EventType, Filter, Gilrs};

pub fn gilrs_event_startup_system(
    main_thread: Res<MainThreadExecutor>,
    mut events: EventWriter<GamepadEventRaw>,
) {
    main_thread.run(|non_send_resources| {
        let gilrs = non_send_resources.resource::<Gilrs>();
        for (id, gamepad) in gilrs.gamepads() {
            let info = GamepadInfo {
                name: gamepad.name().into(),
            };

            events.send(GamepadEventRaw::new(
                convert_gamepad_id(id),
                GamepadEventType::Connected(info),
            ));
        }
    });
}

pub fn gilrs_event_system(
    main_thread: Res<MainThreadExecutor>,
    mut events: EventWriter<GamepadEventRaw>,
) {
    main_thread.run(|non_send_resources| {
        let mut gilrs = non_send_resources.resource_mut::<Gilrs>();
        while let Some(gilrs_event) = gilrs
            .next_event()
            .filter_ev(&axis_dpad_to_button, &mut gilrs)
        {
            gilrs.update(&gilrs_event);

            match gilrs_event.event {
                EventType::Connected => {
                    let pad = gilrs.gamepad(gilrs_event.id);
                    let info = GamepadInfo {
                        name: pad.name().into(),
                    };

                    events.send(GamepadEventRaw::new(
                        convert_gamepad_id(gilrs_event.id),
                        GamepadEventType::Connected(info),
                    ));
                }
                EventType::Disconnected => {
                    events.send(GamepadEventRaw::new(
                        convert_gamepad_id(gilrs_event.id),
                        GamepadEventType::Disconnected,
                    ));
                }
                EventType::ButtonChanged(gilrs_button, value, _) => {
                    if let Some(button_type) = convert_button(gilrs_button) {
                        events.send(GamepadEventRaw::new(
                            convert_gamepad_id(gilrs_event.id),
                            GamepadEventType::ButtonChanged(button_type, value),
                        ));
                    }
                }
                EventType::AxisChanged(gilrs_axis, value, _) => {
                    if let Some(axis_type) = convert_axis(gilrs_axis) {
                        events.send(GamepadEventRaw::new(
                            convert_gamepad_id(gilrs_event.id),
                            GamepadEventType::AxisChanged(axis_type, value),
                        ));
                    }
                }
                _ => (),
            };
        }
        gilrs.inc();
    });
}
