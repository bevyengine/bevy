//! Light probes for baked global illumination.

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, AssetId, Handle};
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
use bevy_math::{Affine3A, FloatOrd, Mat4, Vec3A, Vec4};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_instances::ExtractInstancesPlugin,
    primitives::{Aabb, Frustum},
    render_asset::RenderAssets,
    render_resource::{DynamicUniformBuffer, Sampler, Shader, ShaderType, TextureView},
    renderer::{RenderDevice, RenderQueue},
    settings::WgpuFeatures,
    texture::{FallbackImage, GpuImage, Image},
    view::ExtractedView,
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_transform::prelude::GlobalTransform;
use bevy_utils::{tracing::error, HashMap};

use std::hash::Hash;
use std::ops::Deref;

use crate::{
    irradiance_volume::IRRADIANCE_VOLUME_SHADER_HANDLE,
    light_probe::environment_map::{
        EnvironmentMapIds, EnvironmentMapLight, ENVIRONMENT_MAP_SHADER_HANDLE,
    },
};

use self::irradiance_volume::IrradianceVolume;

pub const LIGHT_PROBE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(8954249792581071582);

pub mod environment_map;
pub mod irradiance_volume;

/// The maximum number of each type of light probe that each view will consider.
///
/// Because the fragment shader does a linear search through the list for each
/// fragment, this number needs to be relatively small.
pub const MAX_VIEW_LIGHT_PROBES: usize = 8;

/// How many texture bindings are used in the fragment shader, *not* counting
/// environment maps or irradiance volumes.
const STANDARD_MATERIAL_FRAGMENT_SHADER_MIN_TEXTURE_BINDINGS: usize = 16;

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
/// kind of illumination, which can either be an [`EnvironmentMapLight`] or an
/// [`IrradianceVolume`].
///
/// When multiple sources of indirect illumination can be applied to a fragment,
/// the highest-quality one is chosen. Diffuse and specular illumination are
/// considered separately, so, for example, Bevy may decide to sample the
/// diffuse illumination from an irradiance volume and the specular illumination
/// from a reflection probe. From highest priority to lowest priority, the
/// ranking is as follows:
///
/// | Rank | Diffuse              | Specular             |
/// | ---- | -------------------- | -------------------- |
/// | 1    | Lightmap             | Lightmap             |
/// | 2    | Irradiance volume    | Reflection probe     |
/// | 3    | Reflection probe     | View environment map |
/// | 4    | View environment map |                      |
///
/// Note that ambient light is always added to the diffuse component and does
/// not participate in the ranking. That is, ambient light is applied in
/// addition to, not instead of, the light sources above.
///
/// A terminology note: Unfortunately, there is little agreement across game and
/// graphics engines as to what to call the various techniques that Bevy groups
/// under the term *light probe*. In Bevy, a *light probe* is the generic term
/// that encompasses both *reflection probes* and *irradiance volumes*. In
/// object-oriented terms, *light probe* is the superclass, and *reflection
/// probe* and *irradiance volume* are subclasses. In other engines, you may see
/// the term *light probe* refer to an irradiance volume with a single voxel, or
/// perhaps some other technique, while in Bevy *light probe* refers not to a
/// specific technique but rather to a class of techniques. Developers familiar
/// with other engines should be aware of this terminology difference.
#[derive(Component, Debug, Clone, Copy, Default, Reflect)]
#[reflect(Component, Default)]
pub struct LightProbe;

/// A GPU type that stores information about a light probe.
#[derive(Clone, Copy, ShaderType, Default)]
struct RenderLightProbe {
    /// The transform from the world space to the model space. This is used to
    /// efficiently check for bounding box intersection.
    light_from_world_transposed: [Vec4; 3],

    /// The index of the texture or textures in the appropriate binding array or
    /// arrays.
    ///
    /// For example, for reflection probes this is the index of the cubemap in
    /// the diffuse and specular texture arrays.
    texture_index: i32,

