//! Reconcile native logical modifier state without assuming a canonical keymap.
use alloc::collections::BTreeMap;
use bevy_ecs::entity::Entity;
use bevy_input::{
    keyboard::{Key, KeyCode, KeyboardInput},
    ButtonState,
};
use winit::{event::Modifiers, keyboard::ModifiersKeyState};

struct ModifierGroup {
    logical_key: Key,
    group_pressed: bool,
    sides: [(KeyCode, ModifiersKeyState); 2],
}

/// Reconcile recorded logical modifiers while preserving physical key codes.
pub(crate) fn reconcile_modifiers(
    pressed_keys: &mut BTreeMap<KeyCode, Key>,
    window: Entity,
    mods: Modifiers,
) -> Vec<KeyboardInput> {
    let state = mods.state();
    let groups = [
        ModifierGroup {
            logical_key: Key::Super,
            group_pressed: state.super_key(),
            sides: [
                (KeyCode::SuperLeft, mods.lsuper_state()),
                (KeyCode::SuperRight, mods.rsuper_state()),
            ],
        },
        ModifierGroup {
            logical_key: Key::Control,
            group_pressed: state.control_key(),
            sides: [
                (KeyCode::ControlLeft, mods.lcontrol_state()),
                (KeyCode::ControlRight, mods.rcontrol_state()),
            ],
        },
        ModifierGroup {
            logical_key: Key::Alt,
            group_pressed: state.alt_key(),
            sides: [
                (KeyCode::AltLeft, mods.lalt_state()),
                (KeyCode::AltRight, mods.ralt_state()),
            ],
        },
        ModifierGroup {
            logical_key: Key::Shift,
            group_pressed: state.shift_key(),
            sides: [
                (KeyCode::ShiftLeft, mods.lshift_state()),
                (KeyCode::ShiftRight, mods.rshift_state()),
            ],
        },
    ];
    reconcile_groups(pressed_keys, window, &groups)
}

