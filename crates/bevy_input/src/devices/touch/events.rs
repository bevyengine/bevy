use super::*;
use bevy_math::Vec2;

/// A touch input event
#[derive(Debug, Clone)]
pub struct TouchEvent {
    pub phase: TouchPhaseCode,
    pub position: Vec2,
    ///
    /// ## Platform-specific
    ///
    /// Unique identifier of a finger.
    pub id: u64,
}
