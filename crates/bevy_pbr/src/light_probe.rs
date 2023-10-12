use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::Handle;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    reflect::ReflectComponent,
    system::{Commands, Query},
};
use bevy_math::{Mat4, Vec3A, Vec4Swizzles};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::mesh::Mesh;
use bevy_transform::prelude::GlobalTransform;
use smallvec::SmallVec;

use crate::{environment_map::EnvironmentMapLightId, EnvironmentMapLight};

pub struct LightProbePlugin;

/// A cuboid region that provides global illumination to all meshes inside it.
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct LightProbe {
    /// The influence range of the light probe.
    pub half_extents: Vec3A,
}

/// Which light probes this mesh must take into account.
#[derive(Component, Debug, Clone)]
pub struct AppliedLightProbes {
    pub reflection_probe: EnvironmentMapLightId,
}

struct LightProbeApplicationInfo {
    inverse_transform: Mat4,
    half_extents: Vec3A,
    light_probes: AppliedLightProbes,
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
        app.register_type::<LightProbe>()
            .add_systems(PostUpdate, apply_light_probes);
    }
}

pub fn apply_light_probes(
    mut commands: Commands,
    mesh_query: Query<(Entity, &GlobalTransform), With<Handle<Mesh>>>,
    light_probe_query: Query<(&LightProbe, &EnvironmentMapLight, &GlobalTransform)>,
) {
    if mesh_query.is_empty() {
        return;
    }

    let mut light_probes: SmallVec<[LightProbeApplicationInfo; 4]> = SmallVec::new();
    for (light_probe, environment_map_light, light_probe_transform) in light_probe_query.iter() {
        light_probes.push(LightProbeApplicationInfo {
            inverse_transform: light_probe_transform.compute_matrix().inverse(),
            half_extents: light_probe.half_extents,
            light_probes: AppliedLightProbes {
                reflection_probe: environment_map_light.id(),
            },
        })
    }

    'outer: for (mesh_entity, mesh_transform) in mesh_query.iter() {
        for light_probe_info in &light_probes {
            let probe_space_mesh_center: Vec3A = (light_probe_info.inverse_transform
                * mesh_transform.translation_vec3a().extend(1.0))
            .xyz()
            .into();

            if (-light_probe_info.half_extents)
                .cmple(probe_space_mesh_center)
                .all()
                && probe_space_mesh_center
                    .cmple(light_probe_info.half_extents)
                    .all()
            {
                commands
                    .entity(mesh_entity)
                    .insert(light_probe_info.light_probes.clone());
                continue 'outer;
            }
        }
    }
}
