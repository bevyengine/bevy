use crate::{ButtonState, Input};
use bevy_ecs::{event::EventReader, system::ResMut};
use bevy_reflect::{FromReflect, Reflect};

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A keyboard input event.
///
/// This event is the translated version of the `WindowEvent::KeyboardInput` from the `winit` crate.
/// It is available to the end user and can be used for game logic.
///
/// ## Usage
///
/// The event is consumed inside of the [`keyboard_input_system`](crate::keyboard::keyboard_input_system)
/// to update the [`Input<KeyCode>`](crate::Input<KeyCode>) resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Reflect, FromReflect)]
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
}

impl From<&winit::event::KeyboardInput> for KeyboardInput {
    fn from(keyboard_input: &winit::event::KeyboardInput) -> Self {
        KeyboardInput {
            scan_code: keyboard_input.scancode,
            state: keyboard_input.state.into(),
            key_code: keyboard_input.virtual_keycode.map(KeyCode::from),
        }
    }
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
    scan_input.clear();
    key_input.clear();
    for event in keyboard_input_events.iter() {
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

/// The key code of a [`KeyboardInput`](crate::keyboard::KeyboardInput).
///
/// ## Usage
///
/// It is used as the generic `T` value of an [`Input`](crate::Input) to create a `Res<Input<KeyCode>>`.
/// The resource values are mapped to the current layout of the keyboard and correlate to an [`ScanCode`](ScanCode).
///
/// ## Updating
///
/// The resource is updated inside of the [`keyboard_input_system`](crate::keyboard::keyboard_input_system).
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Reflect, FromReflect)]
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
    /// The `Asterik` / `*` key.
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

    /// The `LAlt` / `Left Alt` key. Maps to `Left Option` on Mac.
    LAlt,
    /// The `LBracket` / `Left Bracket` key.
    LBracket,
    /// The `LControl` / `Left Control` key.
    LControl,
    /// The `LShift` / `Left Shift` key.
    LShift,
    /// The `LWin` / `Left Windows` key. Maps to `Left Command` on Mac.
    LWin,

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

    /// The `RAlt` / `Right Alt` key. Maps to `Right Option` on Mac.
    RAlt,
    /// The `RBracket` / `Right Bracket` key.
    RBracket,
    /// The `RControl` / `Right Control` key.
    RControl,
    /// The `RShift` / `Right Shift` key.
    RShift,
    /// The `RWin` / `Right Windows` key. Maps to `Right Command` on Mac.
    RWin,

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

