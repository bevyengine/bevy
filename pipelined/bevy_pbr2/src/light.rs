use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::Reflect;
use bevy_render2::color::Color;

/// A point light
#[derive(Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub radius: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        PointLight {
            color: Color::rgb(1.0, 1.0, 1.0),
            intensity: 200.0,
            range: 20.0,
            radius: 0.0,
        }
    }
}