    /// Scale factor applied to the light generated by this light probe.
    ///
    /// See the comment in [`EnvironmentMapLight`] for details.
    intensity: f32,
}

/// A per-view shader uniform that specifies all the light probes that the view
/// takes into account.
#[derive(ShaderType)]
pub struct LightProbesUniform {
    /// The list of applicable reflection probes, sorted from nearest to the
    /// camera to the farthest away from the camera.
    reflection_probes: [RenderLightProbe; MAX_VIEW_LIGHT_PROBES],

    /// The list of applicable irradiance volumes, sorted from nearest to the
    /// camera to the farthest away from the camera.
    irradiance_volumes: [RenderLightProbe; MAX_VIEW_LIGHT_PROBES],

    /// The number of reflection probes in the list.
    reflection_probe_count: i32,

    /// The number of irradiance volumes in the list.
    irradiance_volume_count: i32,

    /// The index of the diffuse and specular environment maps associated with
    /// the view itself. This is used as a fallback if no reflection probe in
    /// the list contains the fragment.
    view_cubemap_index: i32,

    /// The smallest valid mipmap level for the specular environment cubemap
    /// associated with the view.
    smallest_specular_mip_level_for_view: u32,

    /// The intensity of the environment cubemap associated with the view.
    ///
    /// See the comment in [`EnvironmentMapLight`] for details.
    intensity_for_view: f32,
}

/// A GPU buffer that stores information about all light probes.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct LightProbesBuffer(DynamicUniformBuffer<LightProbesUniform>);

/// A component attached to each camera in the render world that stores the
/// index of the [`LightProbesUniform`] in the [`LightProbesBuffer`].
#[derive(Component, Default, Deref, DerefMut)]
pub struct ViewLightProbesUniformOffset(u32);

/// Information that [`gather_light_probes`] keeps about each light probe.
///
/// This information is parameterized by the [`LightProbeComponent`] type. This
/// will either be [`EnvironmentMapLight`] for reflection probes or
/// [`IrradianceVolume`] for irradiance volumes.
#[allow(dead_code)]
struct LightProbeInfo<C>
where
    C: LightProbeComponent,
{
    // The transform from world space to light probe space.
    light_from_world: Mat4,

    // The transform from light probe space to world space.
    world_from_light: Affine3A,

    // Scale factor applied to the diffuse and specular light generated by this
    // reflection probe.
    //
    // See the comment in [`EnvironmentMapLight`] for details.
    intensity: f32,

    // The IDs of all assets associated with this light probe.
    //
    // Because each type of light probe component may reference different types
    // of assets (e.g. a reflection probe references two cubemap assets while an
    // irradiance volume references a single 3D texture asset), this is generic.
    asset_id: C::AssetId,
}

/// A component, part of the render world, that stores the mapping from asset ID
/// or IDs to the texture index in the appropriate binding arrays.
///
/// Cubemap textures belonging to environment maps are collected into binding
/// arrays, and the index of each texture is presented to the shader for runtime
/// lookup. 3D textures belonging to reflection probes are likewise collected
/// into binding arrays, and the shader accesses the 3D texture by index.
///
/// This component is attached to each view in the render world, because each
/// view may have a different set of light probes that it considers and therefore
/// the texture indices are per-view.
#[derive(Component, Default)]
pub struct RenderViewLightProbes<C>
where
    C: LightProbeComponent,
{
    /// The list of environment maps presented to the shader, in order.
    binding_index_to_textures: Vec<C::AssetId>,

    /// The reverse of `binding_index_to_cubemap`: a map from the texture ID to
    /// the index in `binding_index_to_cubemap`.
    cubemap_to_binding_index: HashMap<C::AssetId, u32>,

    /// Information about each light probe, ready for upload to the GPU, sorted
    /// in order from closest to the camera to farthest.
    ///
    /// Note that this is not necessarily ordered by binding index. So don't
    /// write code like
    /// `render_light_probes[cubemap_to_binding_index[asset_id]]`; instead
    /// search for the light probe with the appropriate binding index in this
    /// array.
    render_light_probes: Vec<RenderLightProbe>,

    /// Information needed to render the light probe attached directly to the
    /// view, if applicable.
    ///
    /// A light probe attached directly to a view represents a "global" light
    /// probe that affects all objects not in the bounding region of any light
    /// probe. Currently, the only light probe type that supports this is the
    /// [`EnvironmentMapLight`].
    view_light_probe_info: C::ViewLightProbeInfo,
}

