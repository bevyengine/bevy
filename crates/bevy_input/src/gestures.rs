//! Gestures functionality, from touchscreens and touchpads.

use bevy_ecs::event::Event;
use bevy_math::Vec2;
use bevy_reflect::Reflect;

#[cfg(feature = "serialize")]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Two-finger pinch gesture, often used for magnifications.
///
/// Positive delta values indicate magnification (zooming in) and
/// negative delta values indicate shrinking (zooming out).
///
/// ## Platform-specific
///
/// - Only available on **`macOS`** and **`iOS`**.
/// - On **`iOS`**, must be enabled first
#[derive(Event, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct PinchGesture(pub f32);

/// Two-finger rotation gesture.
///
/// Positive delta values indicate rotation counterclockwise and
/// negative delta values indicate rotation clockwise.
///
/// ## Platform-specific
///
/// - Only available on **`macOS`** and **`iOS`**.
/// - On **`iOS`**, must be enabled first
#[derive(Event, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct RotationGesture(pub f32);

/// Double tap gesture.
///
/// ## Platform-specific
///
/// - Only available on **`macOS`** and **`iOS`**.
/// - On **`iOS`**, must be enabled first
#[derive(Event, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct DoubleTapGesture;

/// Pan gesture.
///
/// ## Platform-specific
///
/// - On **`iOS`**, must be enabled first
#[derive(Event, Debug, Clone, Copy, PartialEq, Reflect)]
#[reflect(Debug, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
    reflect(Serialize, Deserialize)
)]
pub struct PanGesture(pub Vec2);
