//! The keyboard input functionality.

use crate::{ButtonState, Input};
use bevy_ecs::entity::Entity;
use bevy_ecs::{
    change_detection::DetectChangesMut,
    event::{Event, EventReader},
    system::ResMut,
};
use bevy_reflect::Reflect;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A keyboard input event.
///
/// This event is the translated version of the `WindowEvent::KeyboardInput` from the `winit` crate.
/// It is available to the end user and can be used for game logic.
///
/// ## Usage
///
/// The event is consumed inside of the [`keyboard_input_system`]
/// to update the [`Input<KeyCode>`](crate::Input<KeyCode>) resource.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct KeyboardInput {
    /// The scan code of the key.
    pub scan_code: u32,
    /// The key code of the key.
    pub key_code: Option<KeyCode>,
    /// The press state of the key.
    pub state: ButtonState,
    /// Window that received the input.
    pub window: Entity,
}

/// Updates the [`Input<KeyCode>`] resource with the latest [`KeyboardInput`] events.
///
/// ## Differences
///
/// The main difference between the [`KeyboardInput`] event and the [`Input<KeyCode>`] or [`Input<ScanCode>`] resources is that
/// the latter have convenient functions such as [`Input::pressed`], [`Input::just_pressed`] and [`Input::just_released`].
pub fn keyboard_input_system(
    mut scan_input: ResMut<Input<ScanCode>>,
    mut key_input: ResMut<Input<KeyCode>>,
    mut keyboard_input_events: EventReader<KeyboardInput>,
) {
    // Avoid clearing if it's not empty to ensure change detection is not triggered.
    scan_input.bypass_change_detection().clear();
    key_input.bypass_change_detection().clear();
    for event in keyboard_input_events.read() {
        let KeyboardInput {
            scan_code, state, ..
        } = event;
        if let Some(key_code) = event.key_code {
            match state {
                ButtonState::Pressed => key_input.press(key_code),
                ButtonState::Released => key_input.release(key_code),
            }
        }
        match state {
            ButtonState::Pressed => scan_input.press(ScanCode(*scan_code)),
            ButtonState::Released => scan_input.release(ScanCode(*scan_code)),
        }
    }
}

/// The key code of a [`KeyboardInput`].
///
/// ## Usage
///
/// It is used as the generic `T` value of an [`Input`] to create a `Res<Input<KeyCode>>`.
/// The resource values are mapped to the current layout of the keyboard and correlate to an [`ScanCode`].
///
/// ## Updating
///
/// The resource is updated inside of the [`keyboard_input_system`].
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Reflect)]
#[reflect(Debug, Hash, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
#[repr(u32)]
pub enum KeyCode {
    /// The `1` key over the letters.
    Key1,
    /// The `2` key over the letters.
    Key2,
    /// The `3` key over the letters.
    Key3,
    /// The `4` key over the letters.
    Key4,
    /// The `5` key over the letters.
    Key5,
    /// The `6` key over the letters.
    Key6,
    /// The `7` key over the letters.
    Key7,
    /// The `8` key over the letters.
    Key8,
    /// The `9` key over the letters.
    Key9,
    /// The `0` key over the letters.
    Key0,

    /// The `A` key.
    A,
    /// The `B` key.
    B,
    /// The `C` key.
    C,
    /// The `D` key.
    D,
    /// The `E` key.
    E,
    /// The `F` key.
    F,
    /// The `G` key.
    G,
    /// The `H` key.
    H,
    /// The `I` key.
    I,
    /// The `J` key.
    J,
    /// The `K` key.
    K,
    /// The `L` key.
    L,
    /// The `M` key.
    M,
    /// The `N` key.
    N,
    /// The `O` key.
    O,
    /// The `P` key.
    P,
    /// The `Q` key.
    Q,
    /// The `R` key.
    R,
    /// The `S` key.
    S,
    /// The `T` key.
    T,
    /// The `U` key.
    U,
    /// The `V` key.
    V,
    /// The `W` key.
    W,
    /// The `X` key.
    X,
    /// The `Y` key.
    Y,
    /// The `Z` key.
    Z,

