use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::Reflect;
use bevy_render2::color::Color;

/// An omnidirectional light
#[derive(Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct OmniLight {
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub radius: f32,
}

impl Default for OmniLight {
    fn default() -> Self {
        OmniLight {
            color: Color::rgb(1.0, 1.0, 1.0),
            intensity: 200.0,
            range: 20.0,
            radius: 0.0,
        }
    }
}

// Ambient light color.
#[derive(Debug)]
pub struct AmbientLight {
    pub color: Color,
    /// Color is premultiplied by brightness before being passed to the shader
    pub brightness: f32,
}

impl Default for AmbientLight {
    fn default() -> Self {
        Self {
            color: Color::rgb(1.0, 1.0, 1.0),
            brightness: 0.05,
        }
    }
}