/// A trait implemented by all components that represent light probes.
///
/// Currently, the two light probe types are [`EnvironmentMapLight`] and
/// [`IrradianceVolume`], for reflection probes and irradiance volumes
/// respectively.
///
/// Most light probe systems are written to be generic over the type of light
/// probe. This allows much of the code to be shared and enables easy addition
/// of more light probe types (e.g. real-time reflection planes) in the future.
pub trait LightProbeComponent: Send + Sync + Component + Sized {
    /// Holds [`AssetId`]s of the texture or textures that this light probe
    /// references.
    ///
    /// This can just be [`AssetId`] if the light probe only references one
    /// texture. If it references multiple textures, it will be a structure
    /// containing those asset IDs.
    type AssetId: Send + Sync + Clone + Eq + Hash;

    /// If the light probe can be attached to the view itself (as opposed to a
    /// cuboid region within the scene), this contains the information that will
    /// be passed to the GPU in order to render it. Otherwise, this will be
    /// `()`.
    ///
    /// Currently, only reflection probes (i.e. [`EnvironmentMapLight`]) can be
    /// attached directly to views.
    type ViewLightProbeInfo: Send + Sync + Default;

    /// Returns the asset ID or asset IDs of the texture or textures referenced
    /// by this light probe.
    fn id(&self, image_assets: &RenderAssets<GpuImage>) -> Option<Self::AssetId>;

    /// Returns the intensity of this light probe.
    ///
    /// This is a scaling factor that will be multiplied by the value or values
    /// sampled from the texture.
    fn intensity(&self) -> f32;

    /// Creates an instance of [`RenderViewLightProbes`] containing all the
    /// information needed to render this light probe.
    ///
    /// This is called for every light probe in view every frame.
    fn create_render_view_light_probes(
        view_component: Option<&Self>,
        image_assets: &RenderAssets<GpuImage>,
    ) -> RenderViewLightProbes<Self>;
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
            LIGHT_PROBE_SHADER_HANDLE,
            "light_probe.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            ENVIRONMENT_MAP_SHADER_HANDLE,
            "environment_map.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            IRRADIANCE_VOLUME_SHADER_HANDLE,
            "irradiance_volume.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<LightProbe>()
            .register_type::<EnvironmentMapLight>()
            .register_type::<IrradianceVolume>();
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_plugins(ExtractInstancesPlugin::<EnvironmentMapIds>::new())
            .init_resource::<LightProbesBuffer>()
            .add_systems(ExtractSchedule, gather_light_probes::<EnvironmentMapLight>)
            .add_systems(ExtractSchedule, gather_light_probes::<IrradianceVolume>)
            .add_systems(
                Render,
                upload_light_probes.in_set(RenderSet::PrepareResources),
            );
    }
}

