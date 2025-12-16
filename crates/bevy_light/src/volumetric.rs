use bevy_asset::Handle;
use bevy_camera::visibility::Visibility;
use bevy_color::Color;
use bevy_ecs::prelude::*;
use bevy_image::Image;
use bevy_math::Vec3;
use bevy_reflect::prelude::*;
use bevy_transform::components::Transform;

/// Add this component to a [`DirectionalLight`](crate::DirectionalLight) with a shadow map
/// (`shadows_enabled: true`) to make volumetric fog interact with it.
///
/// This allows the light to generate light shafts/god rays.
#[derive(Clone, Copy, Component, Default, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct VolumetricLight;

/// When placed on a [`bevy_camera::Camera3d`], enables
/// volumetric fog and volumetric lighting, also known as light shafts or god
/// rays.
///
/// Requires using WebGPU on Wasm builds.
#[derive(Clone, Copy, Component, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
pub struct VolumetricFog {
    /// Color of the ambient light.
    ///
    /// This is separate from Bevy's [`AmbientLight`](crate::AmbientLight) because an
    /// [`EnvironmentMapLight`](crate::EnvironmentMapLight) is
    /// still considered an ambient light for the purposes of volumetric fog. If you're using a
    /// [`EnvironmentMapLight`](crate::EnvironmentMapLight), for best results,
    /// this should be a good approximation of the average color of the environment map.
    ///
    /// Defaults to white.
    pub ambient_color: Color,

    /// The brightness of the ambient light.
    ///
    /// If there's no [`EnvironmentMapLight`](crate::EnvironmentMapLight),
    /// set this to 0.
    ///
    /// Defaults to 0.1.
    pub ambient_intensity: f32,

    /// The maximum distance to offset the ray origin randomly by, in meters.
    ///
    /// This is intended for use with temporal antialiasing. It helps fog look
    /// less blocky by varying the start position of the ray, using interleaved
    /// gradient noise.
    pub jitter: f32,

    /// The number of raymarching steps to perform.
    ///
    /// Higher values produce higher-quality results with less banding, but
    /// reduce performance.
    ///
    /// The default value is 64.
    pub step_count: u32,
}

impl Default for VolumetricFog {
    fn default() -> Self {
        Self {
            step_count: 64,
            // Matches `AmbientLight` defaults.
            ambient_color: Color::WHITE,
            ambient_intensity: 0.1,
            jitter: 0.0,
        }
    }
}

#[derive(Clone, Component, Debug, Reflect)]
#[reflect(Component, Default, Debug, Clone)]
#[require(Transform, Visibility)]
pub struct FogVolume {
    /// The color of the fog.
    ///
    /// Note that the fog must be lit by a [`VolumetricLight`] or ambient light
    /// in order for this color to appear.
    ///
    /// Defaults to white.
    pub fog_color: Color,

    /// The density of fog, which measures how dark the fog is.
    ///
    /// The default value is 0.1.
    pub density_factor: f32,

    /// Optional 3D voxel density texture for the fog.
    pub density_texture: Option<Handle<Image>>,

    /// Configurable offset of the density texture in UVW coordinates.
    ///
    /// This can be used to scroll a repeating density texture in a direction over time
    /// to create effects like fog moving in the wind. Make sure to configure the texture
    /// to use `ImageAddressMode::Repeat` if this is your intention.
    ///
    /// Has no effect when no density texture is present.
    ///
    /// The default value is (0, 0, 0).
    pub density_texture_offset: Vec3,

    /// The absorption coefficient, which measures what fraction of light is
    /// absorbed by the fog at each step.
    ///
    /// Increasing this value makes the fog darker.
    ///
    /// The default value is 0.3.
    pub absorption: f32,

    /// The scattering coefficient, which measures the fraction of light that's
    /// scattered toward, and away from, the viewer.
    ///
    /// The default value is 0.3.
    pub scattering: f32,

    /// Measures the fraction of light that's scattered *toward* the camera, as
    /// opposed to *away* from the camera.
    ///
    /// Increasing this value makes light shafts become more prominent when the
    /// camera is facing toward their source and less prominent when the camera
    /// is facing away. Essentially, a high value here means the light shafts
    /// will fade into view as the camera focuses on them and fade away when the
    /// camera is pointing away.
    ///
    /// The default value is 0.8.
    pub scattering_asymmetry: f32,

    /// Applies a nonphysical color to the light.
    ///
    /// This can be useful for artistic purposes but is nonphysical.
    ///
    /// The default value is white.
    pub light_tint: Color,

    /// Scales the light by a fixed fraction.
    ///
    /// This can be useful for artistic purposes but is nonphysical.
    ///
    /// The default value is 1.0, which results in no adjustment.
    pub light_intensity: f32,
}

impl Default for FogVolume {
    fn default() -> Self {
        Self {
            absorption: 0.3,
            scattering: 0.3,
            density_factor: 0.1,
            density_texture: None,
            density_texture_offset: Vec3::ZERO,
            scattering_asymmetry: 0.5,
            fog_color: Color::WHITE,
            light_tint: Color::WHITE,
            light_intensity: 1.0,
        }
    }
}
