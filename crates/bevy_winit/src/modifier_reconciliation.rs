//! Reconcile native logical modifier state without assuming a canonical keymap.
use bevy_ecs::entity::Entity;
use bevy_input::{
    keyboard::{Key, KeyCode, KeyboardInput},
    ButtonState,
};
use std::collections::HashMap;
use winit::{event::Modifiers, keyboard::ModifiersKeyState};

type ModifierGroup = (Key, bool, [(KeyCode, ModifiersKeyState); 2]);

/// Reconcile recorded logical modifiers while preserving physical key codes.
pub(crate) fn reconcile_modifiers(
    pressed_keys: &mut HashMap<KeyCode, Key>,
    window: Entity,
    mods: Modifiers,
) -> Vec<KeyboardInput> {
    let state = mods.state();
    let groups = [
        (
            Key::Super,
            state.super_key(),
            [
                (KeyCode::SuperLeft, mods.lsuper_state()),
                (KeyCode::SuperRight, mods.rsuper_state()),
            ],
        ),
        (
            Key::Control,
            state.control_key(),
            [
                (KeyCode::ControlLeft, mods.lcontrol_state()),
                (KeyCode::ControlRight, mods.rcontrol_state()),
            ],
        ),
        (
            Key::Alt,
            state.alt_key(),
            [
                (KeyCode::AltLeft, mods.lalt_state()),
                (KeyCode::AltRight, mods.ralt_state()),
            ],
        ),
        (
            Key::Shift,
            state.shift_key(),
            [
                (KeyCode::ShiftLeft, mods.lshift_state()),
                (KeyCode::ShiftRight, mods.rshift_state()),
            ],
        ),
    ];
    reconcile_groups(pressed_keys, window, &groups)
}