    /// The `Escape` / `ESC` key, next to the `F1` key.
    Escape,

    /// The `F1` key.
    F1,
    /// The `F2` key.
    F2,
    /// The `F3` key.
    F3,
    /// The `F4` key.
    F4,
    /// The `F5` key.
    F5,
    /// The `F6` key.
    F6,
    /// The `F7` key.
    F7,
    /// The `F8` key.
    F8,
    /// The `F9` key.
    F9,
    /// The `F10` key.
    F10,
    /// The `F11` key.
    F11,
    /// The `F12` key.
    F12,
    /// The `F13` key.
    F13,
    /// The `F14` key.
    F14,
    /// The `F15` key.
    F15,
    /// The `F16` key.
    F16,
    /// The `F17` key.
    F17,
    /// The `F18` key.
    F18,
    /// The `F19` key.
    F19,
    /// The `F20` key.
    F20,
    /// The `F21` key.
    F21,
    /// The `F22` key.
    F22,
    /// The `F23` key.
    F23,
    /// The `F24` key.
    F24,

    /// The `Snapshot` / `Print Screen` key.
    Snapshot,
    /// The `Scroll` / `Scroll Lock` key.
    Scroll,
    /// The `Pause` / `Break` key, next to the `Scroll` key.
    Pause,

    /// The `Insert` key, next to the `Backspace` key.
    Insert,
    /// The `Home` key.
    Home,
    /// The `Delete` key.
    Delete,
    /// The `End` key.
    End,
    /// The `PageDown` key.
    PageDown,
    /// The `PageUp` key.
    PageUp,

    /// The `Left` / `Left Arrow` key.
    Left,
    /// The `Up` / `Up Arrow` key.
    Up,
    /// The `Right` / `Right Arrow` key.
    Right,
    /// The `Down` / `Down Arrow` key.
    Down,

    /// The `Back` / `Backspace` key.
    Back,
    /// The `Return` / `Enter` key.
    Return,
    /// The `Space` / `Spacebar` / ` ` key.
    Space,

    /// The `Compose` key on Linux.
    Compose,
    /// The `Caret` / `^` key.
    Caret,

    /// The `Numlock` key.
    Numlock,
    /// The `Numpad0` / `0` key.
    Numpad0,
    /// The `Numpad1` / `1` key.
    Numpad1,
    /// The `Numpad2` / `2` key.
    Numpad2,
    /// The `Numpad3` / `3` key.
    Numpad3,
    /// The `Numpad4` / `4` key.
    Numpad4,
    /// The `Numpad5` / `5` key.
    Numpad5,
    /// The `Numpad6` / `6` key.
    Numpad6,
    /// The `Numpad7` / `7` key.
    Numpad7,
    /// The `Numpad8` / `8` key.
    Numpad8,
    /// The `Numpad9` / `9` key.
    Numpad9,

    /// The `AbntC1` key.
    AbntC1,
    /// The `AbntC2` key.
    AbntC2,

    /// The `NumpadAdd` / `+` key.
    NumpadAdd,
    /// The `Apostrophe` / `'` key.
    Apostrophe,
    /// The `Apps` key.
    Apps,
    /// The `Asterisk` / `*` key.
    Asterisk,
    /// The `Plus` / `+` key.
    Plus,
    /// The `At` / `@` key.
    At,
    /// The `Ax` key.
    Ax,
    /// The `Backslash` / `\` key.
    Backslash,
    /// The `Calculator` key.
    Calculator,
    /// The `Capital` key.
    Capital,
    /// The `Colon` / `:` key.
    Colon,
    /// The `Comma` / `,` key.
    Comma,
    /// The `Convert` key.
    Convert,
    /// The `NumpadDecimal` / `.` key.
    NumpadDecimal,
    /// The `NumpadDivide` / `/` key.
    NumpadDivide,
    /// The `Equals` / `=` key.
    Equals,
    /// The `Grave` / `Backtick` / `` ` `` key.
    Grave,
    /// The `Kana` key.
    Kana,
    /// The `Kanji` key.
    Kanji,