impl From<winit::event::VirtualKeyCode> for KeyCode {
    fn from(virtual_key_code: winit::event::VirtualKeyCode) -> Self {
        match virtual_key_code {
            winit::event::VirtualKeyCode::Key1 => KeyCode::Key1,
            winit::event::VirtualKeyCode::Key2 => KeyCode::Key2,
            winit::event::VirtualKeyCode::Key3 => KeyCode::Key3,
            winit::event::VirtualKeyCode::Key4 => KeyCode::Key4,
            winit::event::VirtualKeyCode::Key5 => KeyCode::Key5,
            winit::event::VirtualKeyCode::Key6 => KeyCode::Key6,
            winit::event::VirtualKeyCode::Key7 => KeyCode::Key7,
            winit::event::VirtualKeyCode::Key8 => KeyCode::Key8,
            winit::event::VirtualKeyCode::Key9 => KeyCode::Key9,
            winit::event::VirtualKeyCode::Key0 => KeyCode::Key0,
            winit::event::VirtualKeyCode::A => KeyCode::A,
            winit::event::VirtualKeyCode::B => KeyCode::B,
            winit::event::VirtualKeyCode::C => KeyCode::C,
            winit::event::VirtualKeyCode::D => KeyCode::D,
            winit::event::VirtualKeyCode::E => KeyCode::E,
            winit::event::VirtualKeyCode::F => KeyCode::F,
            winit::event::VirtualKeyCode::G => KeyCode::G,
            winit::event::VirtualKeyCode::H => KeyCode::H,
            winit::event::VirtualKeyCode::I => KeyCode::I,
            winit::event::VirtualKeyCode::J => KeyCode::J,
            winit::event::VirtualKeyCode::K => KeyCode::K,
            winit::event::VirtualKeyCode::L => KeyCode::L,
            winit::event::VirtualKeyCode::M => KeyCode::M,
            winit::event::VirtualKeyCode::N => KeyCode::N,
            winit::event::VirtualKeyCode::O => KeyCode::O,
            winit::event::VirtualKeyCode::P => KeyCode::P,
            winit::event::VirtualKeyCode::Q => KeyCode::Q,
            winit::event::VirtualKeyCode::R => KeyCode::R,
            winit::event::VirtualKeyCode::S => KeyCode::S,
            winit::event::VirtualKeyCode::T => KeyCode::T,
            winit::event::VirtualKeyCode::U => KeyCode::U,
            winit::event::VirtualKeyCode::V => KeyCode::V,
            winit::event::VirtualKeyCode::W => KeyCode::W,
            winit::event::VirtualKeyCode::X => KeyCode::X,
            winit::event::VirtualKeyCode::Y => KeyCode::Y,
            winit::event::VirtualKeyCode::Z => KeyCode::Z,
            winit::event::VirtualKeyCode::Escape => KeyCode::Escape,
            winit::event::VirtualKeyCode::F1 => KeyCode::F1,
            winit::event::VirtualKeyCode::F2 => KeyCode::F2,
            winit::event::VirtualKeyCode::F3 => KeyCode::F3,
            winit::event::VirtualKeyCode::F4 => KeyCode::F4,
            winit::event::VirtualKeyCode::F5 => KeyCode::F5,
            winit::event::VirtualKeyCode::F6 => KeyCode::F6,
            winit::event::VirtualKeyCode::F7 => KeyCode::F7,
            winit::event::VirtualKeyCode::F8 => KeyCode::F8,
            winit::event::VirtualKeyCode::F9 => KeyCode::F9,
            winit::event::VirtualKeyCode::F10 => KeyCode::F10,
            winit::event::VirtualKeyCode::F11 => KeyCode::F11,
            winit::event::VirtualKeyCode::F12 => KeyCode::F12,
            winit::event::VirtualKeyCode::F13 => KeyCode::F13,
            winit::event::VirtualKeyCode::F14 => KeyCode::F14,
            winit::event::VirtualKeyCode::F15 => KeyCode::F15,
            winit::event::VirtualKeyCode::F16 => KeyCode::F16,
            winit::event::VirtualKeyCode::F17 => KeyCode::F17,
            winit::event::VirtualKeyCode::F18 => KeyCode::F18,
            winit::event::VirtualKeyCode::F19 => KeyCode::F19,
            winit::event::VirtualKeyCode::F20 => KeyCode::F20,
            winit::event::VirtualKeyCode::F21 => KeyCode::F21,
            winit::event::VirtualKeyCode::F22 => KeyCode::F22,
            winit::event::VirtualKeyCode::F23 => KeyCode::F23,
            winit::event::VirtualKeyCode::F24 => KeyCode::F24,
            winit::event::VirtualKeyCode::Snapshot => KeyCode::Snapshot,
            winit::event::VirtualKeyCode::Scroll => KeyCode::Scroll,
            winit::event::VirtualKeyCode::Pause => KeyCode::Pause,
            winit::event::VirtualKeyCode::Insert => KeyCode::Insert,
            winit::event::VirtualKeyCode::Home => KeyCode::Home,
            winit::event::VirtualKeyCode::Delete => KeyCode::Delete,
            winit::event::VirtualKeyCode::End => KeyCode::End,
            winit::event::VirtualKeyCode::PageDown => KeyCode::PageDown,
            winit::event::VirtualKeyCode::PageUp => KeyCode::PageUp,
            winit::event::VirtualKeyCode::Left => KeyCode::Left,
            winit::event::VirtualKeyCode::Up => KeyCode::Up,
            winit::event::VirtualKeyCode::Right => KeyCode::Right,
            winit::event::VirtualKeyCode::Down => KeyCode::Down,
            winit::event::VirtualKeyCode::Back => KeyCode::Back,
            winit::event::VirtualKeyCode::Return => KeyCode::Return,
            winit::event::VirtualKeyCode::Space => KeyCode::Space,
            winit::event::VirtualKeyCode::Compose => KeyCode::Compose,
            winit::event::VirtualKeyCode::Caret => KeyCode::Caret,
            winit::event::VirtualKeyCode::Numlock => KeyCode::Numlock,
            winit::event::VirtualKeyCode::Numpad0 => KeyCode::Numpad0,
            winit::event::VirtualKeyCode::Numpad1 => KeyCode::Numpad1,
            winit::event::VirtualKeyCode::Numpad2 => KeyCode::Numpad2,
            winit::event::VirtualKeyCode::Numpad3 => KeyCode::Numpad3,
            winit::event::VirtualKeyCode::Numpad4 => KeyCode::Numpad4,
            winit::event::VirtualKeyCode::Numpad5 => KeyCode::Numpad5,
            winit::event::VirtualKeyCode::Numpad6 => KeyCode::Numpad6,
            winit::event::VirtualKeyCode::Numpad7 => KeyCode::Numpad7,
            winit::event::VirtualKeyCode::Numpad8 => KeyCode::Numpad8,
            winit::event::VirtualKeyCode::Numpad9 => KeyCode::Numpad9,
            winit::event::VirtualKeyCode::AbntC1 => KeyCode::AbntC1,
            winit::event::VirtualKeyCode::AbntC2 => KeyCode::AbntC2,
            winit::event::VirtualKeyCode::NumpadAdd => KeyCode::NumpadAdd,
            winit::event::VirtualKeyCode::Apostrophe => KeyCode::Apostrophe,
            winit::event::VirtualKeyCode::Apps => KeyCode::Apps,
            winit::event::VirtualKeyCode::Asterisk => KeyCode::Asterisk,
            winit::event::VirtualKeyCode::Plus => KeyCode::Plus,
            winit::event::VirtualKeyCode::At => KeyCode::At,
            winit::event::VirtualKeyCode::Ax => KeyCode::Ax,
            winit::event::VirtualKeyCode::Backslash => KeyCode::Backslash,
            winit::event::VirtualKeyCode::Calculator => KeyCode::Calculator,
            winit::event::VirtualKeyCode::Capital => KeyCode::Capital,
            winit::event::VirtualKeyCode::Colon => KeyCode::Colon,
            winit::event::VirtualKeyCode::Comma => KeyCode::Comma,
            winit::event::VirtualKeyCode::Convert => KeyCode::Convert,
            winit::event::VirtualKeyCode::NumpadDecimal => KeyCode::NumpadDecimal,
            winit::event::VirtualKeyCode::NumpadDivide => KeyCode::NumpadDivide,
            winit::event::VirtualKeyCode::Equals => KeyCode::Equals,
            winit::event::VirtualKeyCode::Grave => KeyCode::Grave,
            winit::event::VirtualKeyCode::Kana => KeyCode::Kana,
            winit::event::VirtualKeyCode::Kanji => KeyCode::Kanji,
            winit::event::VirtualKeyCode::LAlt => KeyCode::LAlt,
            winit::event::VirtualKeyCode::LBracket => KeyCode::LBracket,
            winit::event::VirtualKeyCode::LControl => KeyCode::LControl,
            winit::event::VirtualKeyCode::LShift => KeyCode::LShift,
            winit::event::VirtualKeyCode::LWin => KeyCode::LWin,
            winit::event::VirtualKeyCode::Mail => KeyCode::Mail,
            winit::event::VirtualKeyCode::MediaSelect => KeyCode::MediaSelect,
            winit::event::VirtualKeyCode::MediaStop => KeyCode::MediaStop,
            winit::event::VirtualKeyCode::Minus => KeyCode::Minus,
            winit::event::VirtualKeyCode::NumpadMultiply => KeyCode::NumpadMultiply,
            winit::event::VirtualKeyCode::Mute => KeyCode::Mute,
            winit::event::VirtualKeyCode::MyComputer => KeyCode::MyComputer,
            winit::event::VirtualKeyCode::NavigateForward => KeyCode::NavigateForward,
            winit::event::VirtualKeyCode::NavigateBackward => KeyCode::NavigateBackward,
            winit::event::VirtualKeyCode::NextTrack => KeyCode::NextTrack,
            winit::event::VirtualKeyCode::NoConvert => KeyCode::NoConvert,
            winit::event::VirtualKeyCode::NumpadComma => KeyCode::NumpadComma,
            winit::event::VirtualKeyCode::NumpadEnter => KeyCode::NumpadEnter,
            winit::event::VirtualKeyCode::NumpadEquals => KeyCode::NumpadEquals,
            winit::event::VirtualKeyCode::OEM102 => KeyCode::Oem102,
            winit::event::VirtualKeyCode::Period => KeyCode::Period,
            winit::event::VirtualKeyCode::PlayPause => KeyCode::PlayPause,
            winit::event::VirtualKeyCode::Power => KeyCode::Power,
            winit::event::VirtualKeyCode::PrevTrack => KeyCode::PrevTrack,
            winit::event::VirtualKeyCode::RAlt => KeyCode::RAlt,
            winit::event::VirtualKeyCode::RBracket => KeyCode::RBracket,
            winit::event::VirtualKeyCode::RControl => KeyCode::RControl,
            winit::event::VirtualKeyCode::RShift => KeyCode::RShift,
            winit::event::VirtualKeyCode::RWin => KeyCode::RWin,
            winit::event::VirtualKeyCode::Semicolon => KeyCode::Semicolon,
            winit::event::VirtualKeyCode::Slash => KeyCode::Slash,
            winit::event::VirtualKeyCode::Sleep => KeyCode::Sleep,
            winit::event::VirtualKeyCode::Stop => KeyCode::Stop,
            winit::event::VirtualKeyCode::NumpadSubtract => KeyCode::NumpadSubtract,
            winit::event::VirtualKeyCode::Sysrq => KeyCode::Sysrq,
            winit::event::VirtualKeyCode::Tab => KeyCode::Tab,
            winit::event::VirtualKeyCode::Underline => KeyCode::Underline,
            winit::event::VirtualKeyCode::Unlabeled => KeyCode::Unlabeled,
            winit::event::VirtualKeyCode::VolumeDown => KeyCode::VolumeDown,
            winit::event::VirtualKeyCode::VolumeUp => KeyCode::VolumeUp,
            winit::event::VirtualKeyCode::Wake => KeyCode::Wake,
            winit::event::VirtualKeyCode::WebBack => KeyCode::WebBack,
            winit::event::VirtualKeyCode::WebFavorites => KeyCode::WebFavorites,
            winit::event::VirtualKeyCode::WebForward => KeyCode::WebForward,
            winit::event::VirtualKeyCode::WebHome => KeyCode::WebHome,
            winit::event::VirtualKeyCode::WebRefresh => KeyCode::WebRefresh,
            winit::event::VirtualKeyCode::WebSearch => KeyCode::WebSearch,
            winit::event::VirtualKeyCode::WebStop => KeyCode::WebStop,
            winit::event::VirtualKeyCode::Yen => KeyCode::Yen,
            winit::event::VirtualKeyCode::Copy => KeyCode::Copy,
            winit::event::VirtualKeyCode::Paste => KeyCode::Paste,
            winit::event::VirtualKeyCode::Cut => KeyCode::Cut,
        }
    }
}

/// The scan code of a [`KeyboardInput`](crate::keyboard::KeyboardInput).
///
/// ## Usage
///
/// It is used as the generic <T> value of an [`Input`](crate::Input) to create a `Res<Input<ScanCode>>`.
/// The resource values are mapped to the physical location of a key on the keyboard and correlate to an [`KeyCode`](KeyCode)
///
/// ## Updating
///
/// The resource is updated inside of the [`keyboard_input_system`](crate::keyboard::keyboard_input_system).
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Reflect, FromReflect)]
#[reflect(Debug, Hash, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct ScanCode(pub u32);
