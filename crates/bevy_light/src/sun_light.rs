use bevy_ecs::prelude::*;
use super::DirectionalLight;

/// This component marks a directional light entity for changing the size and intensity of the sun disk.
#[derive(Component, Clone)]
#[require(DirectionalLight)]
pub struct SunLight {
    /// The angular size of the sun disk in radians as observed from Earth.
    pub angular_size: f32,
    /// Multiplier applied to the brightness of the sun disk in the sky.
    ///
    /// `0.0` disables the sun disk entirely while still
    /// allowing the sun's radiance to scatter into the atmosphere,
    /// and `1.0` renders the sun disk at its normal intensity.
    pub intensity: f32,
}

impl SunLight {
    pub const SUN: SunLight = SunLight {
        // 32 arc minutes is the mean size of the sun disk when the Earth is
        // exactly 1 astronomical unit from the sun.
        angular_size: 0.00930842,
        intensity: 1.0,
    };
}

impl Default for SunLight {
    fn default() -> Self {
        Self::SUN
    }
}

impl Default for &SunLight {
    fn default() -> Self {
        &SunLight::SUN
    }
}