/// Gathers up all light probes of a single type in the scene and assigns them
/// to views, performing frustum culling and distance sorting in the process.
fn gather_light_probes<C>(
    image_assets: Res<RenderAssets<GpuImage>>,
    light_probe_query: Extract<Query<(&GlobalTransform, &C), With<LightProbe>>>,
    view_query: Extract<Query<(Entity, &GlobalTransform, &Frustum, Option<&C>), With<Camera3d>>>,
    mut reflection_probes: Local<Vec<LightProbeInfo<C>>>,
    mut view_reflection_probes: Local<Vec<LightProbeInfo<C>>>,
    mut commands: Commands,
) where
    C: LightProbeComponent,
{
    // Create [`LightProbeInfo`] for every light probe in the scene.
    reflection_probes.clear();
    reflection_probes.extend(
        light_probe_query
            .iter()
            .filter_map(|query_row| LightProbeInfo::new(query_row, &image_assets)),
    );

    // Build up the light probes uniform and the key table.
    for (view_entity, view_transform, view_frustum, view_component) in view_query.iter() {
        // Cull light probes outside the view frustum.
        view_reflection_probes.clear();
        view_reflection_probes.extend(
            reflection_probes
                .iter()
                .filter(|light_probe_info| light_probe_info.frustum_cull(view_frustum))
                .cloned(),
        );

        // Sort by distance to camera.
        view_reflection_probes.sort_by_cached_key(|light_probe_info| {
            light_probe_info.camera_distance_sort_key(view_transform)
        });

        // Create the light probes list.
        let mut render_view_light_probes =
            C::create_render_view_light_probes(view_component, &image_assets);

        // Gather up the light probes in the list.
        render_view_light_probes.maybe_gather_light_probes(&view_reflection_probes);

        // Record the per-view light probes.
        if render_view_light_probes.is_empty() {
            commands
                .get_or_spawn(view_entity)
                .remove::<RenderViewLightProbes<C>>();
        } else {
            commands
                .get_or_spawn(view_entity)
                .insert(render_view_light_probes);
        }
    }
}

// A system that runs after [`gather_light_probes`] and populates the GPU
// uniforms with the results.
//
// Note that, unlike [`gather_light_probes`], this system is not generic over
// the type of light probe. It collects light probes of all types together into
// a single structure, ready to be passed to the shader.
fn upload_light_probes(
    mut commands: Commands,
    views: Query<Entity, With<ExtractedView>>,
    mut light_probes_buffer: ResMut<LightProbesBuffer>,
    mut view_light_probes_query: Query<(
        Option<&RenderViewLightProbes<EnvironmentMapLight>>,
        Option<&RenderViewLightProbes<IrradianceVolume>>,
    )>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    // If there are no views, bail.
    if views.is_empty() {
        return;
    }

    // Initialize the uniform buffer writer.
    let mut writer = light_probes_buffer
        .get_writer(views.iter().len(), &render_device, &render_queue)
        .unwrap();

    // Process each view.
    for view_entity in views.iter() {
        let Ok((render_view_environment_maps, render_view_irradiance_volumes)) =
            view_light_probes_query.get_mut(view_entity)
        else {
            error!("Failed to find `RenderViewLightProbes` for the view!");
            continue;
        };

        // Initialize the uniform with only the view environment map, if there
        // is one.
        let mut light_probes_uniform = LightProbesUniform {
            reflection_probes: [RenderLightProbe::default(); MAX_VIEW_LIGHT_PROBES],
            irradiance_volumes: [RenderLightProbe::default(); MAX_VIEW_LIGHT_PROBES],
            reflection_probe_count: render_view_environment_maps
                .map(|maps| maps.len())
                .unwrap_or_default()
                .min(MAX_VIEW_LIGHT_PROBES) as i32,
            irradiance_volume_count: render_view_irradiance_volumes
                .map(|maps| maps.len())
                .unwrap_or_default()
                .min(MAX_VIEW_LIGHT_PROBES) as i32,
            view_cubemap_index: render_view_environment_maps
                .map(|maps| maps.view_light_probe_info.cubemap_index)
                .unwrap_or(-1),
            smallest_specular_mip_level_for_view: render_view_environment_maps
                .map(|maps| maps.view_light_probe_info.smallest_specular_mip_level)
                .unwrap_or(0),
            intensity_for_view: render_view_environment_maps
                .map(|maps| maps.view_light_probe_info.intensity)
                .unwrap_or(1.0),
        };

        // Add any environment maps that [`gather_light_probes`] found to the
        // uniform.
        if let Some(render_view_environment_maps) = render_view_environment_maps {
            render_view_environment_maps.add_to_uniform(
                &mut light_probes_uniform.reflection_probes,
                &mut light_probes_uniform.reflection_probe_count,
            );
        }

        // Add any irradiance volumes that [`gather_light_probes`] found to the
        // uniform.
        if let Some(render_view_irradiance_volumes) = render_view_irradiance_volumes {
            render_view_irradiance_volumes.add_to_uniform(
                &mut light_probes_uniform.irradiance_volumes,
                &mut light_probes_uniform.irradiance_volume_count,
            );
        }

        // Queue the view's uniforms to be written to the GPU.
        let uniform_offset = writer.write(&light_probes_uniform);

        commands
            .entity(view_entity)
            .insert(ViewLightProbesUniformOffset(uniform_offset));
    }
}

