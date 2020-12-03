use crate::{ElementState, Input};
use bevy_app::prelude::*;
use bevy_ecs::{Local, Res, ResMut};

/// A key input event from a keyboard device
#[derive(Debug, Clone)]
pub struct KeyboardInput {
    pub scan_code: u32,
    pub key_code: Option<KeyCode>,
    pub state: ElementState,
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

pub mod modifiers {
    /// Identifies a key modifier
    #[derive(Debug, Default, Hash, PartialEq, Eq, Clone, Copy)]
    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    pub struct KeyModifiers(u8);

    macro_rules! key_modifiers_accessors {
        ( $( $mask:ident => $get:ident , $set:ident , $clear:ident ; )* ) => {

            $(

                pub fn $get (&self) -> bool { self.0 & Self::$mask == Self::$mask }

                pub fn $set (&mut self) { self.0 |= Self::$mask; }

                pub fn $clear (&mut self) { self.0 &= !Self::$mask; }

            )*


        }
    }

    impl KeyModifiers {
        pub const MASK_ALT: u8 = 4;
        pub const MASK_CTRL: u8 = 2;
        pub const MASK_LOGO: u8 = 8;
        pub const MASK_SHIFT: u8 = 1;

        key_modifiers_accessors! {
            MASK_SHIFT => has_shift, set_shift, clear_shift;
            MASK_CTRL  => has_ctrl , set_ctrl , clear_ctrl ;
            MASK_ALT   => has_alt  , set_alt  , clear_alt  ;
            MASK_LOGO  => has_logo , set_logo , clear_logo ;
        }

        pub fn from_raw(value: u8) -> Option<Self> {
            if (value & !(Self::MASK_SHIFT | Self::MASK_CTRL | Self::MASK_ALT | Self::MASK_LOGO))
                == 0
            {
                Some(Self(value))
            } else {
                None
            }
        }
    }

    impl std::ops::BitOr for KeyModifiers {
        type Output = Self;

        fn bitor(self, rhs: Self) -> Self::Output {
            Self(self.0 | rhs.0)
        }
    }

    pub const SHIFT: KeyModifiers = KeyModifiers(KeyModifiers::MASK_SHIFT);
    pub const CTRL: KeyModifiers = KeyModifiers(KeyModifiers::MASK_CTRL);
    pub const ALT: KeyModifiers = KeyModifiers(KeyModifiers::MASK_ALT);
    pub const LOGO: KeyModifiers = KeyModifiers(KeyModifiers::MASK_LOGO);
}

pub use modifiers::KeyModifiers;
