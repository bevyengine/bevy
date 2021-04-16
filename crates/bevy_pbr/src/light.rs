use bevy_core::Byteable;
use bevy_ecs::reflect::ReflectComponent;
use bevy_reflect::Reflect;
use bevy_render::color::Color;
use bevy_transform::components::GlobalTransform;

/// A point light
#[derive(Debug, Reflect)]
#[reflect(Component)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        PointLight {
            color: Color::rgb(1.0, 1.0, 1.0),
            intensity: 200.0,
            range: 20.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(crate) struct PointLightUniform {
    pub pos: [f32; 4],
    pub color: [f32; 4],
    // storing as a `[f32; 4]` for memory alignement
    pub inverse_range_squared: [f32; 4],
}

unsafe impl Byteable for PointLightUniform {}

impl PointLightUniform {
    pub fn from(light: &PointLight, global_transform: &GlobalTransform) -> PointLightUniform {
        let (x, y, z) = global_transform.translation.into();

        // premultiply color by intensity
        // we don't use the alpha at all, so no reason to multiply only [0..3]
        let color: [f32; 4] = (light.color * light.intensity).into();
        PointLightUniform {
            pos: [x, y, z, 1.0],
            color,
            inverse_range_squared: [1.0 / (light.range * light.range), 0., 0., 0.],
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