    /// The `Left Alt` key. Maps to `Left Option` on Mac.
    AltLeft,
    /// The `Left Bracket` / `[` key.
    BracketLeft,
    /// The `Left Control` key.
    ControlLeft,
    /// The `Left Shift` key.
    ShiftLeft,
    /// The `Left Super` key.
    /// Generic keyboards usually display this key with the *Microsoft Windows* logo.
    /// Apple keyboards call this key the *Command Key* and display it using the ⌘ character.
    #[doc(alias("LWin", "LMeta", "LLogo"))]
    SuperLeft,

    /// The `Mail` key.
    Mail,
    /// The `MediaSelect` key.
    MediaSelect,
    /// The `MediaStop` key.
    MediaStop,
    /// The `Minus` / `-` key.
    Minus,
    /// The `NumpadMultiply` / `*` key.
    NumpadMultiply,
    /// The `Mute` key.
    Mute,
    /// The `MyComputer` key.
    MyComputer,
    /// The `NavigateForward` / `Prior` key.
    NavigateForward,
    /// The `NavigateBackward` / `Next` key.
    NavigateBackward,
    /// The `NextTrack` key.
    NextTrack,
    /// The `NoConvert` key.
    NoConvert,
    /// The `NumpadComma` / `,` key.
    NumpadComma,
    /// The `NumpadEnter` key.
    NumpadEnter,
    /// The `NumpadEquals` / `=` key.
    NumpadEquals,
    /// The `Oem102` key.
    Oem102,
    /// The `Period` / `.` key.
    Period,
    /// The `PlayPause` key.
    PlayPause,
    /// The `Power` key.
    Power,
    /// The `PrevTrack` key.
    PrevTrack,

    /// The `Right Alt` key. Maps to `Right Option` on Mac.
    AltRight,
    /// The `Right Bracket` / `]` key.
    BracketRight,
    /// The `Right Control` key.
    ControlRight,
    /// The `Right Shift` key.
    ShiftRight,
    /// The `Right Super` key.
    /// Generic keyboards usually display this key with the *Microsoft Windows* logo.
    /// Apple keyboards call this key the *Command Key* and display it using the ⌘ character.
    #[doc(alias("RWin", "RMeta", "RLogo"))]
    SuperRight,

    /// The `Semicolon` / `;` key.
    Semicolon,
    /// The `Slash` / `/` key.
    Slash,
    /// The `Sleep` key.
    Sleep,
    /// The `Stop` key.
    Stop,
    /// The `NumpadSubtract` / `-` key.
    NumpadSubtract,
    /// The `Sysrq` key.
    Sysrq,
    /// The `Tab` / `   ` key.
    Tab,
    /// The `Underline` / `_` key.
    Underline,
    /// The `Unlabeled` key.
    Unlabeled,

    /// The `VolumeDown` key.
    VolumeDown,
    /// The `VolumeUp` key.
    VolumeUp,

    /// The `Wake` key.
    Wake,

    /// The `WebBack` key.
    WebBack,
    /// The `WebFavorites` key.
    WebFavorites,
    /// The `WebForward` key.
    WebForward,
    /// The `WebHome` key.
    WebHome,
    /// The `WebRefresh` key.
    WebRefresh,
    /// The `WebSearch` key.
    WebSearch,
    /// The `WebStop` key.
    WebStop,

    /// The `Yen` key.
    Yen,

    /// The `Copy` key.
    Copy,
    /// The `Paste` key.
    Paste,
    /// The `Cut` key.
    Cut,
}

