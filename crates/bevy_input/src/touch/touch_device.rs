use bevy_math::Vec2;

/// A touch input event
#[derive(Debug, Clone)]
pub struct TouchInput {
    pub phase: TouchPhase,
    pub position: Vec2,
    ///
    /// ## Platform-specific
    ///
    /// Unique identifier of a finger.
    pub id: u64,
}

/// Describes touch-screen input state.
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum TouchPhase {
    Started,
    Moved,
    Ended,
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct Touch {
    pub id: u64,
    pub start_position: Vec2,
    pub previous_position: Vec2,
    pub position: Vec2,
}

impl Touch {
    pub fn delta(&self) -> Vec2 {
        self.position - self.previous_position
    }

    pub fn distance(&self) -> Vec2 {
        self.position - self.start_position
    }
}
