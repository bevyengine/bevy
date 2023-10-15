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

/// Adds support for light probes, cuboid bounding regions that apply global
/// illumination to objects within them.
pub struct LightProbePlugin;

/// A cuboid region that provides global illumination to all meshes inside it.
///
/// A mesh is considered inside the light probe if the mesh's origin is
/// contained within the cuboid centered at the light probe's transform with
/// width, height, and depth equal to double the value of `half_extents`.
///
/// Note that a light probe will have no effect unless the entity contains some
/// kind of illumination. At present, the only supported type of illumination is
/// the [EnvironmentMapLight].
#[derive(Component, Debug, Clone, Copy, Reflect)]
#[reflect(Component, Default)]
pub struct LightProbe {
    /// The influence range of the light probe.
    pub half_extents: Vec3A,
}

/// Which light probes this mesh must take into account.
#[derive(Component, Debug, Clone)]
pub struct AppliedLightProbes {
    /// The ID of the single light probe that this mesh reflects.
    pub reflection_probe: EnvironmentMapLightId,
}

// Information about the light probe that applies to each mesh.
//
// This is a transient structure only used by the [apply_light_probes] system.
struct LightProbeApplicationInfo {
    // Maps from the light probe's space into world space; i.e. the opposite of the light probe's
    // [GlobalTransform].
    inverse_transform: Mat4,

    // The half-extents of the light probe.
    half_extents: Vec3A,

    // The ID of the light probe.
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

/// Determines which light probe applies to each mesh and attaches
/// [AppliedLightProbes] components to them as appropriate.
pub fn apply_light_probes(
    mut commands: Commands,
    mesh_query: Query<(Entity, &GlobalTransform), With<Handle<Mesh>>>,
    light_probe_query: Query<(&LightProbe, &EnvironmentMapLight, &GlobalTransform)>,
) {
    // If there are no meshes, we can just bail.
    if mesh_query.is_empty() {
        return;
    }

    // Gather up information about all light probes in the scene.
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

    // For each mesh and each light probe…
    'outer: for (mesh_entity, mesh_transform) in mesh_query.iter() {
        for light_probe_info in &light_probes {
            // Determine the mesh center in the light probe's object space.
            // (This makes it easier to test whether the mesh is inside the
            // light probe's AABB.)
            let probe_space_mesh_center: Vec3A = (light_probe_info.inverse_transform
                * mesh_transform.translation_vec3a().extend(1.0))
            .xyz()
            .into();

            // If the mesh is inside the AABB of the light probe, add the
            // [AppliedLightProbes] component. Note that at present we naïvely
            // consider the first bounding light probe to be the one that
            // matches.
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

        // If we got here, no light probe applies, so remove the component.
        commands.entity(mesh_entity).remove::<AppliedLightProbes>();
    }
}
