//! Light probes for baked global illumination.

use bevy_app::{App, Plugin};
use bevy_asset::load_internal_asset;
use bevy_core_pipeline::core_3d::Camera3d;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{Commands, Local, Query, Res, ResMut, Resource},
};
use bevy_math::{Affine3A, Mat4, Vec3A, Vec4};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_instances::ExtractInstancesPlugin,
    primitives::{Aabb, Frustum},
    render_asset::RenderAssets,
    render_resource::{DynamicUniformBuffer, Shader, ShaderType},
    renderer::{RenderDevice, RenderQueue},
    texture::Image,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::{EntityHashMap, FloatOrd};

use crate::light_probe::environment_map::{
    EnvironmentMapIds, EnvironmentMapLight, RenderViewEnvironmentMaps,
    ENVIRONMENT_MAP_SHADER_HANDLE,
};

pub mod environment_map;

/// The maximum number of reflection probes that each view will consider.
///
/// Because the fragment shader does a linear search through the list for each
/// fragment, this number needs to be relatively small.
pub const MAX_VIEW_REFLECTION_PROBES: usize = 8;

/// Adds support for light probes: cuboid bounding regions that apply global
/// illumination to objects within them.
///
/// This also adds support for view environment maps: diffuse and specular
/// cubemaps applied to all objects that a view renders.
pub struct LightProbePlugin;

/// A marker component for a light probe, which is a cuboid region that provides
/// global illumination to all fragments inside it.
///
/// The light probe range is conceptually a unit cube (1×1×1) centered on the
/// origin.  The [`bevy_transform::prelude::Transform`] applied to this entity
/// can scale, rotate, or translate that cube so that it contains all fragments
/// that should take this light probe into account.
///
/// Note that a light probe will have no effect unless the entity contains some
/// kind of illumination. At present, the only supported type of illumination is
/// the [`EnvironmentMapLight`].
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component, Default)]
pub struct LightProbe;

/// A GPU type that stores information about a reflection probe.
#[derive(Clone, Copy, ShaderType, Default)]
struct RenderReflectionProbe {
    /// The transform from the world space to the model space. This is used to
    /// efficiently check for bounding box intersection.
    inverse_transpose_transform: [Vec4; 3],

    /// The index of the environment map in the diffuse and specular cubemap
    /// binding arrays.
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
    view_cubemap_index: i32,

    /// The smallest valid mipmap level for the specular environment cubemap
    /// associated with the view.
    smallest_specular_mip_level_for_view: u32,
}

/// A map from each camera to the light probe uniform associated with it.
#[derive(Resource, Default, Deref, DerefMut)]
struct RenderLightProbes(EntityHashMap<Entity, LightProbesUniform>);

/// A GPU buffer that stores information about all light probes.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct LightProbesBuffer(DynamicUniformBuffer<LightProbesUniform>);

/// A component attached to each camera in the render world that stores the
/// index of the [`LightProbesUniform`] in the [`LightProbesBuffer`].
#[derive(Component, Default, Deref, DerefMut)]
pub struct ViewLightProbesUniformOffset(u32);

/// Information that [`gather_light_probes`] keeps about each light probe.
#[derive(Clone, Copy)]
#[allow(dead_code)]
struct LightProbeInfo {
    // The transform from world space to light probe space.
    inverse_transform: Mat4,
    // The transform from light probe space to world space.
    affine_transform: Affine3A,
    // The diffuse and specular environment maps associated with this light
    // probe.
    environment_maps: EnvironmentMapIds,
}

impl LightProbe {
    /// Creates a new light probe component.
    #[inline]
    pub fn new() -> Self {
        Self
    }
}

impl Plugin for LightProbePlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            ENVIRONMENT_MAP_SHADER_HANDLE,
            "environment_map.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<LightProbe>()
            .register_type::<EnvironmentMapLight>();
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_plugins(ExtractInstancesPlugin::<EnvironmentMapIds>::new())
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
/// This populates the [`RenderLightProbes`] resource.
fn gather_light_probes(
    mut render_light_probes: ResMut<RenderLightProbes>,
    image_assets: Res<RenderAssets<Image>>,
    light_probe_query: Extract<Query<(&GlobalTransform, &EnvironmentMapLight), With<LightProbe>>>,
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
    mut light_probes: Local<Vec<LightProbeInfo>>,
    mut view_light_probes: Local<Vec<LightProbeInfo>>,
    mut commands: Commands,
) {
    // Create [`LightProbeInfo`] for every light probe in the scene.
    light_probes.clear();
    light_probes.extend(
        light_probe_query
            .iter()
            .filter_map(|query_row| LightProbeInfo::new(query_row, &image_assets)),
    );

    // Build up the light probes uniform and the key table.
    render_light_probes.clear();
    for (view_entity, view_transform, view_frustum, view_environment_maps) in view_query.iter() {
        // Cull light probes outside the view frustum.
        view_light_probes.clear();
        view_light_probes.extend(
            light_probes
                .iter()
                .filter(|light_probe_info| light_probe_info.frustum_cull(view_frustum))
                .cloned(),
        );

        // Sort by distance to camera.
        view_light_probes.sort_by_cached_key(|light_probe_info| {
            light_probe_info.camera_distance_sort_key(view_transform)
        });

        // Create the light probes uniform.
        let (light_probes_uniform, render_view_environment_maps) =
            LightProbesUniform::build(view_environment_maps, &view_light_probes, &image_assets);

        // Record the uniforms.
        render_light_probes.insert(view_entity, light_probes_uniform);

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
    // Get the uniform buffer writer.
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
            smallest_specular_mip_level_for_view: 0,
        }
    }
}