fn reconcile_groups(
    pressed_keys: &mut HashMap<KeyCode, Key>,
    window: Entity,
    groups: &[ModifierGroup],
) -> Vec<KeyboardInput> {
    let mut recovered = Vec::new();
    // Inspect the original map so a released remap isn't recovered as a canonical key.
    for (logical_key, _, sides) in groups {
        let held_by_remap = pressed_keys
            .iter()
            .any(|(code, key)| key == logical_key && !sides.iter().any(|(side, _)| side == code));
        if held_by_remap {
            continue;
        }
        for &(key_code, side_state) in sides {
            if side_state == ModifiersKeyState::Pressed && !pressed_keys.contains_key(&key_code) {
                recovered.push((key_code, logical_key.clone()));
            }
        }
    }

    let mut synthesized: Vec<_> = pressed_keys
        .extract_if(|&key_code, logical_key| {
            let Some((_, group_pressed, sides)) =
                groups.iter().find(|(key, ..)| key == logical_key)
            else {
                return false;
            };
            let any_side_pressed = sides
                .iter()
                .any(|&(_, state)| state == ModifiersKeyState::Pressed);
            // Per-side state only describes the canonical key of this logical group.
            let side_released = sides.iter().any(|&(side, state)| {
                side == key_code && state == ModifiersKeyState::Unknown && any_side_pressed
            });
            !group_pressed || side_released
        })
        .map(|(key_code, logical_key)| KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Released,
            repeat: false,
            window,
            text: None,
        })
        .collect();
    synthesized.sort_unstable_by_key(|event| event.key_code);
    // Release before pressing a different side, so the logical key finishes pressed.
    synthesized.extend(recovered.into_iter().map(|(key_code, logical_key)| {
        pressed_keys.insert(key_code, logical_key.clone());
        KeyboardInput {
            key_code,
            logical_key,
            state: ButtonState::Pressed,
            repeat: false,
            window,
            text: None,
        }
    }));
    synthesized
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::App;
    use bevy_ecs::message::Messages;
    use bevy_input::{ButtonInput, InputPlugin};
    use winit::keyboard::{
        ModifiersKeyState::{Pressed, Unknown},
        ModifiersState,
    };

    fn modifier_groups() -> [(Key, [KeyCode; 2]); 4] {
        [
            (Key::Control, [KeyCode::ControlLeft, KeyCode::ControlRight]),
            (Key::Alt, [KeyCode::AltLeft, KeyCode::AltRight]),
            (Key::Shift, [KeyCode::ShiftLeft, KeyCode::ShiftRight]),
            (Key::Super, [KeyCode::SuperLeft, KeyCode::SuperRight]),
        ]
    }

    fn input(
        window: Entity,
        key_code: KeyCode,
        logical_key: Key,
        state: ButtonState,
    ) -> KeyboardInput {
        KeyboardInput {
            key_code,
            logical_key,
            state,
            window,
            repeat: false,
            text: None,
        }
    }

    #[test]
    fn remapped_modifiers_follow_logical_state() {
        let window = Entity::PLACEHOLDER;
        assert!(
            reconcile_modifiers(&mut HashMap::new(), window, ModifiersState::all().into())
                .is_empty()
        );
        for (code, key, state) in [
            (KeyCode::ControlLeft, Key::Alt, ModifiersState::ALT),
            (KeyCode::ControlRight, Key::Alt, ModifiersState::ALT),
            (KeyCode::AltLeft, Key::Control, ModifiersState::CONTROL),
            (KeyCode::AltRight, Key::Control, ModifiersState::CONTROL),
            (KeyCode::ShiftLeft, Key::Super, ModifiersState::SUPER),
            (KeyCode::ShiftRight, Key::Super, ModifiersState::SUPER),
            (KeyCode::SuperLeft, Key::Shift, ModifiersState::SHIFT),
            (KeyCode::SuperRight, Key::Shift, ModifiersState::SHIFT),
            (KeyCode::CapsLock, Key::Control, ModifiersState::CONTROL),
        ] {
            let mut recorded = HashMap::from([(code, key.clone())]);
            assert!(reconcile_modifiers(&mut recorded, window, state.into()).is_empty());
            assert_eq!(recorded.get(&code), Some(&key));
            assert_eq!(
                reconcile_modifiers(&mut recorded, window, ModifiersState::empty().into()),
                vec![input(window, code, key, ButtonState::Released)]
            );
            assert!(recorded.is_empty());
            assert!(
                reconcile_modifiers(&mut recorded, window, ModifiersState::empty().into())
                    .is_empty()
            );
        }
        let non_modifiers = HashMap::from([
            (KeyCode::AltRight, Key::AltGraph),
            (KeyCode::ControlLeft, Key::Character("x".into())),
            (KeyCode::KeyA, Key::Character("a".into())),
        ]);
        let mut recorded = non_modifiers.clone();
        for state in [ModifiersState::all(), ModifiersState::empty()] {
            assert!(reconcile_modifiers(&mut recorded, window, state.into()).is_empty());
            assert_eq!(recorded, non_modifiers);
        }
    }

    #[test]
    fn releases_have_a_stable_physical_key_order() {
        let window = Entity::PLACEHOLDER;
        let keys = [
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::CapsLock,
            KeyCode::KeyA,
        ];
        let mut expected =
            keys.map(|code| input(window, code, Key::Control, ButtonState::Released));
        expected.sort_unstable_by_key(|event| event.key_code);
        let mut reversed = keys;
        reversed.reverse();
        for codes in [keys, reversed] {
            let mut recorded = HashMap::from(codes.map(|code| (code, Key::Control)));
            assert!(
                reconcile_modifiers(&mut recorded, window, ModifiersState::CONTROL.into())
                    .is_empty()
            );
            assert_eq!(
                reconcile_modifiers(&mut recorded, window, ModifiersState::empty().into()),
                expected
            );
            assert!(recorded.is_empty());
        }
    }

    #[test]
    fn attested_sides_respect_remapped_and_occupied_keys() {
        let window = Entity::PLACEHOLDER;
        for (logical, codes) in modifier_groups() {
            for (side, &code) in codes.iter().enumerate() {
                let other = codes[1 - side];
                let mut sides = codes.map(|code| (code, Unknown));
                sides[side].1 = Pressed;
                let groups = [(logical.clone(), true, sides)];
                let mut recorded = HashMap::from([
                    (KeyCode::CapsLock, logical.clone()),
                    (other, logical.clone()),
                ]);
                assert_eq!(
                    reconcile_groups(&mut recorded, window, &groups),
                    vec![input(window, other, logical.clone(), ButtonState::Released)]
                );
                assert_eq!(
                    recorded,
                    HashMap::from([(KeyCode::CapsLock, logical.clone())])
                );
                assert!(reconcile_groups(&mut recorded, window, &groups).is_empty());

                let occupied = HashMap::from([(code, Key::Character("x".into()))]);
                let mut recorded = occupied.clone();
                assert!(reconcile_groups(&mut recorded, window, &groups).is_empty());
                assert_eq!(recorded, occupied);

                let (other_logical, other_codes) = modifier_groups()
                    .into_iter()
                    .find(|(key, _)| key != &logical)
                    .unwrap();
                let mut recorded = HashMap::from([(code, other_logical.clone())]);
                let groups = [
                    groups[0].clone(),
                    (
                        other_logical.clone(),
                        false,
                        other_codes.map(|code| (code, Unknown)),
                    ),
                ];
                assert_eq!(
                    reconcile_groups(&mut recorded, window, &groups),
                    vec![input(window, code, other_logical, ButtonState::Released)]
                );
                assert!(recorded.is_empty());
            }
            let groups = [(logical.clone(), true, codes.map(|code| (code, Pressed)))];
            assert_eq!(
                reconcile_groups(&mut HashMap::new(), window, &groups),
                codes.map(|code| input(window, code, logical.clone(), ButtonState::Pressed))
            );
        }
    }

    #[test]
    fn attested_recovery_and_side_switch_keep_logical_input_pressed() {
        for (logical, codes) in modifier_groups() {
            for (side, &code) in codes.iter().enumerate() {
                for previous in [None, Some(codes[1 - side])] {
                    let mut app = App::new();
                    app.add_plugins(InputPlugin);
                    let window = Entity::PLACEHOLDER;
                    let mut recorded = HashMap::new();
                    let mut expected = Vec::new();
                    if let Some(other) = previous {
                        recorded.insert(other, logical.clone());
                        app.world_mut().write_message(input(
                            window,
                            other,
                            logical.clone(),
                            ButtonState::Pressed,
                        ));
                        expected.push(input(window, other, logical.clone(), ButtonState::Released));
                    }
                    app.update();
                    let mut sides = codes.map(|code| (code, Unknown));
                    sides[side].1 = Pressed;
                    let groups = [(logical.clone(), true, sides)];
                    expected.push(input(window, code, logical.clone(), ButtonState::Pressed));
                    let events = reconcile_groups(&mut recorded, window, &groups);
                    assert_eq!(recorded, HashMap::from([(code, logical.clone())]));
                    app.world_mut()
                        .resource_mut::<Messages<KeyboardInput>>()
                        .write_batch(events.iter().cloned());
                    app.update();
                    let physical = app.world().resource::<ButtonInput<KeyCode>>();
                    let logical_input = app.world().resource::<ButtonInput<Key>>();
                    assert!(physical.pressed(code));
                    assert!(logical_input.pressed(logical.clone()));
                    assert!(physical.just_pressed(code));
                    if let Some(other) = previous {
                        assert!(!physical.pressed(other));
                    }
                    assert_eq!(events, expected);
                    assert!(reconcile_groups(&mut recorded, window, &groups).is_empty());
                    app.update();
                    let physical = app.world().resource::<ButtonInput<KeyCode>>();
                    let logical_input = app.world().resource::<ButtonInput<Key>>();
                    assert!(!physical.just_pressed(code));
                    assert!(logical_input.pressed(logical.clone()));
                    let events = reconcile_groups(
                        &mut recorded,
                        window,
                        &[(logical.clone(), false, codes.map(|code| (code, Unknown)))],
                    );
                    assert_eq!(
                        events,
                        vec![input(window, code, logical.clone(), ButtonState::Released)]
                    );
                    app.world_mut()
                        .resource_mut::<Messages<KeyboardInput>>()
                        .write_batch(events);
                    app.update();
                    assert!(recorded.is_empty());
                    let physical = app.world().resource::<ButtonInput<KeyCode>>();
                    let logical_input = app.world().resource::<ButtonInput<Key>>();
                    assert!(physical.just_released(code));
                    assert!(logical_input.just_released(logical.clone()));
                }
            }
        }
    }
}
