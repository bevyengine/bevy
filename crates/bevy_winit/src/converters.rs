use bevy_ecs::entity::Entity;
use bevy_input::{
    keyboard::{KeyCode, KeyboardInput},
    mouse::MouseButton,
    touch::{ForceTouch, TouchInput, TouchPhase},
    ButtonState,
};
use bevy_math::Vec2;
use bevy_window::{CursorIcon, EnabledButtons, WindowLevel, WindowTheme};

pub fn convert_keyboard_input(
    keyboard_input: &winit::event::KeyboardInput,
    window: Entity,
) -> KeyboardInput {
    KeyboardInput {
        scan_code: keyboard_input.scancode,
        state: convert_element_state(keyboard_input.state),
        key_code: keyboard_input.virtual_keycode.map(convert_virtual_key_code),
        window,
    }
}

pub fn convert_element_state(element_state: winit::event::ElementState) -> ButtonState {
    match element_state {
        winit::event::ElementState::Pressed => ButtonState::Pressed,
        winit::event::ElementState::Released => ButtonState::Released,
    }
}

pub fn convert_mouse_button(mouse_button: winit::event::MouseButton) -> MouseButton {
    match mouse_button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Other(val) => MouseButton::Other(val),
    }
}

pub fn convert_touch_input(
    touch_input: winit::event::Touch,
    location: winit::dpi::LogicalPosition<f64>,
) -> TouchInput {
    TouchInput {
        phase: match touch_input.phase {
            winit::event::TouchPhase::Started => TouchPhase::Started,
            winit::event::TouchPhase::Moved => TouchPhase::Moved,
            winit::event::TouchPhase::Ended => TouchPhase::Ended,
            winit::event::TouchPhase::Cancelled => TouchPhase::Canceled,
        },
        position: Vec2::new(location.x as f32, location.y as f32),
        force: touch_input.force.map(|f| match f {
            winit::event::Force::Calibrated {
                force,
                max_possible_force,
                altitude_angle,
            } => ForceTouch::Calibrated {
                force,
                max_possible_force,
                altitude_angle,
            },
            winit::event::Force::Normalized(x) => ForceTouch::Normalized(x),
        }),
        id: touch_input.id,
    }
}

