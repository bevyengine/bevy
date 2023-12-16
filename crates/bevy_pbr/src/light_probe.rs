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
    render_asset::RenderAssets,
    render_resource::{DynamicUniformBuffer, ShaderType},
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::{EntityHashMap, FloatOrd};
use smallvec::SmallVec;

use crate::{
    environment_map::{EnvironmentMapIds, RenderViewEnvironmentMaps},
    EnvironmentMapLight,
};

/// The maximum number of reflection probes that each view will consider.
///
/// Because the fragment shader does a linear search through the list for each
/// fragment, this number needs to be relatively small.
pub const MAX_VIEW_REFLECTION_PROBES: usize = 8;

/// Adds support for light probes: cuboid bounding regions that apply global
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
struct RenderReflectionProbe {
    /// The transform from the world space to the model space. This is used to
    /// efficiently check for bounding box intersection.
    inverse_transform: Mat4,

    /// The half-extents of the bounding cube.
    half_extents: Vec3,

    /// The index of the environment map in the diffuse and specular cubemap texture array.
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

    /// The index of the diffuse and specular environment maps associated with
    /// the view itself. This is used as a fallback if no reflection probe in
    /// the list contains the fragment.
    cubemap_index: i32,
}

/// A map from each camera to the light probe uniform associated with it.
#[derive(Resource, Default, Deref, DerefMut)]
struct RenderLightProbes(EntityHashMap<Entity, LightProbesUniform>);

/// A GPU buffer that stores information about all light probes.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct LightProbesBuffer(DynamicUniformBuffer<LightProbesUniform>);

/// A component attached to each camera in the render world that stores the
/// index of the [LightProbesUniform] in the [LightProbesBuffer].
#[derive(Component, Default, Deref, DerefMut)]
pub struct ViewLightProbesUniformOffset(u32);

/// Information that [`gather_light_probes`] keeps about each light probe.
#[derive(Clone, Copy)]
struct LightProbeInfo {
    // The transform from world space to light probe space.
    inverse_transform: Mat4,
    // The transform from light probe space to world space.
    affine_transform: Affine3A,
    // Extents of the bounding box.
    half_extents: Vec3,
    environment_maps: EnvironmentMapIds,
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

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<LightProbesBuffer>()
            .init_resource::<RenderLightProbes>()
            .add_systems(ExtractSchedule, gather_light_probes)
            .add_systems(
                Render,
                upload_light_probes.in_set(RenderSet::PrepareResources),
            );
    }
}

/// Gathers up all light probes in the scene and assigns them to views,
/// performing frustum culling and distance sorting in the process.
///
/// This populates the [`LightProbesUniforms`] resource.
fn gather_light_probes(
    mut light_probes_uniforms: ResMut<RenderLightProbes>,
    image_assets: Res<RenderAssets<Image>>,
    light_probe_query: Extract<Query<(&LightProbe, &GlobalTransform, &EnvironmentMapLight)>>,
    view_query: Extract<
        Query<
            (
                Entity,
                &GlobalTransform,
                &Frustum,
                Option<&EnvironmentMapLight>,
            ),
            With<Camera3d>,
        >,
    >,
    mut commands: Commands,
) {
    // Create [`LightProbeInfo`] for every light probe in the scene.
    let light_probes: SmallVec<[LightProbeInfo; 8]> = light_probe_query
        .iter()
        .filter_map(|query_row| LightProbeInfo::new(query_row, &image_assets))
        .collect();

    // Build up the light probes uniform and the key table.
    light_probes_uniforms.clear();
    for (view_entity, view_transform, view_frustum, view_environment_maps) in view_query.iter() {
        // Cull light probes outside the view frustum.
        let mut view_light_probes: SmallVec<[LightProbeInfo; 8]> = light_probes
            .iter()
            .filter(|light_probe_info| light_probe_info.frustum_cull(view_frustum))
            .cloned()
            .collect();

        // Sort by distance to camera.
        view_light_probes.sort_by_cached_key(|light_probe_info| {
            light_probe_info.camera_distance_sort_key(view_transform)
        });

        // Create the light probes uniform.
        let (light_probes_uniform, render_view_environment_maps) =
            LightProbesUniform::build(view_environment_maps, &image_assets, &light_probes);

        // Record the uniforms.
        light_probes_uniforms.insert(view_entity, light_probes_uniform);

        // Record the per-view environment maps.
        let mut commands = commands.get_or_spawn(view_entity);
        if render_view_environment_maps.is_empty() {
            commands.remove::<RenderViewEnvironmentMaps>();
        } else {
            commands.insert(render_view_environment_maps);
        }
    }
}