impl LightProbesUniform {
    /// Constructs a [`LightProbesUniform`] containing all the environment maps
    /// that fragments rendered by a single view need to consider.
    ///
    /// The `view_environment_maps` parameter describes the environment maps
    /// attached to the view. The `light_probes` parameter is expected to be the
    /// list of light probes in the scene, sorted by increasing view distance
    /// from the camera.
    fn build(
        view_environment_maps: Option<&EnvironmentMapLight>,
        light_probes: &[LightProbeInfo],
        image_assets: &RenderAssets<Image>,
    ) -> (LightProbesUniform, RenderViewEnvironmentMaps) {
        let mut render_view_environment_maps = RenderViewEnvironmentMaps::new();

        // Find the index of the cubemap associated with the view, and determine
        // its smallest mip level.
        let (mut view_cubemap_index, mut smallest_specular_mip_level_for_view) = (-1, 0);
        if let Some(EnvironmentMapLight {
            diffuse_map: diffuse_map_handle,
            specular_map: specular_map_handle,
        }) = view_environment_maps
        {
            if let (Some(_), Some(specular_map)) = (
                image_assets.get(diffuse_map_handle),
                image_assets.get(specular_map_handle),
            ) {
                view_cubemap_index =
                    render_view_environment_maps.get_or_insert_cubemap(&EnvironmentMapIds {
                        diffuse: diffuse_map_handle.id(),
                        specular: specular_map_handle.id(),
                    }) as i32;
                smallest_specular_mip_level_for_view = specular_map.mip_level_count - 1;
            }
        };

        // Initialize the uniform to only contain the view environment map, if
        // applicable.
        let mut uniform = LightProbesUniform {
            reflection_probes: [RenderReflectionProbe::default(); MAX_VIEW_REFLECTION_PROBES],
            reflection_probe_count: light_probes.len().min(MAX_VIEW_REFLECTION_PROBES) as i32,
            view_cubemap_index,
            smallest_specular_mip_level_for_view,
        };

        // Add reflection probes from the scene, if supported by the current
        // platform.
        uniform.maybe_gather_reflection_probes(&mut render_view_environment_maps, light_probes);
        (uniform, render_view_environment_maps)
    }

    /// Gathers up all reflection probes in the scene and writes them into this
    /// uniform and `render_view_environment_maps`.
    #[cfg(all(not(feature = "shader_format_glsl"), not(target_arch = "wasm32")))]
    fn maybe_gather_reflection_probes(
        &mut self,
        render_view_environment_maps: &mut RenderViewEnvironmentMaps,
        light_probes: &[LightProbeInfo],
    ) {
        for (reflection_probe, light_probe) in self
            .reflection_probes
            .iter_mut()
            .zip(light_probes.iter().take(MAX_VIEW_REFLECTION_PROBES))
        {
            // Determine the index of the cubemap in the binding array.
            let cubemap_index = render_view_environment_maps
                .get_or_insert_cubemap(&light_probe.environment_maps)
                as i32;

            // Transpose the inverse transform to compress the structure on the
            // GPU (from 4 `Vec4`s to 3 `Vec4`s). The shader will transpose it
            // to recover the original inverse transform.
            let inverse_transpose_transform = light_probe.inverse_transform.transpose();

            // Write in the reflection probe data.
            *reflection_probe = RenderReflectionProbe {
                inverse_transpose_transform: [
                    inverse_transpose_transform.x_axis,
                    inverse_transpose_transform.y_axis,
                    inverse_transpose_transform.z_axis,
                ],
                cubemap_index,
            };
        }
    }

    /// This is the version of `maybe_gather_reflection_probes` used on
    /// platforms in which binding arrays aren't available. It's simply a no-op.
    #[cfg(any(feature = "shader_format_glsl", target_arch = "wasm32"))]
    fn maybe_gather_reflection_probes(
        &mut self,
        _: &mut RenderViewEnvironmentMaps,
        _: &[LightProbeInfo],
    ) {
    }
}

impl LightProbeInfo {
    /// Given the set of light probe components, constructs and returns
    /// [`LightProbeInfo`]. This is done for every light probe in the scene
    /// every frame.
    fn new(
        (light_probe_transform, environment_map): (&GlobalTransform, &EnvironmentMapLight),
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
                half_extents: Vec3A::splat(0.5),
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