pub fn convert_virtual_key_code(virtual_key_code: winit::event::VirtualKeyCode) -> KeyCode {
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
        winit::event::VirtualKeyCode::LAlt => KeyCode::AltLeft,
        winit::event::VirtualKeyCode::LBracket => KeyCode::BracketLeft,
        winit::event::VirtualKeyCode::LControl => KeyCode::ControlLeft,
        winit::event::VirtualKeyCode::LShift => KeyCode::ShiftLeft,
        winit::event::VirtualKeyCode::LWin => KeyCode::SuperLeft,
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
        winit::event::VirtualKeyCode::RAlt => KeyCode::AltRight,
        winit::event::VirtualKeyCode::RBracket => KeyCode::BracketRight,
        winit::event::VirtualKeyCode::RControl => KeyCode::ControlRight,
        winit::event::VirtualKeyCode::RShift => KeyCode::ShiftRight,
        winit::event::VirtualKeyCode::RWin => KeyCode::SuperRight,
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

pub fn convert_cursor_icon(cursor_icon: CursorIcon) -> winit::window::CursorIcon {
    match cursor_icon {
        CursorIcon::Default => winit::window::CursorIcon::Default,
        CursorIcon::Crosshair => winit::window::CursorIcon::Crosshair,
        CursorIcon::Hand => winit::window::CursorIcon::Hand,
        CursorIcon::Arrow => winit::window::CursorIcon::Arrow,
        CursorIcon::Move => winit::window::CursorIcon::Move,
        CursorIcon::Text => winit::window::CursorIcon::Text,
        CursorIcon::Wait => winit::window::CursorIcon::Wait,
        CursorIcon::Help => winit::window::CursorIcon::Help,
        CursorIcon::Progress => winit::window::CursorIcon::Progress,
        CursorIcon::NotAllowed => winit::window::CursorIcon::NotAllowed,
        CursorIcon::ContextMenu => winit::window::CursorIcon::ContextMenu,
        CursorIcon::Cell => winit::window::CursorIcon::Cell,
        CursorIcon::VerticalText => winit::window::CursorIcon::VerticalText,
        CursorIcon::Alias => winit::window::CursorIcon::Alias,
        CursorIcon::Copy => winit::window::CursorIcon::Copy,
        CursorIcon::NoDrop => winit::window::CursorIcon::NoDrop,
        CursorIcon::Grab => winit::window::CursorIcon::Grab,
        CursorIcon::Grabbing => winit::window::CursorIcon::Grabbing,
        CursorIcon::AllScroll => winit::window::CursorIcon::AllScroll,
        CursorIcon::ZoomIn => winit::window::CursorIcon::ZoomIn,
        CursorIcon::ZoomOut => winit::window::CursorIcon::ZoomOut,
        CursorIcon::EResize => winit::window::CursorIcon::EResize,
        CursorIcon::NResize => winit::window::CursorIcon::NResize,
        CursorIcon::NeResize => winit::window::CursorIcon::NeResize,
        CursorIcon::NwResize => winit::window::CursorIcon::NwResize,
        CursorIcon::SResize => winit::window::CursorIcon::SResize,
        CursorIcon::SeResize => winit::window::CursorIcon::SeResize,
        CursorIcon::SwResize => winit::window::CursorIcon::SwResize,
        CursorIcon::WResize => winit::window::CursorIcon::WResize,
        CursorIcon::EwResize => winit::window::CursorIcon::EwResize,
        CursorIcon::NsResize => winit::window::CursorIcon::NsResize,
        CursorIcon::NeswResize => winit::window::CursorIcon::NeswResize,
        CursorIcon::NwseResize => winit::window::CursorIcon::NwseResize,
        CursorIcon::ColResize => winit::window::CursorIcon::ColResize,
        CursorIcon::RowResize => winit::window::CursorIcon::RowResize,
    }
}

pub fn convert_window_level(window_level: WindowLevel) -> winit::window::WindowLevel {
    match window_level {
        WindowLevel::AlwaysOnBottom => winit::window::WindowLevel::AlwaysOnBottom,
        WindowLevel::Normal => winit::window::WindowLevel::Normal,
        WindowLevel::AlwaysOnTop => winit::window::WindowLevel::AlwaysOnTop,
    }
}

pub fn convert_winit_theme(theme: winit::window::Theme) -> WindowTheme {
    match theme {
        winit::window::Theme::Light => WindowTheme::Light,
        winit::window::Theme::Dark => WindowTheme::Dark,
    }
}

pub fn convert_window_theme(theme: WindowTheme) -> winit::window::Theme {
    match theme {
        WindowTheme::Light => winit::window::Theme::Light,
        WindowTheme::Dark => winit::window::Theme::Dark,
    }
}

pub fn convert_enabled_buttons(enabled_buttons: EnabledButtons) -> winit::window::WindowButtons {
    let mut window_buttons = winit::window::WindowButtons::empty();
    if enabled_buttons.minimize {
        window_buttons.insert(winit::window::WindowButtons::MINIMIZE);
    }
    if enabled_buttons.maximize {
        window_buttons.insert(winit::window::WindowButtons::MAXIMIZE);
    }
    if enabled_buttons.close {
        window_buttons.insert(winit::window::WindowButtons::CLOSE);
    }
    window_buttons
}

use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{
    AxisId, DeviceEvent, DeviceId, ElementState, Ime, ModifiersState, StartCause, Touch,
};
use winit::window::{Theme, WindowId};

use std::path::PathBuf;

// TODO: can remove all these types when we upgrade to winit 0.29
#[derive(Debug, PartialEq)]
#[allow(clippy::enum_variant_names)]
pub(crate) enum Event<T> {
    NewEvents(StartCause),
    WindowEvent {
        window_id: WindowId,
        event: WindowEvent,
    },
    DeviceEvent {
        device_id: DeviceId,
        event: DeviceEvent,
    },
    UserEvent(T),
    Suspended,
    Resumed,
    MainEventsCleared,
    RedrawRequested(WindowId),
    RedrawEventsCleared,
    LoopDestroyed,
}

#[derive(Debug, PartialEq)]
pub(crate) enum WindowEvent {
    Resized(PhysicalSize<u32>),
    Moved(PhysicalPosition<i32>),
    CloseRequested,
    Destroyed,
    DroppedFile(PathBuf),
    HoveredFile(PathBuf),
    HoveredFileCancelled,
    ReceivedCharacter(char),
    Focused(bool),
    KeyboardInput {
        device_id: DeviceId,
        input: winit::event::KeyboardInput,
        is_synthetic: bool,
    },
    ModifiersChanged(ModifiersState),
    Ime(Ime),
    CursorMoved {
        device_id: DeviceId,
        position: PhysicalPosition<f64>,
    },

    CursorEntered {
        device_id: DeviceId,
    },
    CursorLeft {
        device_id: DeviceId,
    },
    MouseWheel {
        device_id: DeviceId,
        delta: winit::event::MouseScrollDelta,
        phase: winit::event::TouchPhase,
    },
    MouseInput {
        device_id: DeviceId,
        state: ElementState,
        button: winit::event::MouseButton,
    },
    TouchpadMagnify {
        device_id: DeviceId,
        delta: f64,
        phase: winit::event::TouchPhase,
    },
    SmartMagnify {
        device_id: DeviceId,
    },
    TouchpadRotate {
        device_id: DeviceId,
        delta: f32,
        phase: winit::event::TouchPhase,
    },
    TouchpadPressure {
        device_id: DeviceId,
        pressure: f32,
        stage: i64,
    },
    AxisMotion {
        device_id: DeviceId,
        axis: AxisId,
        value: f64,
    },
    Touch(Touch),
    ScaleFactorChanged {
        scale_factor: f64,
        new_inner_size: PhysicalSize<u32>,
    },
    ThemeChanged(Theme),
    Occluded(bool),
}

pub(crate) fn convert_event<T>(event: winit::event::Event<'_, T>) -> Event<T> {
    match event {
        winit::event::Event::NewEvents(start_cause) => Event::NewEvents(start_cause),
        winit::event::Event::WindowEvent { window_id, event } => Event::WindowEvent {
            window_id,
            event: convert_window_event(event),
        },
        winit::event::Event::DeviceEvent { device_id, event } => {
            Event::DeviceEvent { device_id, event }
        }
        winit::event::Event::UserEvent(value) => Event::UserEvent(value),
        winit::event::Event::Suspended => Event::Suspended,
        winit::event::Event::Resumed => Event::Resumed,
        winit::event::Event::MainEventsCleared => Event::MainEventsCleared,
        winit::event::Event::RedrawRequested(window_id) => Event::RedrawRequested(window_id),
        winit::event::Event::RedrawEventsCleared => Event::RedrawEventsCleared,
        winit::event::Event::LoopDestroyed => Event::LoopDestroyed,
    }
}

pub(crate) fn convert_window_event(event: winit::event::WindowEvent<'_>) -> WindowEvent {
    match event {
        winit::event::WindowEvent::AxisMotion {
            device_id,
            axis,
            value,
        } => WindowEvent::AxisMotion {
            device_id,
            axis,
            value,
        },
        winit::event::WindowEvent::CloseRequested => WindowEvent::CloseRequested,
        winit::event::WindowEvent::CursorEntered { device_id } => {
            WindowEvent::CursorEntered { device_id }
        }
        winit::event::WindowEvent::CursorLeft { device_id } => {
            WindowEvent::CursorLeft { device_id }
        }
        winit::event::WindowEvent::CursorMoved {
            device_id,
            position,
            ..
        } => WindowEvent::CursorMoved {
            device_id,
            position,
        },
        winit::event::WindowEvent::Destroyed => WindowEvent::Destroyed,
        winit::event::WindowEvent::DroppedFile(path_buf) => WindowEvent::DroppedFile(path_buf),
        winit::event::WindowEvent::Focused(b) => WindowEvent::Focused(b),
        winit::event::WindowEvent::HoveredFile(path_buf) => WindowEvent::HoveredFile(path_buf),
        winit::event::WindowEvent::HoveredFileCancelled => WindowEvent::HoveredFileCancelled,
        winit::event::WindowEvent::Ime(ime) => WindowEvent::Ime(ime),
        winit::event::WindowEvent::KeyboardInput {
            device_id,
            input,
            is_synthetic,
        } => WindowEvent::KeyboardInput {
            device_id,
            input,
            is_synthetic,
        },
        winit::event::WindowEvent::ModifiersChanged(modifiers_state) => {
            WindowEvent::ModifiersChanged(modifiers_state)
        }
        winit::event::WindowEvent::MouseInput {
            device_id,
            state,
            button,
            ..
        } => WindowEvent::MouseInput {
            device_id,
            state,
            button,
        },
        winit::event::WindowEvent::MouseWheel {
            device_id,
            delta,
            phase,
            ..
        } => WindowEvent::MouseWheel {
            device_id,
            delta,
            phase,
        },
        winit::event::WindowEvent::Moved(new_position) => WindowEvent::Moved(new_position),
        winit::event::WindowEvent::Occluded(b) => WindowEvent::Occluded(b),
        winit::event::WindowEvent::ReceivedCharacter(char) => WindowEvent::ReceivedCharacter(char),
        winit::event::WindowEvent::Resized(new_size) => WindowEvent::Resized(new_size),
        winit::event::WindowEvent::ScaleFactorChanged {
            scale_factor,
            new_inner_size,
        } => WindowEvent::ScaleFactorChanged {
            scale_factor,
            new_inner_size: *new_inner_size,
        },
        winit::event::WindowEvent::SmartMagnify { device_id } => {
            WindowEvent::SmartMagnify { device_id }
        }
        winit::event::WindowEvent::ThemeChanged(theme) => WindowEvent::ThemeChanged(theme),
        winit::event::WindowEvent::Touch(touch) => WindowEvent::Touch(touch),
        winit::event::WindowEvent::TouchpadMagnify {
            device_id,
            delta,
            phase,
        } => WindowEvent::TouchpadMagnify {
            device_id,
            delta,
            phase,
        },
        winit::event::WindowEvent::TouchpadPressure {
            device_id,
            pressure,
            stage,
        } => WindowEvent::TouchpadPressure {
            device_id,
            pressure,
            stage,
        },
        winit::event::WindowEvent::TouchpadRotate {
            device_id,
            delta,
            phase,
        } => WindowEvent::TouchpadRotate {
            device_id,
            delta,
            phase,
        },
    }
}
