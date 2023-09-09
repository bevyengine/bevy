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
/// The event is consumed inside of the [`keyboard_input_system`](crate::keyboard::keyboard_input_system)
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

/// The scan code of a [`KeyboardInput`](crate::keyboard::KeyboardInput).
///
/// ## Usage
///
/// It is used as the generic `<T>` value of an [`Input`](crate::Input) to create a `Res<Input<ScanCode>>`.
/// The resource values are mapped to the physical location of a key on the keyboard and correlate to an [`KeyCode`](KeyCode)
///
/// ## Updating
///
/// The resource is updated inside of the [`keyboard_input_system`](crate::keyboard::keyboard_input_system).
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Reflect)]
#[reflect(Debug, Hash, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct ScanCode(pub u32);