impl Default for LightProbesUniform {
    fn default() -> Self {
        Self {
            reflection_probes: [RenderLightProbe::default(); MAX_VIEW_LIGHT_PROBES],
            irradiance_volumes: [RenderLightProbe::default(); MAX_VIEW_LIGHT_PROBES],
            reflection_probe_count: 0,
            irradiance_volume_count: 0,
            view_cubemap_index: -1,
            smallest_specular_mip_level_for_view: 0,
            intensity_for_view: 1.0,
        }
    }
}

impl<C> LightProbeInfo<C>
where
    C: LightProbeComponent,
{
    /// Given the set of light probe components, constructs and returns
    /// [`LightProbeInfo`]. This is done for every light probe in the scene
    /// every frame.
    fn new(
        (light_probe_transform, environment_map): (&GlobalTransform, &C),
        image_assets: &RenderAssets<GpuImage>,
    ) -> Option<LightProbeInfo<C>> {
        environment_map.id(image_assets).map(|id| LightProbeInfo {
            world_from_light: light_probe_transform.affine(),
            light_from_world: light_probe_transform.compute_matrix().inverse(),
            asset_id: id,
            intensity: environment_map.intensity(),
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
            &self.world_from_light,
            true,
            false,
        )
    }

    /// Returns the squared distance from this light probe to the camera,
    /// suitable for distance sorting.
    fn camera_distance_sort_key(&self, view_transform: &GlobalTransform) -> FloatOrd {
        FloatOrd(
            (self.world_from_light.translation - view_transform.translation_vec3a())
                .length_squared(),
        )
    }
}

impl<C> RenderViewLightProbes<C>
where
    C: LightProbeComponent,
{
    /// Creates a new empty list of light probes.
    fn new() -> RenderViewLightProbes<C> {
        RenderViewLightProbes {
            binding_index_to_textures: vec![],
            cubemap_to_binding_index: HashMap::new(),
            render_light_probes: vec![],
            view_light_probe_info: C::ViewLightProbeInfo::default(),
        }
    }

    /// Returns true if there are no light probes in the list.
    pub(crate) fn is_empty(&self) -> bool {
        self.binding_index_to_textures.is_empty()
    }

    /// Returns the number of light probes in the list.
    pub(crate) fn len(&self) -> usize {
        self.binding_index_to_textures.len()
    }

    /// Adds a cubemap to the list of bindings, if it wasn't there already, and
    /// returns its index within that list.
    pub(crate) fn get_or_insert_cubemap(&mut self, cubemap_id: &C::AssetId) -> u32 {
        *self
            .cubemap_to_binding_index
            .entry((*cubemap_id).clone())
            .or_insert_with(|| {
                let index = self.binding_index_to_textures.len() as u32;
                self.binding_index_to_textures.push((*cubemap_id).clone());
                index
            })
    }

    /// Adds all the light probes in this structure to the supplied array, which
    /// is expected to be shipped to the GPU.
    fn add_to_uniform(
        &self,
        render_light_probes: &mut [RenderLightProbe; MAX_VIEW_LIGHT_PROBES],
        render_light_probe_count: &mut i32,
    ) {
        render_light_probes[0..self.render_light_probes.len()]
            .copy_from_slice(&self.render_light_probes[..]);
        *render_light_probe_count = self.render_light_probes.len() as i32;
    }

    /// Gathers up all light probes of the given type in the scene and records
    /// them in this structure.
    fn maybe_gather_light_probes(&mut self, light_probes: &[LightProbeInfo<C>]) {
        for light_probe in light_probes.iter().take(MAX_VIEW_LIGHT_PROBES) {
            // Determine the index of the cubemap in the binding array.
            let cubemap_index = self.get_or_insert_cubemap(&light_probe.asset_id);

            // Transpose the inverse transform to compress the structure on the
            // GPU (from 4 `Vec4`s to 3 `Vec4`s). The shader will transpose it
            // to recover the original inverse transform.
            let light_from_world_transposed = light_probe.light_from_world.transpose();

            // Write in the light probe data.
            self.render_light_probes.push(RenderLightProbe {
                light_from_world_transposed: [
                    light_from_world_transposed.x_axis,
                    light_from_world_transposed.y_axis,
                    light_from_world_transposed.z_axis,
                ],
                texture_index: cubemap_index as i32,
                intensity: light_probe.intensity,
            });
        }
    }
}

impl<C> Clone for LightProbeInfo<C>
where
    C: LightProbeComponent,
{
    fn clone(&self) -> Self {
        Self {
            light_from_world: self.light_from_world,
            world_from_light: self.world_from_light,
            intensity: self.intensity,
            asset_id: self.asset_id.clone(),
        }
    }
}

/// Adds a diffuse or specular texture view to the `texture_views` list, and
/// populates `sampler` if this is the first such view.
pub(crate) fn add_cubemap_texture_view<'a>(
    texture_views: &mut Vec<&'a <TextureView as Deref>::Target>,
    sampler: &mut Option<&'a Sampler>,
    image_id: AssetId<Image>,
    images: &'a RenderAssets<GpuImage>,
    fallback_image: &'a FallbackImage,
) {
    match images.get(image_id) {
        None => {
            // Use the fallback image if the cubemap isn't loaded yet.
            texture_views.push(&*fallback_image.cube.texture_view);
        }
        Some(image) => {
            // If this is the first texture view, populate `sampler`.
            if sampler.is_none() {
                *sampler = Some(&image.sampler);
            }

            texture_views.push(&*image.texture_view);
        }
    }
}

