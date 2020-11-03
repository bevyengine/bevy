use crate::Input;
use bevy_app::prelude::*;
use bevy_ecs::{Local, Res, ResMut};

/// A key input event from a keyboard device
#[derive(Debug, Clone)]
pub struct KeyboardInput {
    pub scan_code: u32,
    pub key_code: Option<KeyCode>,
    pub state: ElementState,
}

/// The current "press" state of an element
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ElementState {
    Pressed,
    Released,
}

impl ElementState {
    pub fn is_pressed(&self) -> bool {
        matches!(self, ElementState::Pressed)
    }
}

/// State used by the keyboard input system
#[derive(Default)]
pub struct KeyboardInputState {
    keyboard_input_event_reader: EventReader<KeyboardInput>,
}

/// Updates the Input<KeyCode> resource with the latest KeyboardInput events
pub fn keyboard_input_system(
    mut state: Local<KeyboardInputState>,
    mut keyboard_input: ResMut<Input<KeyCode>>,
    keyboard_input_events: Res<Events<KeyboardInput>>,
) {
    keyboard_input.update();
    for event in state
        .keyboard_input_event_reader
        .iter(&keyboard_input_events)
    {
        if let KeyboardInput {
            key_code: Some(key_code),
            state,
            ..
        } = event
        {
            match state {
                ElementState::Pressed => keyboard_input.press(*key_code),
                ElementState::Released => keyboard_input.release(*key_code),
            }
        }
    }
}

/// The key code of a keyboard input.
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[repr(u32)]
pub enum KeyCode {
    /// The '1' key over the letters.
    Key1,
    /// The '2' key over the letters.
    Key2,
    /// The '3' key over the letters.
    Key3,
    /// The '4' key over the letters.
    Key4,
    /// The '5' key over the letters.
    Key5,
    /// The '6' key over the letters.
    Key6,
    /// The '7' key over the letters.
    Key7,
    /// The '8' key over the letters.
    Key8,
    /// The '9' key over the letters.
    Key9,
    /// The '0' key over the 'O' and 'P' keys.
    Key0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    /// The Escape key, next to F1.
    Escape,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    /// Print Screen/SysRq.
    Snapshot,
    /// Scroll Lock.
    Scroll,
    /// Pause/Break key, next to Scroll lock.
    Pause,

    /// `Insert`, next to Backspace.
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,

    Left,
    Up,
    Right,
    Down,

    /// The Backspace key, right over Enter.
    Back,
    /// The Enter key.
    Return,
    /// The space bar.
    Space,

    /// The "Compose" key on Linux.
    Compose,

    Caret,

    Numlock,
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,

    AbntC1,
    AbntC2,
    NumpadAdd,
    Apostrophe,
    Apps,
    Asterisk,
    Plus,
    At,
    Ax,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    NumpadDecimal,
    NumpadDivide,
    Equals,
    Grave,
    Kana,
    Kanji,
    LAlt,
    LBracket,
    LControl,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    NumpadMultiply,
    Mute,
    MyComputer,
    NavigateForward,  // also called "Prior"
    NavigateBackward, // also called "Next"
    NextTrack,
    NoConvert,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    OEM102,
    Period,
    PlayPause,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    NumpadSubtract,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,
}

impl KeyCode {
    /// Converts a key into its textual char representation on a QWERTY keyboard depending on the provided shift status.
    ///
    /// Calls [`KeyCode::to_qwerty_char_with_shift`] if shift is down, or [`KeyCode::to_qwerty_char_without_shift`] if shift isn't down.
    ///
    /// Returns `None` if the key has no valid textual mapping on a QWERTY keyboard with the provided shift status.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::prelude::KeyCode;
    /// #
    /// let c1 = KeyCode::Slash.to_qwerty_char(false);
    /// assert_eq!(c1, Some('/'));
    ///
    /// let c2 = KeyCode::Slash.to_qwerty_char(true);
    /// assert_eq!(c2, Some('?'));
    ///
    /// let c3 = KeyCode::End.to_qwerty_char(true);
    /// assert_eq!(c3, None);
    /// ```
    pub fn to_qwerty_char(self, shift_down: bool) -> Option<char> {
        if shift_down {
            self.to_qwerty_char_with_shift()
        } else {
            self.to_qwerty_char_without_shift()
        }
    }

