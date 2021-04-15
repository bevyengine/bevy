use bevy_ecs::reflect::ReflectComponent;
use bevy_math::Vec3;
use bevy_reflect::Reflect;
use bevy_render::color::Color;
use bevy_transform::components::GlobalTransform;

use crevice::std140::AsStd140;

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

#[derive(Debug, Clone, Copy, AsStd140)]
pub(crate) struct PointLightUniform {
    pub pos: Vec3,
    pub inverse_range_squared: f32,
    pub color: Vec3,
}

impl PointLightUniform {
    pub fn from_tuple(tuple: (&PointLight, &GlobalTransform)) -> PointLightUniform {
        Self::from(tuple.0, tuple.1)
    }

    pub fn from(light: &PointLight, global_transform: &GlobalTransform) -> PointLightUniform {
        let (x, y, z) = global_transform.translation.into();

        // premultiply color by intensity
        // we don't use the alpha at all, so no reason to multiply only [0..3]
        let [r, g, b, _]: [f32; 4] = (light.color * light.intensity).into();
        PointLightUniform {
            pos: Vec3::new(x, y, z),
            color: Vec3::new(r, g, b),
            inverse_range_squared: 1.0 / (light.range * light.range),
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

// custom implementation that premultiplies brightness and only passes 3 components
impl AsStd140 for AmbientLight {
    type Std140Type = crevice::std140::Vec3;

    fn as_std140(&self) -> Self::Std140Type {
        let precomputed: Color = self.color * self.brightness;
        crevice::std140::Vec3 {
            x: precomputed.r(),
            y: precomputed.g(),
            z: precomputed.b(),
        }
    }
}
