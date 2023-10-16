//! Light probes for baked global illumination.

use bevy_app::{App, Plugin};
use bevy_core_pipeline::core_3d::Camera3d;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut, Resource},
};
use bevy_math::{Affine3A, Mat4, Vec3, Vec3A};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    primitives::{Aabb, Frustum},
    render_resource::{DynamicUniformBuffer, ShaderType},
    renderer::{RenderDevice, RenderQueue},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::{EntityHashMap, FloatOrd};
use smallvec::SmallVec;

use crate::{
    environment_map::{self, RenderEnvironmentMaps},
    EnvironmentMapLight,
};

/// The maximum number of reflection probes that each view will consider.
///
/// Because the fragment shader does a linear search through the list for each
/// fragment, this number needs to be relatively small.
pub const MAX_VIEW_REFLECTION_PROBES: usize = 4;

/// Adds support for light probes, cuboid bounding regions that apply global
/// illumination to objects within them.
pub struct LightProbePlugin;

/// A cuboid region that provides global illumination to all fragments inside it.
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

/// A GPU type that stores information about a reflection probe.
#[derive(Clone, Copy, ShaderType, Default)]
pub struct RenderReflectionProbe {
    /// The transform from the world space to the model space. This is used to
    /// efficiently check for bounding box intersection.
    inverse_transform: Mat4,

    /// The half-extents of the bounding cube.
    half_extents: Vec3,

    /// The index of the environment map in the diffuse and specular cubemap
    /// texture arrays.
    cubemap_index: i32,
}

/// A per-view shader uniform that specifies all the light probes that the view
/// takes into account.
#[derive(ShaderType)]
pub struct LightProbesUniform {
    /// The list of applicable reflection probes, sorted from nearest to the
    /// camera to the farthest away from the camera.
    reflection_probes: [RenderReflectionProbe; MAX_VIEW_REFLECTION_PROBES],

    /// The number of reflection probes in the list.
    reflection_probe_count: i32,

    /// The index of the environment map associated with the view itself. This
    /// is used as a fallback if no reflection probe in the list contains the
    /// fragment.
    view_cubemap_index: i32,
}

/// A map from each camera to the light probe uniform associated with it.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct LightProbesUniforms(EntityHashMap<Entity, LightProbesUniform>);

/// A GPU buffer that stores information about all light probes.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct LightProbesBuffer(pub DynamicUniformBuffer<LightProbesUniform>);

/// A component attached to each camera in the render world that stores the
/// index of the [LightProbesUniform] in the [LightProbesBuffer].
#[derive(Component, Default, Deref, DerefMut)]
pub struct ViewLightProbesUniformOffset(pub u32);

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

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<LightProbesBuffer>()
            .init_resource::<LightProbesUniforms>()
            .add_systems(ExtractSchedule, gather_light_probes)
            .add_systems(
                Render,
                upload_light_probes
                    .in_set(RenderSet::PrepareResources)
                    .after(environment_map::prepare_environment_maps),
            );
    }
}