impl KeyCode {
    /// Naive mapping of keycode to `char`.
    ///
    /// This function ignores keyboard layout, for example:
    /// * difference between US/UK layouts
    /// * non-English layouts
    /// * layouts of non-standard keyboards with unusual extra keys
    ///
    /// When there's no obvious mapping (for example, for F1-F12 keys), `None` is returned.
    #[allow(clippy::match_same_arms)]
    pub fn char_in_us_layout(self, shift: bool) -> Option<char> {
        match (self, shift) {
            (KeyCode::Key1, false) => Some('1'),
            (KeyCode::Key1, true) => Some('!'),
            (KeyCode::Key2, false) => Some('2'),
            (KeyCode::Key2, true) => Some('@'),
            (KeyCode::Key3, false) => Some('3'),
            (KeyCode::Key3, true) => Some('#'),
            (KeyCode::Key4, false) => Some('4'),
            (KeyCode::Key4, true) => Some('$'),
            (KeyCode::Key5, false) => Some('5'),
            (KeyCode::Key5, true) => Some('%'),
            (KeyCode::Key6, false) => Some('6'),
            (KeyCode::Key6, true) => Some('^'),
            (KeyCode::Key7, false) => Some('7'),
            (KeyCode::Key7, true) => Some('&'),
            (KeyCode::Key8, false) => Some('8'),
            (KeyCode::Key8, true) => Some('*'),
            (KeyCode::Key9, false) => Some('9'),
            (KeyCode::Key9, true) => Some('('),
            (KeyCode::Key0, false) => Some('0'),
            (KeyCode::Key0, true) => Some(')'),
            (KeyCode::A, false) => Some('a'),
            (KeyCode::A, true) => Some('A'),
            (KeyCode::B, false) => Some('b'),
            (KeyCode::B, true) => Some('B'),
            (KeyCode::C, false) => Some('c'),
            (KeyCode::C, true) => Some('C'),
            (KeyCode::D, false) => Some('d'),
            (KeyCode::D, true) => Some('D'),
            (KeyCode::E, false) => Some('e'),
            (KeyCode::E, true) => Some('E'),
            (KeyCode::F, false) => Some('f'),
            (KeyCode::F, true) => Some('F'),
            (KeyCode::G, false) => Some('g'),
            (KeyCode::G, true) => Some('G'),
            (KeyCode::H, false) => Some('h'),
            (KeyCode::H, true) => Some('H'),
            (KeyCode::I, false) => Some('i'),
            (KeyCode::I, true) => Some('I'),
            (KeyCode::J, false) => Some('j'),
            (KeyCode::J, true) => Some('J'),
            (KeyCode::K, false) => Some('k'),
            (KeyCode::K, true) => Some('K'),
            (KeyCode::L, false) => Some('l'),
            (KeyCode::L, true) => Some('L'),
            (KeyCode::M, false) => Some('m'),
            (KeyCode::M, true) => Some('M'),
            (KeyCode::N, false) => Some('n'),
            (KeyCode::N, true) => Some('N'),
            (KeyCode::O, false) => Some('o'),
            (KeyCode::O, true) => Some('O'),
            (KeyCode::P, false) => Some('p'),
            (KeyCode::P, true) => Some('P'),
            (KeyCode::Q, false) => Some('q'),
            (KeyCode::Q, true) => Some('Q'),
            (KeyCode::R, false) => Some('r'),
            (KeyCode::R, true) => Some('R'),
            (KeyCode::S, false) => Some('s'),
            (KeyCode::S, true) => Some('S'),
            (KeyCode::T, false) => Some('t'),
            (KeyCode::T, true) => Some('T'),
            (KeyCode::U, false) => Some('u'),
            (KeyCode::U, true) => Some('U'),
            (KeyCode::V, false) => Some('v'),
            (KeyCode::V, true) => Some('V'),
            (KeyCode::W, false) => Some('w'),
            (KeyCode::W, true) => Some('W'),
            (KeyCode::X, false) => Some('x'),
            (KeyCode::X, true) => Some('X'),
            (KeyCode::Y, false) => Some('y'),
            (KeyCode::Y, true) => Some('Y'),
            (KeyCode::Z, false) => Some('z'),
            (KeyCode::Z, true) => Some('Z'),
            (KeyCode::Escape, _) => None,
            (KeyCode::F1, _) => None,
            (KeyCode::F2, _) => None,
            (KeyCode::F3, _) => None,
            (KeyCode::F4, _) => None,
            (KeyCode::F5, _) => None,
            (KeyCode::F6, _) => None,
            (KeyCode::F7, _) => None,
            (KeyCode::F8, _) => None,
            (KeyCode::F9, _) => None,
            (KeyCode::F10, _) => None,
            (KeyCode::F11, _) => None,
            (KeyCode::F12, _) => None,
            (KeyCode::F13, _) => None,
            (KeyCode::F14, _) => None,
            (KeyCode::F15, _) => None,
            (KeyCode::F16, _) => None,
            (KeyCode::F17, _) => None,
            (KeyCode::F18, _) => None,
            (KeyCode::F19, _) => None,
            (KeyCode::F20, _) => None,
            (KeyCode::F21, _) => None,
            (KeyCode::F22, _) => None,
            (KeyCode::F23, _) => None,
            (KeyCode::F24, _) => None,
            (KeyCode::Snapshot, _) => None,
            (KeyCode::Scroll, _) => None,
            (KeyCode::Pause, _) => None,
            (KeyCode::Insert, _) => None,
            (KeyCode::Home, _) => None,
            (KeyCode::Delete, _) => None,
            (KeyCode::End, _) => None,
            (KeyCode::PageDown, _) => None,
            (KeyCode::PageUp, _) => None,
            (KeyCode::Left, _) => None,
            (KeyCode::Up, _) => None,
            (KeyCode::Right, _) => None,
            (KeyCode::Down, _) => None,
            (KeyCode::Back, _) => None,
            (KeyCode::Return, _) => None,
            (KeyCode::Space, _) => Some(' '),
            (KeyCode::Compose, _) => None,
            (KeyCode::Caret, _) => None,
            (KeyCode::Numlock, _) => None,
            (KeyCode::Numpad0, false) => Some('0'),
            (KeyCode::Numpad0, true) => None,
            (KeyCode::Numpad1, false) => Some('1'),
            (KeyCode::Numpad1, true) => None,
            (KeyCode::Numpad2, false) => Some('2'),
            (KeyCode::Numpad2, true) => None,
            (KeyCode::Numpad3, false) => Some('3'),
            (KeyCode::Numpad3, true) => None,
            (KeyCode::Numpad4, false) => Some('4'),
            (KeyCode::Numpad4, true) => None,
            (KeyCode::Numpad5, false) => Some('5'),
            (KeyCode::Numpad5, true) => None,
            (KeyCode::Numpad6, false) => Some('6'),
            (KeyCode::Numpad6, true) => None,
            (KeyCode::Numpad7, false) => Some('7'),
            (KeyCode::Numpad7, true) => None,
            (KeyCode::Numpad8, false) => Some('8'),
            (KeyCode::Numpad8, true) => None,
            (KeyCode::Numpad9, false) => Some('9'),
            (KeyCode::Numpad9, true) => None,
            (KeyCode::AbntC1, _) => None,
            (KeyCode::AbntC2, _) => None,
            (KeyCode::NumpadAdd, _) => None,
            (KeyCode::Apostrophe, _) => None,
            (KeyCode::Apps, _) => None,
            (KeyCode::Asterisk, _) => Some('*'),
            (KeyCode::Plus, false) => Some('+'),
            (KeyCode::Plus, true) => Some('='),
            (KeyCode::At, _) => None,
            (KeyCode::Ax, _) => None,
            (KeyCode::Backslash, false) => Some('\\'),
            (KeyCode::Backslash, true) => Some('|'),
            (KeyCode::Calculator, _) => None,
            (KeyCode::Capital, _) => None,
            (KeyCode::Colon, false) => Some(';'),
            (KeyCode::Colon, true) => Some(':'),
            (KeyCode::Comma, false) => Some(','),
            (KeyCode::Comma, true) => Some('<'),
            (KeyCode::Convert, _) => None,
            (KeyCode::NumpadDecimal, false) => Some('.'),
            (KeyCode::NumpadDecimal, true) => None,
            (KeyCode::NumpadDivide, false) => Some('/'),
            (KeyCode::NumpadDivide, true) => None,
            (KeyCode::Equals, false) => Some('='),
            (KeyCode::Equals, true) => Some('+'),
            (KeyCode::Grave, false) => Some('`'),
            (KeyCode::Grave, true) => Some('~'),
            (KeyCode::Kana, _) => None,
            (KeyCode::Kanji, _) => None,
            (KeyCode::AltLeft, _) => None,
            (KeyCode::BracketLeft, false) => Some('['),
            (KeyCode::BracketLeft, true) => Some('{'),
            (KeyCode::ControlLeft, _) => None,
            (KeyCode::ShiftLeft, _) => None,
            (KeyCode::SuperLeft, _) => None,
            (KeyCode::Mail, _) => None,
            (KeyCode::MediaSelect, _) => None,
            (KeyCode::MediaStop, _) => None,
            (KeyCode::Minus, false) => Some('-'),
            (KeyCode::Minus, true) => Some('_'),
            (KeyCode::NumpadMultiply, false) => Some('*'),
            (KeyCode::NumpadMultiply, true) => None,
            (KeyCode::Mute, _) => None,
            (KeyCode::MyComputer, _) => None,
            (KeyCode::NavigateForward, _) => None,
            (KeyCode::NavigateBackward, _) => None,
            (KeyCode::NextTrack, _) => None,
            (KeyCode::NoConvert, _) => None,
            (KeyCode::NumpadComma, false) => Some(','),
            (KeyCode::NumpadComma, true) => None,
            (KeyCode::NumpadEnter, false) => Some('\n'),
            (KeyCode::NumpadEnter, true) => None,
            (KeyCode::NumpadEquals, false) => Some('='),
            (KeyCode::NumpadEquals, true) => None,
            (KeyCode::Oem102, _) => None,
            (KeyCode::Period, false) => Some('.'),
            (KeyCode::Period, true) => Some('>'),
            (KeyCode::PlayPause, _) => None,
            (KeyCode::Power, _) => None,
            (KeyCode::PrevTrack, _) => None,
            (KeyCode::AltRight, _) => None,
            (KeyCode::BracketRight, false) => Some(']'),
            (KeyCode::BracketRight, true) => Some('}'),
            (KeyCode::ControlRight, _) => None,
            (KeyCode::ShiftRight, _) => None,
            (KeyCode::SuperRight, _) => None,
            (KeyCode::Semicolon, false) => Some(';'),
            (KeyCode::Semicolon, true) => Some(':'),
            (KeyCode::Slash, false) => Some('/'),
            (KeyCode::Slash, true) => Some('?'),
            (KeyCode::Sleep, _) => None,
            (KeyCode::Stop, _) => None,
            (KeyCode::NumpadSubtract, false) => Some('-'),
            (KeyCode::NumpadSubtract, true) => None,
            (KeyCode::Sysrq, _) => None,
            (KeyCode::Tab, false) => Some('\t'),
            (KeyCode::Tab, true) => None,
            (KeyCode::Underline, _) => None,
            (KeyCode::Unlabeled, _) => None,
            (KeyCode::VolumeDown, _) => None,
            (KeyCode::VolumeUp, _) => None,
            (KeyCode::Wake, _) => None,
            (KeyCode::WebBack, _) => None,
            (KeyCode::WebFavorites, _) => None,
            (KeyCode::WebForward, _) => None,
            (KeyCode::WebHome, _) => None,
            (KeyCode::WebRefresh, _) => None,
            (KeyCode::WebSearch, _) => None,
            (KeyCode::WebStop, _) => None,
            (KeyCode::Yen, _) => None,
            (KeyCode::Copy, _) => None,
            (KeyCode::Paste, _) => None,
            (KeyCode::Cut, _) => None,
        }
    }
}

/// The scan code of a [`KeyboardInput`].
///
/// ## Usage
///
/// It is used as the generic `<T>` value of an [`Input`] to create a `Res<Input<ScanCode>>`.
/// The resource values are mapped to the physical location of a key on the keyboard and correlate to an [`KeyCode`]
///
/// ## Updating
///
/// The resource is updated inside of the [`keyboard_input_system`].
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Reflect)]
#[reflect(Debug, Hash, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct ScanCode(pub u32);
