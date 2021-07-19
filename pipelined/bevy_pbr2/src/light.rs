use bevy_render2::{camera::OrthographicProjection, color::Color};

/// A light that emits light in all directions from a central point.
#[derive(Debug, Clone, Copy)]
pub struct PointLight {
    pub color: Color,
    pub intensity: f32,
    pub range: f32,
    pub radius: f32,
    pub shadow_depth_bias: f32,
    /// A bias applied along the direction of the fragment's surface normal. It is scaled to the
    /// shadow map's texel size so that it can be small close to the camera and gets larger further
    /// away.
    pub shadow_normal_bias: f32,
}

impl Default for PointLight {
    fn default() -> Self {
        PointLight {
            color: Color::rgb(1.0, 1.0, 1.0),
            intensity: 200.0,
            range: 20.0,
            radius: 0.0,
            shadow_depth_bias: Self::DEFAULT_SHADOW_DEPTH_BIAS,
            shadow_normal_bias: Self::DEFAULT_SHADOW_NORMAL_BIAS,
        }
    }
}

impl PointLight {
    pub const DEFAULT_SHADOW_DEPTH_BIAS: f32 = 0.02;
    pub const DEFAULT_SHADOW_NORMAL_BIAS: f32 = 0.5;
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
#[derive(Debug, Clone)]
pub struct DirectionalLight {
    pub color: Color,
    pub illuminance: f32,
    pub shadow_projection: OrthographicProjection,
    pub shadow_depth_bias: f32,
    /// A bias applied along the direction of the fragment's surface normal. It is scaled to the
    /// shadow map's texel size so that it is automatically adjusted to the orthographic projection.
    pub shadow_normal_bias: f32,
}

impl Default for DirectionalLight {
    fn default() -> Self {
        let size = 100.0;
        DirectionalLight {
            color: Color::rgb(1.0, 1.0, 1.0),
            illuminance: 100000.0,
            shadow_projection: OrthographicProjection {
                left: -size,
                right: size,
                bottom: -size,
                top: size,
                near: -size,
                far: size,
                ..Default::default()
            },
            shadow_depth_bias: Self::DEFAULT_SHADOW_DEPTH_BIAS,
            shadow_normal_bias: Self::DEFAULT_SHADOW_NORMAL_BIAS,
        }
    }
}

impl DirectionalLight {
    pub const DEFAULT_SHADOW_DEPTH_BIAS: f32 = 0.02;
    pub const DEFAULT_SHADOW_NORMAL_BIAS: f32 = 0.6;
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
