//! The mouse input functionality.

use crate::{ButtonInput, ButtonState};
use bevy_ecs::entity::Entity;
use bevy_ecs::{
    change_detection::DetectChangesMut,
    event::{Event, EventReader},
    system::ResMut,
};
use bevy_math::Vec2;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;

#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// A mouse button input event.
///
/// This event is the translated version of the `WindowEvent::MouseInput` from the `winit` crate.
///
/// ## Usage
///
/// The event is read inside of the [`mouse_button_input_system`]
/// to update the [`ButtonInput<MouseButton>`] resource.
#[derive(Event, Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct MouseButtonInput {
    /// The mouse button assigned to the event.
    pub button: MouseButton,
    /// The pressed state of the button.
    pub state: ButtonState,
    /// Window that received the input.
    pub window: Entity,
}

/// A button on a mouse device.
///
/// ## Usage
///
/// It is used as the generic `T` value of an [`ButtonInput`] to create a `bevy`
/// resource.
///
/// ## Updating
///
/// The resource is updated inside of the [`mouse_button_input_system`].
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum MouseButton {
    /// The left mouse button.
    Left,
    /// The right mouse button.
    Right,
    /// The middle mouse button.
    Middle,
    /// The back mouse button.
    Back,
    /// The forward mouse button.
    Forward,
    /// Another mouse button with the associated number.
    Other(u16),
}

/// An event reporting the change in physical position of a pointing device.
///
/// This represents raw, unfiltered physical motion.
/// It is the translated version of [`DeviceEvent::MouseMotion`] from the `winit` crate.
///
/// All pointing devices connected to a single machine at the same time can emit the event independently.
/// However, the event data does not make it possible to distinguish which device it is referring to.
///
/// [`DeviceEvent::MouseMotion`]: https://docs.rs/winit/latest/winit/event/enum.DeviceEvent.html#variant.MouseMotion
#[derive(Event, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct MouseMotion {
    /// The change in the position of the pointing device since the last event was sent.
    pub delta: Vec2,
}

/// The scroll unit.
///
/// Describes how a value of a [`MouseWheel`] event has to be interpreted.
///
/// The value of the event can either be interpreted as the amount of lines or the amount of pixels
/// to scroll.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum MouseScrollUnit {
    /// The line scroll unit.
    ///
    /// The delta of the associated [`MouseWheel`] event corresponds
    /// to the amount of lines or rows to scroll.
    Line,
    /// The pixel scroll unit.
    ///
    /// The delta of the associated [`MouseWheel`] event corresponds
    /// to the amount of pixels to scroll.
    Pixel,
}

/// A mouse wheel event.
///
/// This event is the translated version of the `WindowEvent::MouseWheel` from the `winit` crate.
#[derive(Event, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy_reflect", derive(Reflect), reflect(Debug, PartialEq))]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub struct MouseWheel {
    /// The mouse scroll unit.
    pub unit: MouseScrollUnit,
    /// The horizontal scroll value.
    pub x: f32,
    /// The vertical scroll value.
    pub y: f32,
    /// Window that received the input.
    pub window: Entity,
}

/// Updates the [`ButtonInput<MouseButton>`] resource with the latest [`MouseButtonInput`] events.
///
/// ## Differences
///
/// The main difference between the [`MouseButtonInput`] event and the [`ButtonInput<MouseButton>`] resource is that
/// the latter has convenient functions like [`ButtonInput::pressed`], [`ButtonInput::just_pressed`] and [`ButtonInput::just_released`].
pub fn mouse_button_input_system(
    mut mouse_button_input: ResMut<ButtonInput<MouseButton>>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
) {
    mouse_button_input.bypass_change_detection().clear();
    for event in mouse_button_input_events.read() {
        match event.state {
            ButtonState::Pressed => mouse_button_input.press(event.button),
            ButtonState::Released => mouse_button_input.release(event.button),
        }
    }
}
