use bevy_core::{Pod, Zeroable};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::Vec3;
use bevy_reflect::Reflect;
use bevy_render::color::Color;
use bevy_transform::components::GlobalTransform;

/// A point light
#[derive(Component, Debug, Clone, Copy, Reflect)]
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

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct PointLightUniform {
    pub pos: [f32; 4],
    pub color: [f32; 4],
    // storing as a `[f32; 4]` for memory alignement
    pub light_params: [f32; 4],
}

impl PointLightUniform {
    pub fn new(light: &PointLight, global_transform: &GlobalTransform) -> PointLightUniform {
        let (x, y, z) = global_transform.translation.into();

        // premultiply color by intensity
        // we don't use the alpha at all, so no reason to multiply only [0..3]
        let color: [f32; 4] = (light.color * light.intensity).into();

        PointLightUniform {
            pos: [x, y, z, 1.0],
            color,
            light_params: [1.0 / (light.range * light.range), light.radius, 0.0, 0.0],
        }
    }
}

/// A Directional light.
///
/// Directional lights don't exist in reality but they are a good
/// approximation for light sources VERY far away, like the sun or
/// the moon.
///
/// Valid values for `illuminance` are:
///
/// | Illuminance (lux) | Surfaces illuminated by                        |
/// |-------------------|------------------------------------------------|
/// | 0.0001            | Moonless, overcast night sky (starlight)       |
/// | 0.002             | Moonless clear night sky with airglow          |
/// | 0.05–0.3          | Full moon on a clear night                     |
/// | 3.4               | Dark limit of civil twilight under a clear sky |
/// | 20–50             | Public areas with dark surroundings            |
/// | 50                | Family living room lights                      |
/// | 80                | Office building hallway/toilet lighting        |
/// | 100               | Very dark overcast day                         |
/// | 150               | Train station platforms                        |
/// | 320–500           | Office lighting                                |
/// | 400               | Sunrise or sunset on a clear day.              |
/// | 1000              | Overcast day; typical TV studio lighting       |
/// | 10,000–25,000     | Full daylight (not direct sun)                 |
/// | 32,000–100,000    | Direct sunlight                                |
///
/// Source: [Wikipedia](https://en.wikipedia.org/wiki/Lux)
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct DirectionalLight {
    pub color: Color,
    pub illuminance: f32,
    direction: Vec3,
}

impl DirectionalLight {
    /// Create a new directional light component.
    pub fn new(color: Color, illuminance: f32, direction: Vec3) -> Self {
        DirectionalLight {
            color,
            illuminance,
            direction: direction.normalize(),
        }
    }

    /// Set direction of light.
    pub fn set_direction(&mut self, direction: Vec3) {
        self.direction = direction.normalize();
    }

    pub fn get_direction(&self) -> Vec3 {
        self.direction
    }
}

impl Default for DirectionalLight {
    fn default() -> Self {
        DirectionalLight {
            color: Color::rgb(1.0, 1.0, 1.0),
            illuminance: 100000.0,
            direction: Vec3::new(0.0, -1.0, 0.0),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
pub(crate) struct DirectionalLightUniform {
    pub dir: [f32; 4],
    pub color: [f32; 4],
}

impl DirectionalLightUniform {
    pub fn new(light: &DirectionalLight) -> DirectionalLightUniform {
        // direction is negated to be ready for N.L
        let dir: [f32; 4] = [
            -light.direction.x,
            -light.direction.y,
            -light.direction.z,
            0.0,
        ];

        // convert from illuminance (lux) to candelas
        //
        // exposure is hard coded at the moment but should be replaced
        // by values coming from the camera
        // see: https://google.github.io/filament/Filament.html#imagingpipeline/physicallybasedcamera/exposuresettings
        const APERTURE: f32 = 4.0;
        const SHUTTER_SPEED: f32 = 1.0 / 250.0;
        const SENSITIVITY: f32 = 100.0;
        let ev100 = f32::log2(APERTURE * APERTURE / SHUTTER_SPEED) - f32::log2(SENSITIVITY / 100.0);
        let exposure = 1.0 / (f32::powf(2.0, ev100) * 1.2);
        let intensity = light.illuminance * exposure;

        // premultiply color by intensity
        // we don't use the alpha at all, so no reason to multiply only [0..3]
        let color: [f32; 4] = (light.color * intensity).into();

        DirectionalLightUniform { dir, color }
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
