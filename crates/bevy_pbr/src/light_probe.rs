use bevy_app::{App, AppLabel, Plugin};
use bevy_ecs::{component::Component, entity::Entity, reflect::ReflectComponent, system::Resource};
use bevy_math::Vec3A;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::RenderApp;
use bevy_utils::EntityHashMap;

pub struct LightProbePlugin;

/// A cuboid region that provides global illumination to all meshes inside it.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct LightProbe {
    /// The influence range of the light probe.
    pub half_extents: Vec3A,
}

/// Which light probe is to be assigned to each mesh.
///
/// TODO: Allow multiple light probes to be assigned to each mesh, and
/// interpolate between them.
#[derive(Resource, Default)]
pub struct RenderMeshLightProbeInstances(EntityHashMap<Entity, RenderMeshLightProbes>);

pub struct RenderMeshLightProbes {
    environment_map_index: u32,
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

        let Ok(ref mut render_app) = app.get_sub_app_mut(RenderApp) else { return };
        render_app.init_resource::<RenderMeshLightProbeInstances>();
    }
}