    /// Converts a key into its textual char representation on a QWERTY keyboard when shift is not pressed.
    ///
    /// Returns `None` if the key has no valid textual mapping on a QWERTY keyboard without shift pressed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::prelude::KeyCode;
    /// #
    /// let c1 = KeyCode::Slash.to_qwerty_char_without_shift();
    /// assert_eq!(c1, Some('/'))
    /// ```
    /// This will cause the slash key to return an actual forward slash character (`/`).
    /// To get the question mark character (`?`), which is associated with the same key,
    /// you must use [`KeyCode::to_qwerty_char_with_shift`].
    ///
    /// ```
    /// # use bevy_input::prelude::KeyCode;
    /// #
    /// let c2 = KeyCode::End.to_qwerty_char_without_shift();
    /// assert_eq!(c2, None);
    /// ```
    /// Since the End key has no textual representation, the function returns `None`.
    pub fn to_qwerty_char_without_shift(self) -> Option<char> {
        let out = match self {
            KeyCode::Key1 => '1',
            KeyCode::Key2 => '2',
            KeyCode::Key3 => '3',
            KeyCode::Key4 => '4',
            KeyCode::Key5 => '5',
            KeyCode::Key6 => '6',
            KeyCode::Key7 => '7',
            KeyCode::Key8 => '8',
            KeyCode::Key9 => '9',
            KeyCode::Key0 => '0',

            KeyCode::A => 'a',
            KeyCode::B => 'b',
            KeyCode::C => 'c',
            KeyCode::D => 'd',
            KeyCode::E => 'e',
            KeyCode::F => 'f',
            KeyCode::G => 'g',
            KeyCode::H => 'h',
            KeyCode::I => 'i',
            KeyCode::J => 'j',
            KeyCode::K => 'k',
            KeyCode::L => 'l',
            KeyCode::M => 'm',
            KeyCode::N => 'n',
            KeyCode::O => 'o',
            KeyCode::P => 'p',
            KeyCode::Q => 'q',
            KeyCode::R => 'r',
            KeyCode::S => 's',
            KeyCode::T => 't',
            KeyCode::U => 'u',
            KeyCode::V => 'v',
            KeyCode::W => 'w',
            KeyCode::X => 'x',
            KeyCode::Y => 'y',
            KeyCode::Z => 'z',

            KeyCode::Numpad0 => '0',
            KeyCode::Numpad1 => '1',
            KeyCode::Numpad2 => '2',
            KeyCode::Numpad3 => '3',
            KeyCode::Numpad4 => '4',
            KeyCode::Numpad5 => '5',
            KeyCode::Numpad6 => '6',
            KeyCode::Numpad7 => '7',
            KeyCode::Numpad8 => '8',
            KeyCode::Numpad9 => '9',

            KeyCode::NumpadAdd => '+',
            KeyCode::Apostrophe => '\'',
            KeyCode::Backslash => '\\',
            KeyCode::Comma => ',',
            KeyCode::NumpadDecimal => '.',
            KeyCode::NumpadDivide => '/',
            KeyCode::Equals => '=',
            KeyCode::Grave => '`',
            KeyCode::LBracket => '[',
            KeyCode::Minus => '-',
            KeyCode::Period => '.',
            KeyCode::RBracket => ']',
            KeyCode::Semicolon => ';',
            KeyCode::Slash => '/',
            KeyCode::Tab => '\t',

            _ => return None,
        };

        Some(out)
    }

    /// Converts a key into its textual char representation on a QWERTY keyboard when shift is pressed.
    ///
    /// Returns `None` if the key has no valid textual mapping on a QWERTY keyboard with shift pressed.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_input::prelude::KeyCode;
    /// #
    /// let c1 = KeyCode::Slash.to_qwerty_char_with_shift();
    /// assert_eq!(c1, Some('?'));
    /// ```
    /// This will cause the slash key to return a question mark character (`?`).
    /// To get the actual forward slash character (`/`), which is associated with the same key,
    /// you must use [`KeyCode::to_qwerty_char_without_shift`].
    ///
    /// ```
    /// # use bevy_input::prelude::KeyCode;
    /// #
    /// let c2 = KeyCode::End.to_qwerty_char_with_shift();
    /// assert_eq!(c2, None);
    /// ```
    /// Since the End key has no textual representation, the function returns `None`.
    pub fn to_qwerty_char_with_shift(self) -> Option<char> {
        let out = match self {
            KeyCode::Key1 => '!',
            KeyCode::Key2 => '@',
            KeyCode::Key3 => '#',
            KeyCode::Key4 => '$',
            KeyCode::Key5 => '%',
            KeyCode::Key6 => '^',
            KeyCode::Key7 => '&',
            KeyCode::Key8 => '*',
            KeyCode::Key9 => '(',
            KeyCode::Key0 => ')',

            KeyCode::A => 'A',
            KeyCode::B => 'B',
            KeyCode::C => 'C',
            KeyCode::D => 'D',
            KeyCode::E => 'E',
            KeyCode::F => 'F',
            KeyCode::G => 'G',
            KeyCode::H => 'H',
            KeyCode::I => 'I',
            KeyCode::J => 'J',
            KeyCode::K => 'K',
            KeyCode::L => 'L',
            KeyCode::M => 'M',
            KeyCode::N => 'N',
            KeyCode::O => 'O',
            KeyCode::P => 'P',
            KeyCode::Q => 'Q',
            KeyCode::R => 'R',
            KeyCode::S => 'S',
            KeyCode::T => 'T',
            KeyCode::U => 'U',
            KeyCode::V => 'V',
            KeyCode::W => 'W',
            KeyCode::X => 'X',
            KeyCode::Y => 'Y',
            KeyCode::Z => 'Z',

            KeyCode::NumpadAdd => '+',
            KeyCode::Apostrophe => '"',
            KeyCode::Backslash => '|',
            KeyCode::Comma => '<',
            KeyCode::NumpadDivide => '/',
            KeyCode::Equals => '+',
            KeyCode::Grave => '~',
            KeyCode::LBracket => '{',
            KeyCode::Minus => '_',
            KeyCode::Period => '>',
            KeyCode::RBracket => '}',
            KeyCode::Semicolon => ':',
            KeyCode::Slash => '?',
            KeyCode::Tab => '\t',

            _ => return None,
        };

        Some(out)
    }
}
