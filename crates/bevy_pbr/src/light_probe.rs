use bevy_app::{Plugin, App};
use bevy_ecs::{component::Component, reflect::ReflectComponent};
use bevy_math::Vec3A;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};

pub struct LightProbePlugin;

/// A cuboid region that provides global illumination to all meshes inside it.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct LightProbe {
    /// The influence range of the light probe.
    pub half_extents: Vec3A,
}

impl LightProbe {
    /// Creates a new light probe component with the given half-extents.
    #[inline]
    pub fn new(half_extents: Vec3A) -> Self {
        Self { half_extents }
    }
}

impl Default for LightProbe {
    #[inline]
    fn default() -> Self {
        Self {
            half_extents: Vec3A::splat(1.0),
        }
    }
}

impl Plugin for LightProbePlugin {
    fn build(&self, app: &mut App) {
        app.register_type::<LightProbe>();
    }
}
