use bevy_ecs::component::Component;
use bevy_math::Vec2;

/// Details of the target the text input will be rendered to
#[derive(Component, PartialEq, Debug, Default)]
pub struct TextInputTarget {
    /// Size of the target in physical pixels
    pub size: Vec2,
    /// Scale factor of the target
    pub scale_factor: f32,
}

impl TextInputTarget {
    /// Returns true if the target has zero or negative size.
    pub fn is_empty(&self) -> bool {
        (self.scale_factor * self.size).cmple(Vec2::ZERO).all()
    }
}
