//! Reconcile native logical modifier state without assuming a canonical keymap.
use bevy_input::keyboard::{Key, KeyCode};
use std::collections::HashMap;
use winit::keyboard::ModifiersKeyState;

/// Reconcile recorded logical modifiers while preserving physical key codes.
pub(crate) fn transitions(
    recorded: &HashMap<KeyCode, Key>,
    logical: &Key,
    sides: [(KeyCode, ModifiersKeyState); 2],
    group_pressed: bool,
) -> Vec<(KeyCode, bool)> {
    let mut changes = Vec::new();
    let any_side_pressed = sides
        .iter()
        .any(|(_, state)| *state == ModifiersKeyState::Pressed);
    let remapped_group_recorded = recorded
        .iter()
        .any(|(code, key)| key == logical && !sides.iter().any(|(canonical, _)| canonical == code));
    for (&code, key) in recorded {
        if key != logical {
            continue;
        }
        // Use side-specific state only when the physical key matches the logical group.
        let canonical_side_released = sides.iter().any(|(canonical, state)| {
            *canonical == code && *state == ModifiersKeyState::Unknown && any_side_pressed
        });
        if !group_pressed || canonical_side_released {
            changes.push((code, false));
        }
    }
    for (code, state) in sides {
        if state == ModifiersKeyState::Pressed
            && !recorded.contains_key(&code)
            && !remapped_group_recorded
        {
            changes.push((code, true));
        }
    }
    changes
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use ModifiersKeyState::{Pressed, Unknown};

    fn modifier_groups() -> [(Key, [KeyCode; 2]); 4] {
        [
            (Key::Control, [KeyCode::ControlLeft, KeyCode::ControlRight]),
            (Key::Alt, [KeyCode::AltLeft, KeyCode::AltRight]),
            (Key::Shift, [KeyCode::ShiftLeft, KeyCode::ShiftRight]),
            (Key::Super, [KeyCode::SuperLeft, KeyCode::SuperRight]),
        ]
    }

    fn control() -> [(KeyCode, ModifiersKeyState); 2] {
        [
            (KeyCode::ControlLeft, Unknown),
            (KeyCode::ControlRight, Unknown),
        ]
    }
    fn alt() -> [(KeyCode, ModifiersKeyState); 2] {
        [(KeyCode::AltLeft, Unknown), (KeyCode::AltRight, Unknown)]
    }

    #[test]
    fn swapped_control_alt_follow_logical_modifier_bits() {
        let recorded = HashMap::from([(KeyCode::ControlLeft, Key::Alt)]);
        assert!(transitions(&recorded, &Key::Control, control(), false).is_empty());
        assert!(transitions(&recorded, &Key::Alt, alt(), true).is_empty());
        assert_eq!(
            transitions(&recorded, &Key::Alt, alt(), false),
            vec![(KeyCode::ControlLeft, false)]
        );
        let recorded = HashMap::from([(KeyCode::AltRight, Key::Control)]);
        assert!(transitions(&recorded, &Key::Alt, alt(), false).is_empty());
        assert!(transitions(&recorded, &Key::Control, control(), true).is_empty());
        assert_eq!(
            transitions(&recorded, &Key::Control, control(), false),
            vec![(KeyCode::AltRight, false)]
        );
    }

    #[test]
    fn canonical_lost_keyup_and_attested_focus_recovery_are_preserved() {
        for (logical, codes) in modifier_groups() {
            let unknown = codes.map(|code| (code, Unknown));
            assert!(transitions(&HashMap::default(), &logical, unknown, true).is_empty());
            for (side, &code) in codes.iter().enumerate() {
                let recorded = HashMap::from([(code, logical.clone())]);
                assert_eq!(
                    transitions(&recorded, &logical, unknown, false),
                    vec![(code, false)]
                );
                assert!(transitions(&recorded, &logical, unknown, true).is_empty());
                let mut attested = unknown;
                attested[side].1 = Pressed;
                assert_eq!(
                    transitions(&HashMap::default(), &logical, attested, true),
                    vec![(code, true)]
                );
                assert!(transitions(&recorded, &logical, attested, true).is_empty());
            }
            assert_eq!(
                transitions(
                    &HashMap::default(),
                    &logical,
                    codes.map(|code| (code, Pressed)),
                    true
                ),
                codes.map(|code| (code, true))
            );
        }
    }

    #[test]
    fn attested_canonical_side_does_not_duplicate_or_release_a_remapped_group() {
        for (logical, codes) in modifier_groups() {
            for (side, &code) in codes.iter().enumerate() {
                let mut attested = codes.map(|code| (code, Unknown));
                attested[side].1 = Pressed;
                let remapped = HashMap::from([(KeyCode::CapsLock, logical.clone())]);
                assert!(transitions(&remapped, &logical, attested, true).is_empty());
                let occupied = HashMap::from([(code, Key::Character("x".into()))]);
                assert!(transitions(&occupied, &logical, attested, true).is_empty());
            }
        }
    }

    #[test]
    fn side_reconciliation_applies_only_to_matching_canonical_logical_group() {
        for (logical, codes) in modifier_groups() {
            for (side, &held) in codes.iter().enumerate() {
                let released = codes[1 - side];
                let mut attested = codes.map(|code| (code, Unknown));
                attested[side].1 = Pressed;
                let mut recorded = HashMap::from([(released, logical.clone())]);
                assert_eq!(
                    transitions(&recorded, &logical, attested, true),
                    vec![(released, false), (held, true)]
                );
                recorded.insert(KeyCode::CapsLock, logical.clone());
                assert_eq!(
                    transitions(&recorded, &logical, attested, true),
                    vec![(released, false)]
                );
            }
        }
    }

    #[test]
    fn multiple_keys_in_one_logical_group_follow_aggregate_state() {
        for (logical, codes) in modifier_groups() {
            for physical_codes in [codes, [KeyCode::CapsLock, KeyCode::KeyA]] {
                let recorded = HashMap::from(physical_codes.map(|code| (code, logical.clone())));
                let unknown = codes.map(|code| (code, Unknown));
                assert!(transitions(&recorded, &logical, unknown, true).is_empty());
                assert_eq!(
                    transitions(&recorded, &logical, unknown, false)
                        .into_iter()
                        .collect::<HashSet<_>>(),
                    HashSet::from(physical_codes.map(|code| (code, false)))
                );
            }
        }
    }

    #[test]
    fn altgraph_and_character_remaps_are_not_released_by_unrelated_modifier_bits() {
        for (code, key) in [
            (KeyCode::AltRight, Key::AltGraph),
            (KeyCode::ControlLeft, Key::Character("x".into())),
        ] {
            let recorded = HashMap::from([(code, key)]);
            for (logical, sides) in [
                (Key::Alt, alt()),
                (Key::Control, control()),
                (
                    Key::Shift,
                    [
                        (KeyCode::ShiftLeft, Unknown),
                        (KeyCode::ShiftRight, Unknown),
                    ],
                ),
                (
                    Key::Super,
                    [
                        (KeyCode::SuperLeft, Unknown),
                        (KeyCode::SuperRight, Unknown),
                    ],
                ),
            ] {
                assert!(transitions(&recorded, &logical, sides, false).is_empty());
            }
        }
    }
}
