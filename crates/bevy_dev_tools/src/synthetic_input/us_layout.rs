//! What a US keyboard sends for a key: its [`KeyCode`], logical [`Key`] and text.

use bevy_input::keyboard::{Key, KeyCode};

/// Each printable key, with the character it types unshifted and shifted.
const PRINTABLE: &[(KeyCode, char, char)] = &[
    (KeyCode::Backquote, '`', '~'),
    (KeyCode::Minus, '-', '_'),
    (KeyCode::Equal, '=', '+'),
    (KeyCode::BracketLeft, '[', '{'),
    (KeyCode::BracketRight, ']', '}'),
    (KeyCode::Backslash, '\\', '|'),
    (KeyCode::Semicolon, ';', ':'),
    (KeyCode::Quote, '\'', '"'),
    (KeyCode::Comma, ',', '<'),
    (KeyCode::Period, '.', '>'),
    (KeyCode::Slash, '/', '?'),
    (KeyCode::Digit0, '0', ')'),
    (KeyCode::Digit1, '1', '!'),
    (KeyCode::Digit2, '2', '@'),
    (KeyCode::Digit3, '3', '#'),
    (KeyCode::Digit4, '4', '$'),
    (KeyCode::Digit5, '5', '%'),
    (KeyCode::Digit6, '6', '^'),
    (KeyCode::Digit7, '7', '&'),
    (KeyCode::Digit8, '8', '*'),
    (KeyCode::Digit9, '9', '('),
    (KeyCode::KeyA, 'a', 'A'),
    (KeyCode::KeyB, 'b', 'B'),
    (KeyCode::KeyC, 'c', 'C'),
    (KeyCode::KeyD, 'd', 'D'),
    (KeyCode::KeyE, 'e', 'E'),
    (KeyCode::KeyF, 'f', 'F'),
    (KeyCode::KeyG, 'g', 'G'),
    (KeyCode::KeyH, 'h', 'H'),
    (KeyCode::KeyI, 'i', 'I'),
    (KeyCode::KeyJ, 'j', 'J'),
    (KeyCode::KeyK, 'k', 'K'),
    (KeyCode::KeyL, 'l', 'L'),
    (KeyCode::KeyM, 'm', 'M'),
    (KeyCode::KeyN, 'n', 'N'),
    (KeyCode::KeyO, 'o', 'O'),
    (KeyCode::KeyP, 'p', 'P'),
    (KeyCode::KeyQ, 'q', 'Q'),
    (KeyCode::KeyR, 'r', 'R'),
    (KeyCode::KeyS, 's', 'S'),
    (KeyCode::KeyT, 't', 'T'),
    (KeyCode::KeyU, 'u', 'U'),
    (KeyCode::KeyV, 'v', 'V'),
    (KeyCode::KeyW, 'w', 'W'),
    (KeyCode::KeyX, 'x', 'X'),
    (KeyCode::KeyY, 'y', 'Y'),
    (KeyCode::KeyZ, 'z', 'Z'),
];

/// The key that types `character`, and whether it needs Shift.
pub(super) fn character_key(character: char) -> Option<(KeyCode, bool)> {
    if character == ' ' {
        return Some((KeyCode::Space, false));
    }
    PRINTABLE.iter().find_map(|&(key_code, plain, shifted)| {
        (character == plain)
            .then_some((key_code, false))
            .or((character == shifted).then_some((key_code, true)))
    })
}

/// The logical key and text of an unshifted press of `key_code`.
pub(super) fn logical_key(key_code: KeyCode) -> Option<(Key, Option<char>)> {
    if let Some(&(_, plain, _)) = PRINTABLE.iter().find(|(code, ..)| *code == key_code) {
        return Some((
            Key::Character(plain.encode_utf8(&mut [0; 4]).into()),
            Some(plain),
        ));
    }
    let named = match key_code {
        // A named key that also types a character.
        KeyCode::Space => return Some((Key::Space, Some(' '))),
        KeyCode::Enter => Key::Enter,
        KeyCode::Tab => Key::Tab,
        KeyCode::Escape => Key::Escape,
        KeyCode::Backspace => Key::Backspace,
        KeyCode::Delete => Key::Delete,
        KeyCode::Insert => Key::Insert,
        KeyCode::Home => Key::Home,
        KeyCode::End => Key::End,
        KeyCode::PageUp => Key::PageUp,
        KeyCode::PageDown => Key::PageDown,
        KeyCode::ArrowUp => Key::ArrowUp,
        KeyCode::ArrowDown => Key::ArrowDown,
        KeyCode::ArrowLeft => Key::ArrowLeft,
        KeyCode::ArrowRight => Key::ArrowRight,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Key::Shift,
        KeyCode::ControlLeft | KeyCode::ControlRight => Key::Control,
        KeyCode::AltLeft | KeyCode::AltRight => Key::Alt,
        KeyCode::SuperLeft | KeyCode::SuperRight => Key::Super,
        KeyCode::F1 => Key::F1,
        KeyCode::F2 => Key::F2,
        KeyCode::F3 => Key::F3,
        KeyCode::F4 => Key::F4,
        KeyCode::F5 => Key::F5,
        KeyCode::F6 => Key::F6,
        KeyCode::F7 => Key::F7,
        KeyCode::F8 => Key::F8,
        KeyCode::F9 => Key::F9,
        KeyCode::F10 => Key::F10,
        KeyCode::F11 => Key::F11,
        KeyCode::F12 => Key::F12,
        _ => return None,
    };
    Some((named, None))
}