/// Gathers up all light probes in the scene and assigns them to views,
/// performing frustum culling and distance sorting in the process.
///
/// This populates the [LightProbesUniforms] resource.
pub fn gather_light_probes(
    render_environment_maps: Res<RenderEnvironmentMaps>,
    mut light_probes_uniforms: ResMut<LightProbesUniforms>,
    light_probe_query: Extract<Query<(&LightProbe, &EnvironmentMapLight, &GlobalTransform)>>,
    view_query: Extract<
        Query<
            (
                Entity,
                Option<&EnvironmentMapLight>,
                &GlobalTransform,
                &Frustum,
            ),
            With<Camera3d>,
        >,
    >,
) {
    // Create [RenderLightProbe]s for every light probe in the scene.
    let mut light_probes: SmallVec<[LightProbeInfo; 8]> = SmallVec::new();
    for (light_probe, light_probe_light, light_probe_transform) in light_probe_query.iter() {
        if let Some(&cubemap_index) = render_environment_maps
            .light_id_indices
            .get(&light_probe_light.id())
        {
            light_probes.push(LightProbeInfo {
                affine_transform: light_probe_transform.affine(),
                inverse_transform: light_probe_transform.compute_matrix().inverse(),
                half_extents: light_probe.half_extents.into(),
                cubemap_index,
            })
        }
    }

    // Build up the light probes uniform.
    light_probes_uniforms.clear();
    for (view_entity, view_environment_map, view_transform, view_frustum) in view_query.iter() {
        // Cull light probes outside the view frustum.
        let mut view_light_probes: SmallVec<[LightProbeInfo; 8]> = SmallVec::new();
        for light_probe_info in &light_probes {
            // FIXME(pcwalton): Should we intersect with the far plane?
            if view_frustum.intersects_obb(
                &Aabb {
                    center: Vec3A::default(),
                    half_extents: light_probe_info.half_extents.into(),
                },
                &light_probe_info.affine_transform,
                true,
                false,
            ) {
                view_light_probes.push(*light_probe_info);
            }
        }

        // Sort by distance to camera.
        view_light_probes.sort_by_cached_key(|light_probe_info| {
            FloatOrd(
                (light_probe_info.affine_transform.translation
                    - view_transform.translation_vec3a())
                .length_squared(),
            )
        });

        // Create the light probes uniform.
        let mut light_probes_uniform = LightProbesUniform {
            reflection_probes: [RenderReflectionProbe::default(); MAX_VIEW_REFLECTION_PROBES],
            reflection_probe_count: light_probes.len().min(MAX_VIEW_REFLECTION_PROBES) as i32,
            view_cubemap_index: match view_environment_map {
                Some(view_environment_map) => {
                    render_environment_maps.get_index(&view_environment_map.id())
                }
                None => -1,
            },
        };

        // Now that we have the list of sorted reflection probes, build up the
        // list for the GPU.
        let light_probe_count = light_probes.len().min(MAX_VIEW_REFLECTION_PROBES);
        for light_probe_index in 0..light_probe_count {
            light_probes_uniform.reflection_probes[light_probe_index] = RenderReflectionProbe {
                inverse_transform: light_probes[light_probe_index].inverse_transform,
                half_extents: light_probes[light_probe_index].half_extents,
                cubemap_index: light_probes[light_probe_index].cubemap_index,
            };
        }

        // Insert the result.
        light_probes_uniforms.insert(view_entity, light_probes_uniform);
    }

    // Information that this function keeps about each light probe.
    #[derive(Clone, Copy)]
    struct LightProbeInfo {
        // The transform from world space to light probe space.
        inverse_transform: Mat4,
        // The transform from light probe space to world space.
        affine_transform: Affine3A,
        // Extents of the bounding box.
        half_extents: Vec3,
        // The index of the reflection probe corresponding to this lightmap in
        // the diffuse and specular cubemap arrays.
        cubemap_index: i32,
    }
}

/// Runs after [gather_light_probes] and uploads the result to the GPU.
pub fn upload_light_probes(
    mut commands: Commands,
    light_probes_uniforms: Res<LightProbesUniforms>,
    mut light_probes_buffer: ResMut<LightProbesBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // Get the writer.
    let Some(mut writer) =
        light_probes_buffer.get_writer(light_probes_uniforms.len(), &render_device, &render_queue)
    else {
        return;
    };

    // Send each view's uniforms to the GPU.
    for (&view_entity, light_probes_uniform) in light_probes_uniforms.iter() {
        commands
            .entity(view_entity)
            .insert(ViewLightProbesUniformOffset(
                writer.write(light_probes_uniform),
            ));
    }
}

impl Default for LightProbesUniform {
    fn default() -> Self {
        Self {
            reflection_probes: [RenderReflectionProbe::default(); MAX_VIEW_REFLECTION_PROBES],
            reflection_probe_count: 0,
            view_cubemap_index: -1,
        }
    }
}
