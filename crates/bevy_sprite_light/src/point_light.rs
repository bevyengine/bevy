use bevy_camera::visibility::Visibility;
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_reflect::prelude::*;
use bevy_transform::components::Transform;

/// A light that provides illumination in all directions.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(Transform, Visibility)]
pub struct PointLight2d {
    /// The color of the light.
    pub color: Color,
    /// The intensity of the light. Higher values make the light brighter.
    pub intensity: f32,
    /// The range of the light. Illumination will only occur within the light's range.
    pub range: f32,
    /// How quickly illumination from the light should deteriorate over distance.
    /// A higher falloff value will result in less illumination at the light's maximum radius.
    pub falloff: f32,
}

impl Default for PointLight2d {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
            range: 100.0,
            falloff: 0.0,
        }
    }
}
