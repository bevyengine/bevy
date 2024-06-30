#![allow(deprecated)]
#![allow(missing_docs)]

use bevy_ecs::prelude::*;
use bevy_input::keyboard::KeyboardInput;
use bevy_input::touch::TouchInput;
use bevy_input::{
    gestures::*,
    keyboard::KeyboardFocusLost,
    mouse::{MouseButtonInput, MouseMotion, MouseWheel},
};
use bevy_reflect::Reflect;
#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};
use bevy_window::{
    AppLifecycle, CursorEntered, CursorLeft, CursorMoved, FileDragAndDrop, Ime, ReceivedCharacter,
    RequestRedraw, WindowBackendScaleFactorChanged, WindowCloseRequested, WindowCreated,
    WindowDestroyed, WindowFocused, WindowMoved, WindowOccluded, WindowResized,
    WindowScaleFactorChanged, WindowThemeChanged,
};

/// Wraps all `bevy_window` events in a common enum.
///
/// Read these events with `EventReader<WinitEvent>` if you need to
/// access window events in the order they were received from `winit`.
/// Otherwise, the event types are individually readable with
/// `EventReader<E>` (e.g. `EventReader<KeyboardInput>`).
#[derive(Event, Debug, Clone, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub enum WinitEvent {
    AppLifecycle(AppLifecycle),
    CursorEntered(CursorEntered),
    CursorLeft(CursorLeft),
    CursorMoved(CursorMoved),
    FileDragAndDrop(FileDragAndDrop),
    Ime(Ime),
    ReceivedCharacter(ReceivedCharacter),
    RequestRedraw(RequestRedraw),
    WindowBackendScaleFactorChanged(WindowBackendScaleFactorChanged),
    WindowCloseRequested(WindowCloseRequested),
    WindowCreated(WindowCreated),
    WindowDestroyed(WindowDestroyed),
    WindowFocused(WindowFocused),
    WindowMoved(WindowMoved),
    WindowOccluded(WindowOccluded),
    WindowResized(WindowResized),
    WindowScaleFactorChanged(WindowScaleFactorChanged),
    WindowThemeChanged(WindowThemeChanged),

    MouseButtonInput(MouseButtonInput),
    MouseMotion(MouseMotion),
    MouseWheel(MouseWheel),

    PinchGesture(PinchGesture),
    RotationGesture(RotationGesture),
    DoubleTapGesture(DoubleTapGesture),
    PanGesture(PanGesture),

    TouchInput(TouchInput),

    KeyboardInput(KeyboardInput),
    KeyboardFocusLost(KeyboardFocusLost),
}

impl From<AppLifecycle> for WinitEvent {
    fn from(e: AppLifecycle) -> Self {
        Self::AppLifecycle(e)
    }
}
impl From<CursorEntered> for WinitEvent {
    fn from(e: CursorEntered) -> Self {
        Self::CursorEntered(e)
    }
}
impl From<CursorLeft> for WinitEvent {
    fn from(e: CursorLeft) -> Self {
        Self::CursorLeft(e)
    }
}
impl From<CursorMoved> for WinitEvent {
    fn from(e: CursorMoved) -> Self {
        Self::CursorMoved(e)
    }
}
impl From<FileDragAndDrop> for WinitEvent {
    fn from(e: FileDragAndDrop) -> Self {
        Self::FileDragAndDrop(e)
    }
}
impl From<Ime> for WinitEvent {
    fn from(e: Ime) -> Self {
        Self::Ime(e)
    }
}
impl From<ReceivedCharacter> for WinitEvent {
    fn from(e: ReceivedCharacter) -> Self {
        Self::ReceivedCharacter(e)
    }
}
impl From<RequestRedraw> for WinitEvent {
    fn from(e: RequestRedraw) -> Self {
        Self::RequestRedraw(e)
    }
}
impl From<WindowBackendScaleFactorChanged> for WinitEvent {
    fn from(e: WindowBackendScaleFactorChanged) -> Self {
        Self::WindowBackendScaleFactorChanged(e)
    }
}
impl From<WindowCloseRequested> for WinitEvent {
    fn from(e: WindowCloseRequested) -> Self {
        Self::WindowCloseRequested(e)
    }
}
impl From<WindowCreated> for WinitEvent {
    fn from(e: WindowCreated) -> Self {
        Self::WindowCreated(e)
    }
}
impl From<WindowDestroyed> for WinitEvent {
    fn from(e: WindowDestroyed) -> Self {
        Self::WindowDestroyed(e)
    }
}
impl From<WindowFocused> for WinitEvent {
    fn from(e: WindowFocused) -> Self {
        Self::WindowFocused(e)
    }
}
impl From<WindowMoved> for WinitEvent {
    fn from(e: WindowMoved) -> Self {
        Self::WindowMoved(e)
    }
}
impl From<WindowOccluded> for WinitEvent {
    fn from(e: WindowOccluded) -> Self {
        Self::WindowOccluded(e)
    }
}
impl From<WindowResized> for WinitEvent {
    fn from(e: WindowResized) -> Self {
        Self::WindowResized(e)
    }
}
impl From<WindowScaleFactorChanged> for WinitEvent {
    fn from(e: WindowScaleFactorChanged) -> Self {
        Self::WindowScaleFactorChanged(e)
    }
}
impl From<WindowThemeChanged> for WinitEvent {
    fn from(e: WindowThemeChanged) -> Self {
        Self::WindowThemeChanged(e)
    }
}
impl From<MouseButtonInput> for WinitEvent {
    fn from(e: MouseButtonInput) -> Self {
        Self::MouseButtonInput(e)
    }
}
impl From<MouseMotion> for WinitEvent {
    fn from(e: MouseMotion) -> Self {
        Self::MouseMotion(e)
    }
}
impl From<MouseWheel> for WinitEvent {
    fn from(e: MouseWheel) -> Self {
        Self::MouseWheel(e)
    }
}
impl From<PinchGesture> for WinitEvent {
    fn from(e: PinchGesture) -> Self {
        Self::PinchGesture(e)
    }
}
impl From<RotationGesture> for WinitEvent {
    fn from(e: RotationGesture) -> Self {
        Self::RotationGesture(e)
    }
}
impl From<DoubleTapGesture> for WinitEvent {
    fn from(e: DoubleTapGesture) -> Self {
        Self::DoubleTapGesture(e)
    }
}
impl From<PanGesture> for WinitEvent {
    fn from(e: PanGesture) -> Self {
        Self::PanGesture(e)
    }
}
impl From<TouchInput> for WinitEvent {
    fn from(e: TouchInput) -> Self {
        Self::TouchInput(e)
    }
}
impl From<KeyboardInput> for WinitEvent {
    fn from(e: KeyboardInput) -> Self {
        Self::KeyboardInput(e)
    }
}
impl From<KeyboardFocusLost> for WinitEvent {
    fn from(e: KeyboardFocusLost) -> Self {
        Self::KeyboardFocusLost(e)
    }
}
