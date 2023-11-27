//! The touchpad input functionality.

use bevy_ecs::event::Event;
use bevy_reflect::Reflect;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

use crate::InputSource;

/// Touchpad magnification event with two-finger pinch gesture.
///
/// Positive delta values indicate magnification (zooming in) and
/// negative delta values indicate shrinking (zooming out).
///
/// ## Platform-specific
///
/// - Only available on **`macOS`**.
#[derive(Event, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct TouchpadMagnify {
    /// The amount of magnification.
    pub delta: f32,
    /// The source of this input event.
    pub source: InputSource,
}

/// Touchpad rotation event with two-finger rotation gesture.
///
/// Positive delta values indicate rotation counterclockwise and
/// negative delta values indicate rotation clockwise.
///
/// ## Platform-specific
///
/// - Only available on **`macOS`**.
#[derive(Event, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct TouchpadRotate {
    /// The amount of rotation.
    pub delta: f32,
    /// The source of this input event.
    pub source: InputSource,
}