/// Many things can go wrong when attempting to use texture binding arrays
/// (a.k.a. bindless textures). This function checks for these pitfalls:
///
/// 1. If GLSL support is enabled at the feature level, then in debug mode
///     `naga_oil` will attempt to compile all shader modules under GLSL to check
///     validity of names, even if GLSL isn't actually used. This will cause a crash
///     if binding arrays are enabled, because binding arrays are currently
///     unimplemented in the GLSL backend of Naga. Therefore, we disable binding
///     arrays if the `shader_format_glsl` feature is present.
///
/// 2. If there aren't enough texture bindings available to accommodate all the
///     binding arrays, the driver will panic. So we also bail out if there aren't
///     enough texture bindings available in the fragment shader.
///
/// 3. If binding arrays aren't supported on the hardware, then we obviously
///     can't use them.
///
/// 4. If binding arrays are supported on the hardware, but they can only be
///     accessed by uniform indices, that's not good enough, and we bail out.
///
/// If binding arrays aren't usable, we disable reflection probes and limit the
/// number of irradiance volumes in the scene to 1.
pub(crate) fn binding_arrays_are_usable(render_device: &RenderDevice) -> bool {
    !cfg!(feature = "shader_format_glsl")
        && render_device.limits().max_storage_textures_per_shader_stage
            >= (STANDARD_MATERIAL_FRAGMENT_SHADER_MIN_TEXTURE_BINDINGS + MAX_VIEW_LIGHT_PROBES)
                as u32
        && render_device.features().contains(
            WgpuFeatures::TEXTURE_BINDING_ARRAY
                | WgpuFeatures::SAMPLED_TEXTURE_AND_STORAGE_BUFFER_ARRAY_NON_UNIFORM_INDEXING,
        )
}