/// Uploads the result of [`gather_light_probes`] to the GPU.
fn upload_light_probes(
    mut commands: Commands,
    light_probes_uniforms: Res<RenderLightProbes>,
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
            cubemap_index: -1,
        }
    }
}

impl LightProbesUniform {
    fn build(
        view_environment_maps: Option<&EnvironmentMapLight>,
        image_assets: &RenderAssets<Image>,
        light_probes: &[LightProbeInfo],
    ) -> (LightProbesUniform, RenderViewEnvironmentMaps) {
        let mut render_view_environment_maps = RenderViewEnvironmentMaps::new();
        let mut uniform = LightProbesUniform {
            reflection_probes: [RenderReflectionProbe::default(); MAX_VIEW_REFLECTION_PROBES],
            reflection_probe_count: light_probes.len().min(MAX_VIEW_REFLECTION_PROBES) as i32,
            cubemap_index: match view_environment_maps {
                Some(&EnvironmentMapLight {
                    ref diffuse_map,
                    ref specular_map,
                }) if image_assets.get(diffuse_map).is_some()
                    && image_assets.get(specular_map).is_some() =>
                {
                    render_view_environment_maps.get_or_insert_cubemap(&EnvironmentMapIds {
                        diffuse: diffuse_map.id(),
                        specular: specular_map.id(),
                    }) as i32
                }
                _ => -1,
            },
        };

        uniform.maybe_gather_reflection_probes(&mut render_view_environment_maps, &light_probes);
        (uniform, render_view_environment_maps)
    }

    #[cfg(any(feature = "shader_format_glsl", target_arch = "wasm32"))]
    fn maybe_gather_reflection_probes(
        &mut self,
        _: &mut RenderViewEnvironmentMaps,
        _: &[LightProbeInfo],
    ) {
    }

    #[cfg(all(not(feature = "shader_format_glsl"), not(target_arch = "wasm32")))]
    fn maybe_gather_reflection_probes(
        &mut self,
        render_view_environment_maps: &mut RenderViewEnvironmentMaps,
        light_probes: &[LightProbeInfo],
    ) {
        let light_probe_count = light_probes.len().min(MAX_VIEW_REFLECTION_PROBES);
        for light_probe_index in 0..light_probe_count {
            let cubemap_index = render_view_environment_maps
                .get_or_insert_cubemap(&light_probes[light_probe_index].environment_maps)
                as i32;

            self.reflection_probes[light_probe_index] = RenderReflectionProbe {
                inverse_transform: light_probes[light_probe_index].inverse_transform,
                half_extents: light_probes[light_probe_index].half_extents,
                cubemap_index,
            };
        }
    }
}

impl LightProbeInfo {
    /// Given the set of light probe components, constructs and returns
    /// [`LightProbeInfo`]. This is done for every light probe in the scene
    /// every frame.
    fn new(
        (light_probe, light_probe_transform, environment_map): (
            &LightProbe,
            &GlobalTransform,
            &EnvironmentMapLight,
        ),
        image_assets: &RenderAssets<Image>,
    ) -> Option<LightProbeInfo> {
        if image_assets.get(&environment_map.diffuse_map).is_none()
            || image_assets.get(&environment_map.specular_map).is_none()
        {
            return None;
        }

        Some(LightProbeInfo {
            affine_transform: light_probe_transform.affine(),
            inverse_transform: light_probe_transform.compute_matrix().inverse(),
            half_extents: light_probe.half_extents.into(),
            environment_maps: EnvironmentMapIds {
                diffuse: environment_map.diffuse_map.id(),
                specular: environment_map.specular_map.id(),
            },
        })
    }

    /// Returns true if this light probe is in the viewing frustum of the camera
    /// or false if it isn't.
    fn frustum_cull(&self, view_frustum: &Frustum) -> bool {
        view_frustum.intersects_obb(
            &Aabb {
                center: Vec3A::default(),
                half_extents: self.half_extents.into(),
            },
            &self.affine_transform,
            true,
            false,
        )
    }

    /// Returns the squared distance from this light probe to the camera,
    /// suitable for distance sorting.
    fn camera_distance_sort_key(&self, view_transform: &GlobalTransform) -> FloatOrd {
        FloatOrd(
            (self.affine_transform.translation - view_transform.translation_vec3a())
                .length_squared(),
        )
    }
}
