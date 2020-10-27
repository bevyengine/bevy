use bevy_math::Vec2;

/// A mouse motion event
#[derive(Debug, Clone)]
pub struct MouseMotion {
    pub delta: Vec2,
}
