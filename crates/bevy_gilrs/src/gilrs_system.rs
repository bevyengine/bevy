use crate::{
    converter::{convert_axis, convert_button},
    Gilrs, GilrsGamepads,
};
use bevy_ecs::message::MessageWriter;
use bevy_ecs::prelude::Commands;
use bevy_ecs::system::ResMut;
use bevy_input::gamepad::{
    GamepadConnection, GamepadConnectionEvent, RawGamepadAxisChangedEvent,
    RawGamepadButtonChangedEvent, RawGamepadEvent,
};
use gilrs::{ev::filter::axis_dpad_to_button, EventType, Filter};

pub fn gilrs_event_startup_system(
    mut commands: Commands,
    mut gilrs: ResMut<Gilrs>,
    mut gamepads: ResMut<GilrsGamepads>,
    mut events: MessageWriter<GamepadConnectionEvent>,
) {
    gilrs.with(|gilrs| {
        for (id, gamepad) in gilrs.gamepads() {
            // Create entity and add to mapping
            let entity = commands.spawn_empty().id();
            gamepads.id_to_entity.insert(id, entity);
            gamepads.entity_to_id.insert(entity, id);
            events.write(GamepadConnectionEvent {
                gamepad: entity,
                connection: GamepadConnection::Connected {
                    name: gamepad.name().to_string(),
                    vendor_id: gamepad.vendor_id(),
                    product_id: gamepad.product_id(),
                },
            });
        }
    });
}

pub fn gilrs_event_system(
    mut commands: Commands,
    mut gilrs: ResMut<Gilrs>,
    mut gamepads: ResMut<GilrsGamepads>,
    mut events: MessageWriter<RawGamepadEvent>,
    mut connection_events: MessageWriter<GamepadConnectionEvent>,
    mut button_events: MessageWriter<RawGamepadButtonChangedEvent>,
    mut axis_event: MessageWriter<RawGamepadAxisChangedEvent>,
) {
    gilrs.with(|gilrs| {
        while let Some(gilrs_event) = gilrs.next_event().filter_ev(&axis_dpad_to_button, gilrs) {
            gilrs.update(&gilrs_event);
            match gilrs_event.event {
                EventType::Connected => {
                    let pad = gilrs.gamepad(gilrs_event.id);
                    let entity = gamepads.get_entity(gilrs_event.id).unwrap_or_else(|| {
                        let entity = commands.spawn_empty().id();
                        gamepads.id_to_entity.insert(gilrs_event.id, entity);
                        gamepads.entity_to_id.insert(entity, gilrs_event.id);
                        entity
                    });

                    let event = GamepadConnectionEvent::new(
                        entity,
                        GamepadConnection::Connected {
                            name: pad.name().to_string(),
                            vendor_id: pad.vendor_id(),
                            product_id: pad.product_id(),
                        },
                    );
                    events.write(event.clone().into());
                    connection_events.write(event);
                }
                EventType::Disconnected => {
                    let gamepad = gamepads
                        .id_to_entity
                        .get(&gilrs_event.id)
                        .copied()
                        .expect("mapping should exist from connection");
                    let event =
                        GamepadConnectionEvent::new(gamepad, GamepadConnection::Disconnected);
                    events.write(event.clone().into());
                    connection_events.write(event);
                }
                EventType::ButtonChanged(gilrs_button, raw_value, _) => {
                    let Some(button) = convert_button(gilrs_button) else {
                        continue;
                    };
                    let gamepad = gamepads
                        .id_to_entity
                        .get(&gilrs_event.id)
                        .copied()
                        .expect("mapping should exist from connection");
                    events.write(
                        RawGamepadButtonChangedEvent::new(gamepad, button, raw_value).into(),
                    );
                    button_events.write(RawGamepadButtonChangedEvent::new(
                        gamepad, button, raw_value,
                    ));
                }
                EventType::AxisChanged(gilrs_axis, raw_value, _) => {
                    let Some(axis) = convert_axis(gilrs_axis) else {
                        continue;
                    };
                    let gamepad = gamepads
                        .id_to_entity
                        .get(&gilrs_event.id)
                        .copied()
                        .expect("mapping should exist from connection");
                    events.write(RawGamepadAxisChangedEvent::new(gamepad, axis, raw_value).into());
                    axis_event.write(RawGamepadAxisChangedEvent::new(gamepad, axis, raw_value));
                }
                _ => (),
            };
        }
        gilrs.inc();
    });
}