fn reconcile_groups(
    pressed_keys: &mut BTreeMap<KeyCode, Key>,
    window: Entity,
    groups: &[ModifierGroup],
) -> Vec<KeyboardInput> {
    let mut recovered = Vec::new();
    // Inspect the original map so a released remap isn't recovered as a canonical key.
    for group in groups {
        let held_by_remap = pressed_keys.iter().any(|(code, key)| {
            key == &group.logical_key && !group.sides.iter().any(|(side, _)| side == code)
        });
        if held_by_remap {
            continue;
        }
        for &(key_code, side_state) in &group.sides {
            if side_state == ModifiersKeyState::Pressed && !pressed_keys.contains_key(&key_code) {
                recovered.push((key_code, group.logical_key.clone()));
            }
        }
    }

    let mut synthesized: Vec<_> = pressed_keys
        .extract_if(.., |&key_code, logical_key| {
            let Some(group) = groups
                .iter()
                .find(|group| group.logical_key == *logical_key)
            else {
                return false;
            };
            let any_side_pressed = group
                .sides
                .iter()
                .any(|&(_, state)| state == ModifiersKeyState::Pressed);
            // Per-side state only describes the canonical key of this logical group.
            let side_released = group.sides.iter().any(|&(side, state)| {
                side == key_code && state == ModifiersKeyState::Unknown && any_side_pressed
            });
            !group.group_pressed || side_released
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
    use bevy_input::{ButtonInput, InputPlugin};
    use winit::keyboard::{
        ModifiersKeyState::{Pressed, Unknown},
        ModifiersState,
    };

    const WINDOW: Entity = Entity::PLACEHOLDER;

    fn held_control(sides: [ModifiersKeyState; 2]) -> ModifierGroup {
        ModifierGroup {
            logical_key: Key::Control,
            group_pressed: true,
            sides: [
                (KeyCode::ControlLeft, sides[0]),
                (KeyCode::ControlRight, sides[1]),
            ],
        }
    }

    fn input(key_code: KeyCode, logical_key: Key, state: ButtonState) -> KeyboardInput {
        KeyboardInput {
            key_code,
            logical_key,
            state,
            window: WINDOW,
            repeat: false,
            text: None,
        }
    }

    #[test]
    fn remapped_modifier_stays_pressed_while_its_logical_group_is_held() {
        let mut recorded = BTreeMap::from([(KeyCode::ControlLeft, Key::Alt)]);
        let events = reconcile_modifiers(&mut recorded, WINDOW, ModifiersState::ALT.into());
        assert!(events.is_empty());
        assert_eq!(recorded, BTreeMap::from([(KeyCode::ControlLeft, Key::Alt)]));
    }

    #[test]
    fn logical_group_release_preserves_both_physical_key_codes() {
        let mut recorded = BTreeMap::from([
            (KeyCode::ControlRight, Key::Alt),
            (KeyCode::ControlLeft, Key::Alt),
        ]);
        let events = reconcile_modifiers(&mut recorded, WINDOW, ModifiersState::empty().into());
        assert_eq!(
            events,
            vec![
                input(KeyCode::ControlLeft, Key::Alt, ButtonState::Released),
                input(KeyCode::ControlRight, Key::Alt, ButtonState::Released),
            ]
        );
        assert!(recorded.is_empty());
    }

    #[test]
    fn non_modifier_keys_are_not_reconciled() {
        let keys = BTreeMap::from([
            (KeyCode::AltRight, Key::AltGraph),
            (KeyCode::ControlLeft, Key::Character("x".into())),
            (KeyCode::KeyA, Key::Character("a".into())),
        ]);
        let mut recorded = keys.clone();
        assert!(
            reconcile_modifiers(&mut recorded, WINDOW, ModifiersState::empty().into()).is_empty()
        );
        assert_eq!(recorded, keys);
    }

    #[test]
    fn unknown_sides_do_not_invent_a_press() {
        let mut recorded = BTreeMap::new();
        assert!(
            reconcile_modifiers(&mut recorded, WINDOW, ModifiersState::CONTROL.into()).is_empty()
        );
        assert!(recorded.is_empty());
    }

    #[test]
    fn attested_press_is_recovered_only_once() {
        let mut recorded = BTreeMap::new();
        let groups = [held_control([Unknown, Pressed])];
        assert_eq!(
            reconcile_groups(&mut recorded, WINDOW, &groups),
            vec![input(
                KeyCode::ControlRight,
                Key::Control,
                ButtonState::Pressed
            )]
        );
        assert_eq!(
            recorded,
            BTreeMap::from([(KeyCode::ControlRight, Key::Control)])
        );
        assert!(reconcile_groups(&mut recorded, WINDOW, &groups).is_empty());
    }

    #[test]
    fn recovery_does_not_overwrite_an_occupied_key() {
        let keys = BTreeMap::from([(KeyCode::ControlRight, Key::Character("x".into()))]);
        let mut recorded = keys.clone();
        assert!(
            reconcile_groups(&mut recorded, WINDOW, &[held_control([Unknown, Pressed])]).is_empty()
        );
        assert_eq!(recorded, keys);
    }

    #[test]
    fn recovery_does_not_duplicate_a_remapped_modifier() {
        let mut recorded = BTreeMap::from([
            (KeyCode::CapsLock, Key::Control),
            (KeyCode::ControlLeft, Key::Control),
        ]);
        let events = reconcile_groups(&mut recorded, WINDOW, &[held_control([Unknown, Pressed])]);
        assert_eq!(
            events,
            vec![input(
                KeyCode::ControlLeft,
                Key::Control,
                ButtonState::Released
            )]
        );
        assert_eq!(
            recorded,
            BTreeMap::from([(KeyCode::CapsLock, Key::Control)])
        );
    }

    #[test]
    fn recovery_does_not_reuse_a_key_released_in_the_same_batch() {
        let mut recorded = BTreeMap::from([(KeyCode::ControlRight, Key::Alt)]);
        let groups = [
            held_control([Unknown, Pressed]),
            ModifierGroup {
                logical_key: Key::Alt,
                group_pressed: false,
                sides: [(KeyCode::AltLeft, Unknown), (KeyCode::AltRight, Unknown)],
            },
        ];
        assert_eq!(
            reconcile_groups(&mut recorded, WINDOW, &groups),
            vec![input(
                KeyCode::ControlRight,
                Key::Alt,
                ButtonState::Released
            )]
        );
        assert!(recorded.is_empty());
    }

    #[test]
    fn switching_sides_keeps_logical_input_pressed() {
        let mut app = App::new();
        app.add_plugins(InputPlugin);
        app.world_mut().write_message(input(
            KeyCode::ControlLeft,
            Key::Control,
            ButtonState::Pressed,
        ));
        app.update();
        let mut recorded = BTreeMap::from([(KeyCode::ControlLeft, Key::Control)]);

        let events = reconcile_groups(&mut recorded, WINDOW, &[held_control([Unknown, Pressed])]);
        app.world_mut().write_message_batch(events.iter().cloned());
        app.update();

        let physical = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(!physical.pressed(KeyCode::ControlLeft));
        assert!(physical.pressed(KeyCode::ControlRight));
        assert!(app
            .world()
            .resource::<ButtonInput<Key>>()
            .pressed(Key::Control));
        assert_eq!(
            recorded,
            BTreeMap::from([(KeyCode::ControlRight, Key::Control)])
        );
        assert_eq!(
            events,
            vec![
                input(KeyCode::ControlLeft, Key::Control, ButtonState::Released),
                input(KeyCode::ControlRight, Key::Control, ButtonState::Pressed),
            ]
        );
    }
}
