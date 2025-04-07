use alloc::{collections::BTreeMap, sync::Arc};
use bevy_tasks::{ComputeTaskPool, TaskPool};
use core::{array, error::Error, fmt::Display, num::NonZero, ops::Deref};

use bevy_core_pipeline::{
    core_3d::ViewTransmissionTexture,
    oit::{resolve::is_oit_supported, OitBuffers, OrderIndependentTransparencySettings},
    prepass::ViewPrepassTextures,
    tonemapping::{
        get_lut_bind_group_layout_entries, get_lut_bindings, Tonemapping, TonemappingLuts,
    },
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    name::Name,
    query::With,
    resource::Resource,
    system::{Commands, Query, Res},
    world::{FromWorld, World},
};
use bevy_math::Vec4;
use bevy_platform_support::collections::{hash_map::Entry, HashMap};
#[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
use bevy_render::render_resource::binding_types::texture_cube;
use bevy_render::{
    globals::{GlobalsBuffer, GlobalsUniform},
    render_asset::RenderAssets,
    render_resource::{binding_types::*, *},
    renderer::{RenderAdapter, RenderDevice},
    texture::{FallbackImage, FallbackImageZero, GpuImage},
    view::{
        ExtractedView, Msaa, RenderVisibilityRanges, ViewUniform, ViewUniforms,
        VISIBILITY_RANGES_STORAGE_BUFFER_COUNT,
    },
};

use crate::{
    decal::{
        self,
        clustered::{
            DecalsBuffer, RenderClusteredDecals, RenderViewClusteredDecalBindGroupEntries,
        },
    },
    environment_map::{self, RenderViewEnvironmentMapBindGroupEntries},
    irradiance_volume::{
        self, IrradianceVolume, RenderViewIrradianceVolumeBindGroupEntries,
        IRRADIANCE_VOLUMES_ARE_USABLE,
    },
    prepass, EnvironmentMapUniformBuffer, FogMeta, GlobalClusterableObjectMeta,
    GpuClusterableObjects, GpuFog, GpuLights, LightMeta, LightProbesBuffer, LightProbesUniform,
    MeshPipeline, MeshPipelineKey, RenderViewLightProbes, ScreenSpaceAmbientOcclusionFallbackImage,
    ScreenSpaceAmbientOcclusionResources, ScreenSpaceReflectionsBuffer,
    ScreenSpaceReflectionsUniform, ShadowSamplers, ViewClusterBindings, ViewShadowBindings,
    CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT,
};

#[cfg(debug_assertions)]
use {crate::MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES, bevy_utils::once, tracing::warn};

use environment_map::EnvironmentMapLight;

#[derive(Clone)]
pub struct MeshPipelineViewLayout {
    pub bind_group_layout: BindGroupLayout,

    #[cfg(debug_assertions)]
    pub texture_count: usize,
}

bitflags::bitflags! {
    /// A key that uniquely identifies a [`MeshPipelineViewLayout`].
    ///
    /// Used to generate all possible layouts for the mesh pipeline in [`generate_view_layouts`],
    /// so special care must be taken to not add too many flags, as the number of possible layouts
    /// will grow exponentially.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    #[repr(transparent)]
    pub struct MeshPipelineViewLayoutKey: u32 {
        const MULTISAMPLED                = 1 << 0;
        const DEPTH_PREPASS               = 1 << 1;
        const NORMAL_PREPASS              = 1 << 2;
        const MOTION_VECTOR_PREPASS       = 1 << 3;
        const DEFERRED_PREPASS            = 1 << 4;
        const OIT_ENABLED                 = 1 << 5;
    }
}

impl MeshPipelineViewLayoutKey {
    // The number of possible layouts
    pub const COUNT: usize = Self::all().bits() as usize + 1;

    /// Builds a unique label for each layout based on the flags
    pub fn label(&self) -> String {
        use MeshPipelineViewLayoutKey as Key;

        format!(
            "mesh_view_layout{}{}{}{}{}{}",
            self.contains(Key::MULTISAMPLED)
                .then_some("_multisampled")
                .unwrap_or_default(),
            self.contains(Key::DEPTH_PREPASS)
                .then_some("_depth")
                .unwrap_or_default(),
            self.contains(Key::NORMAL_PREPASS)
                .then_some("_normal")
                .unwrap_or_default(),
            self.contains(Key::MOTION_VECTOR_PREPASS)
                .then_some("_motion")
                .unwrap_or_default(),
            self.contains(Key::DEFERRED_PREPASS)
                .then_some("_deferred")
                .unwrap_or_default(),
            self.contains(Key::OIT_ENABLED)
                .then_some("_oit")
                .unwrap_or_default(),
        )
    }
}

impl From<MeshPipelineKey> for MeshPipelineViewLayoutKey {
    fn from(value: MeshPipelineKey) -> Self {
        let mut result = MeshPipelineViewLayoutKey::empty();

        if value.msaa_samples() > 1 {
            result |= MeshPipelineViewLayoutKey::MULTISAMPLED;
        }
        if value.contains(MeshPipelineKey::DEPTH_PREPASS) {
            result |= MeshPipelineViewLayoutKey::DEPTH_PREPASS;
        }
        if value.contains(MeshPipelineKey::NORMAL_PREPASS) {
            result |= MeshPipelineViewLayoutKey::NORMAL_PREPASS;
        }
        if value.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
            result |= MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS;
        }
        if value.contains(MeshPipelineKey::DEFERRED_PREPASS) {
            result |= MeshPipelineViewLayoutKey::DEFERRED_PREPASS;
        }
        if value.contains(MeshPipelineKey::OIT_ENABLED) {
            result |= MeshPipelineViewLayoutKey::OIT_ENABLED;
        }

        result
    }
}

impl From<Msaa> for MeshPipelineViewLayoutKey {
    fn from(value: Msaa) -> Self {
        let mut result = MeshPipelineViewLayoutKey::empty();

        if value.samples() > 1 {
            result |= MeshPipelineViewLayoutKey::MULTISAMPLED;
        }

        result
    }
}

impl From<Option<&ViewPrepassTextures>> for MeshPipelineViewLayoutKey {
    fn from(value: Option<&ViewPrepassTextures>) -> Self {
        let mut result = MeshPipelineViewLayoutKey::empty();

        if let Some(prepass_textures) = value {
            if prepass_textures.depth.is_some() {
                result |= MeshPipelineViewLayoutKey::DEPTH_PREPASS;
            }
            if prepass_textures.normal.is_some() {
                result |= MeshPipelineViewLayoutKey::NORMAL_PREPASS;
            }
            if prepass_textures.motion_vectors.is_some() {
                result |= MeshPipelineViewLayoutKey::MOTION_VECTOR_PREPASS;
            }
            if prepass_textures.deferred.is_some() {
                result |= MeshPipelineViewLayoutKey::DEFERRED_PREPASS;
            }
        }

        result
    }
}

pub(crate) fn buffer_layout(
    buffer_binding_type: BufferBindingType,
    has_dynamic_offset: bool,
    min_binding_size: Option<NonZero<u64>>,
) -> BindGroupLayoutEntryBuilder {
    match buffer_binding_type {
        BufferBindingType::Uniform => uniform_buffer_sized(has_dynamic_offset, min_binding_size),
        BufferBindingType::Storage { read_only } => {
            if read_only {
                storage_buffer_read_only_sized(has_dynamic_offset, min_binding_size)
            } else {
                storage_buffer_sized(has_dynamic_offset, min_binding_size)
            }
        }
    }
}

/// Returns the appropriate bind group layout vec based on the parameters
fn layout_entries(
    clustered_forward_buffer_binding_type: BufferBindingType,
    visibility_ranges_buffer_binding_type: BufferBindingType,
    layout_key: MeshPipelineViewLayoutKey,
    render_device: &RenderDevice,
    render_adapter: &RenderAdapter,
) -> Vec<BindGroupLayoutEntry> {
    let mut entries = DynamicBindGroupLayoutEntries::new_with_indices(
        ShaderStages::FRAGMENT,
        (
            // View
            (
                0,
                uniform_buffer::<ViewUniform>(true).visibility(ShaderStages::VERTEX_FRAGMENT),
            ),
            // Lights
            (1, uniform_buffer::<GpuLights>(true)),
            // Point Shadow Texture Cube Array
            (
                2,
                #[cfg(all(
                    not(target_abi = "sim"),
                    any(
                        not(feature = "webgl"),
                        not(target_arch = "wasm32"),
                        feature = "webgpu"
                    )
                ))]
                texture_cube_array(TextureSampleType::Depth),
                #[cfg(any(
                    target_abi = "sim",
                    all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu"))
                ))]
                texture_cube(TextureSampleType::Depth),
            ),
            // Point Shadow Texture Array Comparison Sampler
            (3, sampler(SamplerBindingType::Comparison)),
            // Point Shadow Texture Array Linear Sampler
            #[cfg(feature = "experimental_pbr_pcss")]
            (4, sampler(SamplerBindingType::Filtering)),
            // Directional Shadow Texture Array
            (
                5,
                #[cfg(any(
                    not(feature = "webgl"),
                    not(target_arch = "wasm32"),
                    feature = "webgpu"
                ))]
                texture_2d_array(TextureSampleType::Depth),
                #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
                texture_2d(TextureSampleType::Depth),
            ),
            // Directional Shadow Texture Array Comparison Sampler
            (6, sampler(SamplerBindingType::Comparison)),
            // Directional Shadow Texture Array Linear Sampler
            #[cfg(feature = "experimental_pbr_pcss")]
            (7, sampler(SamplerBindingType::Filtering)),
            // PointLights
            (
                8,
                buffer_layout(
                    clustered_forward_buffer_binding_type,
                    false,
                    Some(GpuClusterableObjects::min_size(
                        clustered_forward_buffer_binding_type,
                    )),
                ),
            ),
            // ClusteredLightIndexLists
            (
                9,
                buffer_layout(
                    clustered_forward_buffer_binding_type,
                    false,
                    Some(
                        ViewClusterBindings::min_size_clusterable_object_index_lists(
                            clustered_forward_buffer_binding_type,
                        ),
                    ),
                ),
            ),
            // ClusterOffsetsAndCounts
            (
                10,
                buffer_layout(
                    clustered_forward_buffer_binding_type,
                    false,
                    Some(ViewClusterBindings::min_size_cluster_offsets_and_counts(
                        clustered_forward_buffer_binding_type,
                    )),
                ),
            ),
            // Globals
            (
                11,
                uniform_buffer::<GlobalsUniform>(false).visibility(ShaderStages::VERTEX_FRAGMENT),
            ),
            // Fog
            (12, uniform_buffer::<GpuFog>(true)),
            // Light probes
            (13, uniform_buffer::<LightProbesUniform>(true)),
            // Visibility ranges
            (
                14,
                buffer_layout(
                    visibility_ranges_buffer_binding_type,
                    false,
                    Some(Vec4::min_size()),
                )
                .visibility(ShaderStages::VERTEX),
            ),
            // Screen space reflection settings
            (15, uniform_buffer::<ScreenSpaceReflectionsUniform>(true)),
            // Screen space ambient occlusion texture
            (
                16,
                texture_2d(TextureSampleType::Float { filterable: false }),
            ),
        ),
    );

    // EnvironmentMapLight
    let environment_map_entries =
        environment_map::get_bind_group_layout_entries(render_device, render_adapter);
    entries = entries.extend_with_indices((
        (17, environment_map_entries[0]),
        (18, environment_map_entries[1]),
        (19, environment_map_entries[2]),
        (20, environment_map_entries[3]),
    ));

    // Irradiance volumes
    if IRRADIANCE_VOLUMES_ARE_USABLE {
        let irradiance_volume_entries =
            irradiance_volume::get_bind_group_layout_entries(render_device, render_adapter);
        entries = entries.extend_with_indices((
            (21, irradiance_volume_entries[0]),
            (22, irradiance_volume_entries[1]),
        ));
    }

    // Clustered decals
    if let Some(clustered_decal_entries) =
        decal::clustered::get_bind_group_layout_entries(render_device, render_adapter)
    {
        entries = entries.extend_with_indices((
            (23, clustered_decal_entries[0]),
            (24, clustered_decal_entries[1]),
            (25, clustered_decal_entries[2]),
        ));
    }

    // Tonemapping
    let tonemapping_lut_entries = get_lut_bind_group_layout_entries();
    entries = entries.extend_with_indices((
        (26, tonemapping_lut_entries[0]),
        (27, tonemapping_lut_entries[1]),
    ));

    // Prepass
    if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32")))
        || (cfg!(all(feature = "webgl", target_arch = "wasm32"))
            && !layout_key.contains(MeshPipelineViewLayoutKey::MULTISAMPLED))
    {
        for (entry, binding) in prepass::get_bind_group_layout_entries(layout_key)
            .iter()
            .zip([28, 29, 30, 31])
        {
            if let Some(entry) = entry {
                entries = entries.extend_with_indices(((binding as u32, *entry),));
            }
        }
    }

    // View Transmission Texture
    entries = entries.extend_with_indices((
        (
            32,
            texture_2d(TextureSampleType::Float { filterable: true }),
        ),
        (33, sampler(SamplerBindingType::Filtering)),
    ));

    // OIT
    if layout_key.contains(MeshPipelineViewLayoutKey::OIT_ENABLED) {
        // Check if we can use OIT. This is a hack to avoid errors on webgl --
        // the OIT plugin will warn the user that OIT is not supported on their
        // platform, so we don't need to do it here.
        if is_oit_supported(render_adapter, render_device, false) {
            entries = entries.extend_with_indices((
                // oit_layers
                (34, storage_buffer_sized(false, None)),
                // oit_layer_ids,
                (35, storage_buffer_sized(false, None)),
                // oit_layer_count
                (
                    36,
                    uniform_buffer::<OrderIndependentTransparencySettings>(true),
                ),
            ));
        }
    }

    entries.to_vec()
}

/// Stores the view layouts for every combination of pipeline keys.
///
/// This is wrapped in an [`Arc`] so that it can be efficiently cloned and
/// placed inside specializable pipeline types.
#[derive(Resource, Clone, Deref, DerefMut)]
pub struct MeshPipelineViewLayouts(
    pub Arc<[MeshPipelineViewLayout; MeshPipelineViewLayoutKey::COUNT]>,
);

impl FromWorld for MeshPipelineViewLayouts {
    fn from_world(world: &mut World) -> Self {
        // Generates all possible view layouts for the mesh pipeline, based on all combinations of
        // [`MeshPipelineViewLayoutKey`] flags.

        let render_device = world.resource::<RenderDevice>();
        let render_adapter = world.resource::<RenderAdapter>();

        let clustered_forward_buffer_binding_type = render_device
            .get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT);
        let visibility_ranges_buffer_binding_type = render_device
            .get_supported_read_only_binding_type(VISIBILITY_RANGES_STORAGE_BUFFER_COUNT);

        Self(Arc::new(array::from_fn(|i| {
            let key = MeshPipelineViewLayoutKey::from_bits_truncate(i as u32);
            let entries = layout_entries(
                clustered_forward_buffer_binding_type,
                visibility_ranges_buffer_binding_type,
                key,
                render_device,
                render_adapter,
            );
            #[cfg(debug_assertions)]
            let texture_count: usize = entries
                .iter()
                .filter(|entry| matches!(entry.ty, BindingType::Texture { .. }))
                .count();

            MeshPipelineViewLayout {
                bind_group_layout: render_device
                    .create_bind_group_layout(key.label().as_str(), &entries),
                #[cfg(debug_assertions)]
                texture_count,
            }
        })))
    }
}

impl MeshPipelineViewLayouts {
    pub fn get_view_layout(&self, layout_key: MeshPipelineViewLayoutKey) -> &BindGroupLayout {
        let index = layout_key.bits() as usize;
        let layout = &self[index];

        #[cfg(debug_assertions)]
        if layout.texture_count > MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES {
            // Issue our own warning here because Naga's error message is a bit cryptic in this situation
            once!(warn!("Too many textures in mesh pipeline view layout, this might cause us to hit `wgpu::Limits::max_sampled_textures_per_shader_stage` in some environments."));
        }

        &layout.bind_group_layout
    }
}

/// Generates all possible view layouts for the mesh pipeline, based on all combinations of
/// [`MeshPipelineViewLayoutKey`] flags.
pub fn generate_view_layouts(
    render_device: &RenderDevice,
    render_adapter: &RenderAdapter,
    clustered_forward_buffer_binding_type: BufferBindingType,
    visibility_ranges_buffer_binding_type: BufferBindingType,
) -> [MeshPipelineViewLayout; MeshPipelineViewLayoutKey::COUNT] {
    array::from_fn(|i| {
        let key = MeshPipelineViewLayoutKey::from_bits_truncate(i as u32);
        let entries = layout_entries(
            clustered_forward_buffer_binding_type,
            visibility_ranges_buffer_binding_type,
            key,
            render_device,
            render_adapter,
        );

        #[cfg(debug_assertions)]
        let texture_count: usize = entries
            .iter()
            .filter(|entry| matches!(entry.ty, BindingType::Texture { .. }))
            .count();

        MeshPipelineViewLayout {
            bind_group_layout: render_device
                .create_bind_group_layout(key.label().as_str(), &entries),
            #[cfg(debug_assertions)]
            texture_count,
        }
    })
}

#[derive(Component)]
pub struct MeshViewBindGroup {
    pub value: BindGroup,
}

pub type MeshViewBindGroupFetcher =
    for<'b> fn(
        &'b World,
        Entity,
    ) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError>;

#[derive(Debug, PartialEq, Eq)]
pub enum MeshViewBindGroupFetchError {
    // TODO add binding
    Missing(&'static str),
    Skipped,
}

impl Display for MeshViewBindGroupFetchError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::Missing(resource_or_component) => write!(
                f,
                "Binding group could not be built because could not fetch {}.",
                resource_or_component
            ),
            Self::Skipped => write!(f, "Binding can be skipped."),
        }
    }
}

impl Error for MeshViewBindGroupFetchError {}

pub enum WrappedBindingResource<'b> {
    BindingResource(BindingResource<'b>),
    OwnedTextureView(TextureView),
    OwnedTextureViewArray(Vec<&'b <TextureView as Deref>::Target>),
}

impl<'b> From<BindingResource<'b>> for WrappedBindingResource<'b> {
    fn from(value: BindingResource<'b>) -> Self {
        Self::BindingResource(value)
    }
}

#[derive(Resource)]
pub struct MeshViewBindGroupSources {
    layout_key: Vec<fn(&World, Entity) -> MeshPipelineViewLayoutKey>,
    fetchers: HashMap<u32, MeshViewBindGroupFetcher>,
}

impl MeshViewBindGroupSources {
    pub fn new() -> Self {
        Self {
            layout_key: Vec::new(),
            fetchers: HashMap::default(),
        }
    }

    pub fn push_key(&mut self, layout_key: fn(&World, Entity) -> MeshPipelineViewLayoutKey) {
        self.layout_key.push(layout_key);
    }

    pub fn push_source(
        &mut self,
        binding: u32,
        fetcher: MeshViewBindGroupFetcher,
    ) -> Result<(), BindingAlreadyInUse> {
        match self.fetchers.entry(binding) {
            Entry::Vacant(key) => {
                key.insert(fetcher);
                Ok(())
            }
            Entry::Occupied(_) => Err(BindingAlreadyInUse(binding)),
        }
    }
}

impl Default for MeshViewBindGroupSources {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct BindingAlreadyInUse(u32);

impl Display for BindingAlreadyInUse {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Binding {} already in use.", self.0)
    }
}

impl Error for BindingAlreadyInUse {}

pub fn prepare_mesh_view_bind_groups(
    mut commands: Commands,
    world: &World,
    views: Query<Entity, With<ExtractedView>>,
    names: Query<&Name, With<ExtractedView>>,
    sources: Res<MeshViewBindGroupSources>,
    mesh_pipeline: Res<MeshPipeline>,
    render_device: Res<RenderDevice>,
) {
    let task_pool = ComputeTaskPool::get_or_init(TaskPool::default);
    commands.insert_batch(
        task_pool
            .scope(|scope| {
                for view in views {
                    let mesh_pipeline_ref = &mesh_pipeline;
                    let names_ref = &names;
                    let sources_ref = &sources;
                    let render_device_ref = &render_device;

                    scope.spawn(async move {
                        let layout_key = sources_ref
                            .layout_key
                            .iter()
                            .map(|key_source| key_source(world, view))
                            .reduce(|key, cur| key | cur)
                            .unwrap_or_else(MeshPipelineViewLayoutKey::empty);
                        let layout = mesh_pipeline_ref.get_view_layout(layout_key);

                        let required = sources_ref
                            .fetchers
                            .iter()
                            .map(|(binding, fetcher)| {
                                fetcher(world, view).map(|resource| (*binding, resource))
                            })
                            .filter(|res| !matches!(res, Err(MeshViewBindGroupFetchError::Skipped)))
                            .collect::<Result<Vec<_>, _>>()
                            .inspect_err(|err| {
                                tracing::error!(
                                    "{}: {}",
                                    names_ref
                                        .get(view)
                                        .map(ToString::to_string)
                                        .unwrap_or(view.to_string()),
                                    err
                                );
                            })
                            .ok()?;

                        // BTreeMap because it already sorts keys
                        let mut entries = BTreeMap::new();
                        for (binding, resource) in &required {
                            let br = match resource {
                                WrappedBindingResource::BindingResource(br) => br.clone(),
                                WrappedBindingResource::OwnedTextureView(tv) => tv.into_binding(),
                                WrappedBindingResource::OwnedTextureViewArray(v) => {
                                    v.into_binding()
                                }
                            };
                            entries.insert(*binding, br);
                        }

                        let entries = entries.into_iter().fold(
                            DynamicBindGroupEntries::default(),
                            |mut entries, (binding, resource)| {
                                entries.push(binding, resource);
                                entries
                            },
                        );

                        Some((
                            view,
                            MeshViewBindGroup {
                                value: render_device_ref.create_bind_group(
                                    "mesh_view_bind_group",
                                    layout,
                                    &entries,
                                ),
                            },
                        ))
                    });
                }
            })
            .into_iter()
            .flatten(),
    );
}

pub(super) fn set_msaa_mesh_pipeline_view_layout_key(
    world: &World,
    view: Entity,
) -> MeshPipelineViewLayoutKey {
    world
        .entity(view)
        .get::<Msaa>()
        .map(|msaa| MeshPipelineViewLayoutKey::from(*msaa))
        .unwrap_or_else(MeshPipelineViewLayoutKey::empty)
}

pub(super) fn set_prepass_mesh_pipeline_view_layout_key(
    world: &World,
    view: Entity,
) -> MeshPipelineViewLayoutKey {
    let prepass_textures = world.entity(view).get::<ViewPrepassTextures>();
    MeshPipelineViewLayoutKey::from(prepass_textures)
}

pub(super) fn set_oit_mesh_pipeline_view_layout_key(
    world: &World,
    view: Entity,
) -> MeshPipelineViewLayoutKey {
    let has_oit = world
        .entity(view)
        .contains::<OrderIndependentTransparencySettings>();
    if has_oit {
        MeshPipelineViewLayoutKey::OIT_ENABLED
    } else {
        MeshPipelineViewLayoutKey::empty()
    }
}

pub(super) fn fetch_view_uniforms_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<ViewUniforms>()
        .and_then(|view_uniforms| view_uniforms.uniforms.binding())
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("ViewUniforms"))
}

pub(super) fn fetch_environment_map_uniform_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<EnvironmentMapUniformBuffer>()
        .and_then(|environment_map_uniform| environment_map_uniform.binding())
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing(
            "EnvironmentMapUniformBuffer",
        ))
}

pub(super) fn fetch_light_meta_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<LightMeta>()
        .and_then(|light_meta| light_meta.view_gpu_lights.binding())
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("LightMeta"))
}

pub(super) fn fetch_global_light_meta_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<GlobalClusterableObjectMeta>()
        .and_then(|global_light_meta| global_light_meta.gpu_clusterable_objects.binding())
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing(
            "GlobalClusterableObjectMeta",
        ))
}

pub(super) fn fetch_global_buffers_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<GlobalsBuffer>()
        .and_then(|globals_buffer| globals_buffer.buffer.binding())
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("GlobalsBuffer"))
}

pub(super) fn fetch_fog_meta_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<FogMeta>()
        .and_then(|fog_meta| fog_meta.gpu_fogs.binding())
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("FogMeta"))
}

pub(super) fn fetch_light_probes_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<LightProbesBuffer>()
        .and_then(|light_probes| light_probes.binding())
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("LightProbesBuffer"))
}

pub(super) fn fetch_visibility_ranges_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<RenderVisibilityRanges>()
        .and_then(|visibility_ranges| {
            visibility_ranges
                .buffer()
                .buffer()
                .map(|buffer| buffer.as_entire_binding())
        })
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing(
            "RenderVisibilityRanges",
        ))
}

pub(super) fn fetch_screen_space_reflection_buffer_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<ScreenSpaceReflectionsBuffer>()
        .and_then(|ssr_buffer| ssr_buffer.binding())
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing(
            "ScreenSpaceReflectionsBuffer",
        ))
}

pub(super) fn fetch_ssao_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let ssao_resources = world
        .entity(view)
        .get::<ScreenSpaceAmbientOcclusionResources>();
    let ssao_view = ssao_resources
        .map(|t| {
            t.screen_space_ambient_occlusion_texture
                .default_view
                .into_binding()
        })
        .unwrap_or_else(|| {
            let fallback_image = world.resource::<ScreenSpaceAmbientOcclusionFallbackImage>();
            fallback_image.into_binding()
        });

    Ok(ssao_view.into())
}

pub(super) fn fetch_shadow_binding_point_light_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .entity(view)
        .get::<ViewShadowBindings>()
        .map(|view_shadow_bindings| {
            view_shadow_bindings
                .point_light_depth_texture_view
                .into_binding()
        })
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("ViewShadowBindings"))
}

pub(super) fn fetch_shadow_binding_directional_light_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .entity(view)
        .get::<ViewShadowBindings>()
        .map(|view_shadow_bindings| {
            view_shadow_bindings
                .directional_light_depth_texture_view
                .into_binding()
        })
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("ViewShadowBindings"))
}

pub(super) fn fetch_shadow_sampler_point_light_comparison_sampler_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<ShadowSamplers>()
        .map(|shadow_samplers| {
            shadow_samplers
                .point_light_comparison_sampler
                .into_binding()
        })
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("ShadowSamplers"))
}

#[cfg(feature = "experimental_pbr_pcss")]
pub(super) fn fetch_shadow_sampler_point_light_linear_sampler_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<ShadowSamplers>()
        .map(|shadow_samplers| shadow_samplers.point_light_linear_sampler.into_binding())
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("ShadowSamplers"))
}

pub(super) fn fetch_shadow_sampler_directional_light_comparison_sampler_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<ShadowSamplers>()
        .map(|shadow_samplers| {
            shadow_samplers
                .directional_light_comparison_sampler
                .into_binding()
        })
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("ShadowSamplers"))
}

#[cfg(feature = "experimental_pbr_pcss")]
pub(super) fn fetch_shadow_sampler_directional_light_linear_sampler_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .get_resource::<ShadowSamplers>()
        .map(|shadow_samplers| {
            shadow_samplers
                .directional_light_linear_sampler
                .into_binding()
        })
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("ShadowSamplers"))
}

pub(super) fn fetch_cluster_object_index_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .entity(view)
        .get::<ViewClusterBindings>()
        .and_then(ViewClusterBindings::clusterable_object_index_lists_binding)
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("ViewClusterBindings"))
}

pub(super) fn fetch_cluster_offsets_and_counts_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    world
        .entity(view)
        .get::<ViewClusterBindings>()
        .and_then(ViewClusterBindings::offsets_and_counts_binding)
        .map(WrappedBindingResource::from)
        .ok_or(MeshViewBindGroupFetchError::Missing("ViewClusterBindings"))
}

pub(super) fn fetch_environment_map_diffuse_view_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let render_view_environment_maps = world
        .entity(view)
        .get::<RenderViewLightProbes<EnvironmentMapLight>>();
    let images = world.get_resource::<RenderAssets<GpuImage>>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderAssets<GpuImage>"),
    )?;
    let fallback_image = world
        .get_resource::<FallbackImage>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImage"))?;
    let render_device = world
        .get_resource::<RenderDevice>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderDevice"))?;
    let render_adapter = world
        .get_resource::<RenderAdapter>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderAdapter"))?;
    let environment_map_bind_group_entries = RenderViewEnvironmentMapBindGroupEntries::get(
        render_view_environment_maps,
        images,
        fallback_image,
        render_device,
        render_adapter,
    );

    match environment_map_bind_group_entries {
        RenderViewEnvironmentMapBindGroupEntries::Single {
            diffuse_texture_view,
            specular_texture_view: _,
            sampler: _,
        } => Ok(diffuse_texture_view.into_binding().into()),
        RenderViewEnvironmentMapBindGroupEntries::Multiple {
            diffuse_texture_views,
            specular_texture_views: _,
            sampler: _,
        } => Ok(WrappedBindingResource::OwnedTextureViewArray(
            diffuse_texture_views,
        )),
    }
}

pub(super) fn fetch_environment_map_specular_view_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let render_view_environment_maps = world
        .entity(view)
        .get::<RenderViewLightProbes<EnvironmentMapLight>>();
    let images = world.get_resource::<RenderAssets<GpuImage>>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderAssets<GpuImage>"),
    )?;
    let fallback_image = world
        .get_resource::<FallbackImage>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImage"))?;
    let render_device = world
        .get_resource::<RenderDevice>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderDevice"))?;
    let render_adapter = world
        .get_resource::<RenderAdapter>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderAdapter"))?;
    let environment_map_bind_group_entries = RenderViewEnvironmentMapBindGroupEntries::get(
        render_view_environment_maps,
        images,
        fallback_image,
        render_device,
        render_adapter,
    );

    match environment_map_bind_group_entries {
        RenderViewEnvironmentMapBindGroupEntries::Single {
            diffuse_texture_view: _,
            specular_texture_view,
            sampler: _,
        } => Ok(specular_texture_view.into_binding().into()),
        RenderViewEnvironmentMapBindGroupEntries::Multiple {
            diffuse_texture_views: _,
            specular_texture_views,
            sampler: _,
        } => Ok(WrappedBindingResource::OwnedTextureViewArray(
            specular_texture_views,
        )),
    }
}

pub(super) fn fetch_environment_map_sampler_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let render_view_environment_maps = world
        .entity(view)
        .get::<RenderViewLightProbes<EnvironmentMapLight>>();
    let images = world.get_resource::<RenderAssets<GpuImage>>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderAssets<GpuImage>"),
    )?;
    let fallback_image = world
        .get_resource::<FallbackImage>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImage"))?;
    let render_device = world
        .get_resource::<RenderDevice>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderDevice"))?;
    let render_adapter = world
        .get_resource::<RenderAdapter>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderAdapter"))?;
    let environment_map_bind_group_entries = RenderViewEnvironmentMapBindGroupEntries::get(
        render_view_environment_maps,
        images,
        fallback_image,
        render_device,
        render_adapter,
    );

    match environment_map_bind_group_entries {
        RenderViewEnvironmentMapBindGroupEntries::Single {
            diffuse_texture_view: _,
            specular_texture_view: _,
            sampler,
        }
        | RenderViewEnvironmentMapBindGroupEntries::Multiple {
            diffuse_texture_views: _,
            specular_texture_views: _,
            sampler,
        } => Ok(sampler.into_binding().into()),
    }
}

pub(super) fn fetch_irradiance_volume_texture_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let render_view_irradiance_volumes = world
        .entity(view)
        .get::<RenderViewLightProbes<IrradianceVolume>>();
    let images = world.get_resource::<RenderAssets<GpuImage>>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderAssets<GpuImage>"),
    )?;
    let fallback_image = world
        .get_resource::<FallbackImage>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImage"))?;
    let render_device = world
        .get_resource::<RenderDevice>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderDevice"))?;
    let render_adapter = world
        .get_resource::<RenderAdapter>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderAdapter"))?;
    let irradiance_volume_bind_group_entries = RenderViewIrradianceVolumeBindGroupEntries::get(
        render_view_irradiance_volumes,
        images,
        fallback_image,
        render_device,
        render_adapter,
    );

    match irradiance_volume_bind_group_entries {
        RenderViewIrradianceVolumeBindGroupEntries::Single {
            texture_view,
            sampler: _,
        } => Ok(texture_view.into_binding().into()),
        RenderViewIrradianceVolumeBindGroupEntries::Multiple {
            texture_views,
            sampler: _,
        } => Ok(WrappedBindingResource::OwnedTextureViewArray(texture_views)),
    }
}

pub(super) fn fetch_irradiance_volume_sampler_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let render_view_irradiance_volumes = world
        .entity(view)
        .get::<RenderViewLightProbes<IrradianceVolume>>();
    let images = world.get_resource::<RenderAssets<GpuImage>>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderAssets<GpuImage>"),
    )?;
    let fallback_image = world
        .get_resource::<FallbackImage>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImage"))?;
    let render_device = world
        .get_resource::<RenderDevice>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderDevice"))?;
    let render_adapter = world
        .get_resource::<RenderAdapter>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderAdapter"))?;
    let irradiance_volume_bind_group_entries = RenderViewIrradianceVolumeBindGroupEntries::get(
        render_view_irradiance_volumes,
        images,
        fallback_image,
        render_device,
        render_adapter,
    );

    match irradiance_volume_bind_group_entries {
        RenderViewIrradianceVolumeBindGroupEntries::Single {
            texture_view: _,
            sampler,
        }
        | RenderViewIrradianceVolumeBindGroupEntries::Multiple {
            texture_views: _,
            sampler,
        } => Ok(sampler.into_binding().into()),
    }
}

pub(super) fn fetch_decals_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let render_decals = world.get_resource::<RenderClusteredDecals>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderClusteredDecals"),
    )?;
    let decals_buffer = world
        .get_resource::<DecalsBuffer>()
        .ok_or(MeshViewBindGroupFetchError::Missing("DecalsBuffer"))?;
    let images = world.get_resource::<RenderAssets<GpuImage>>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderAssets<GpuImage>"),
    )?;
    let fallback_image = world
        .get_resource::<FallbackImage>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImage"))?;
    let render_device = world
        .get_resource::<RenderDevice>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderDevice"))?;
    let render_adapter = world
        .get_resource::<RenderAdapter>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderAdapter"))?;
    let decal_bind_group_entries = RenderViewClusteredDecalBindGroupEntries::get(
        render_decals,
        decals_buffer,
        images,
        fallback_image,
        render_device,
        render_adapter,
    );

    // Add the decal bind group entries.
    decal_bind_group_entries
        .map(|render_view_decal_bind_group_entries| {
            render_view_decal_bind_group_entries
                .decals
                .as_entire_binding()
                .into()
        })
        .ok_or(MeshViewBindGroupFetchError::Missing("Decals"))
}

pub(super) fn fetch_decal_texture_views_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let render_decals = world.get_resource::<RenderClusteredDecals>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderClusteredDecals"),
    )?;
    let decals_buffer = world
        .get_resource::<DecalsBuffer>()
        .ok_or(MeshViewBindGroupFetchError::Missing("DecalsBuffer"))?;
    let images = world.get_resource::<RenderAssets<GpuImage>>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderAssets<GpuImage>"),
    )?;
    let fallback_image = world
        .get_resource::<FallbackImage>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImage"))?;
    let render_device = world
        .get_resource::<RenderDevice>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderDevice"))?;
    let render_adapter = world
        .get_resource::<RenderAdapter>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderAdapter"))?;
    let decal_bind_group_entries = RenderViewClusteredDecalBindGroupEntries::get(
        render_decals,
        decals_buffer,
        images,
        fallback_image,
        render_device,
        render_adapter,
    );

    // Add the decal bind group entries.
    decal_bind_group_entries
        .map(|render_view_decal_bind_group_entries| {
            WrappedBindingResource::OwnedTextureViewArray(
                render_view_decal_bind_group_entries.texture_views,
            )
        })
        .ok_or(MeshViewBindGroupFetchError::Missing("DecalTextureViews"))
}

pub(super) fn fetch_decal_sampler_bind_group<'b>(
    world: &'b World,
    _view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let render_decals = world.get_resource::<RenderClusteredDecals>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderClusteredDecals"),
    )?;
    let decals_buffer = world
        .get_resource::<DecalsBuffer>()
        .ok_or(MeshViewBindGroupFetchError::Missing("DecalsBuffer"))?;
    let images = world.get_resource::<RenderAssets<GpuImage>>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderAssets<GpuImage>"),
    )?;
    let fallback_image = world
        .get_resource::<FallbackImage>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImage"))?;
    let render_device = world
        .get_resource::<RenderDevice>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderDevice"))?;
    let render_adapter = world
        .get_resource::<RenderAdapter>()
        .ok_or(MeshViewBindGroupFetchError::Missing("RenderAdapter"))?;
    let decal_bind_group_entries = RenderViewClusteredDecalBindGroupEntries::get(
        render_decals,
        decals_buffer,
        images,
        fallback_image,
        render_device,
        render_adapter,
    );

    // Add the decal bind group entries.
    decal_bind_group_entries
        .map(|render_view_decal_bind_group_entries| {
            render_view_decal_bind_group_entries
                .sampler
                .into_binding()
                .into()
        })
        .ok_or(MeshViewBindGroupFetchError::Missing("DecalSampler"))
}

pub(super) fn fetch_tonemapping_luts_texture_view_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let tonemapping_luts = world
        .get_resource::<TonemappingLuts>()
        .ok_or(MeshViewBindGroupFetchError::Missing("TonemappingLuts"))?;
    let tonemapping = world
        .entity(view)
        .get::<Tonemapping>()
        .ok_or(MeshViewBindGroupFetchError::Missing("Tonemapping"))?;
    let images = world.get_resource::<RenderAssets<GpuImage>>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderAssets<GpuImage>"),
    )?;
    let fallback_image = world
        .get_resource::<FallbackImage>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImage"))?;
    let lut_bindings = get_lut_bindings(images, tonemapping_luts, tonemapping, fallback_image);
    Ok(lut_bindings.0.into_binding().into())
}

pub(super) fn fetch_tonemapping_luts_sampler_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let tonemapping_luts = world
        .get_resource::<TonemappingLuts>()
        .ok_or(MeshViewBindGroupFetchError::Missing("TonemappingLuts"))?;
    let tonemapping = world
        .entity(view)
        .get::<Tonemapping>()
        .ok_or(MeshViewBindGroupFetchError::Missing("Tonemapping"))?;
    let images = world.get_resource::<RenderAssets<GpuImage>>().ok_or(
        MeshViewBindGroupFetchError::Missing("RenderAssets<GpuImage>"),
    )?;
    let fallback_image = world
        .get_resource::<FallbackImage>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImage"))?;
    let lut_bindings = get_lut_bindings(images, tonemapping_luts, tonemapping, fallback_image);
    Ok(lut_bindings.1.into_binding().into())
}

pub(super) fn fetch_depth_texture_view_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let msaa = world
        .entity(view)
        .get::<Msaa>()
        .ok_or(MeshViewBindGroupFetchError::Missing("Msaa"))?;
    let prepass_textures = world.entity(view).get::<ViewPrepassTextures>();
    // When using WebGL, we can't have a depth texture with multisampling
    if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32"))) || msaa.samples() == 1 {
        prepass::get_bindings(prepass_textures)[0]
            .clone()
            .map(WrappedBindingResource::OwnedTextureView)
            .ok_or(MeshViewBindGroupFetchError::Skipped)
    } else {
        Err(MeshViewBindGroupFetchError::Skipped)
    }
}

pub(super) fn fetch_normal_texture_view_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let msaa = world
        .entity(view)
        .get::<Msaa>()
        .ok_or(MeshViewBindGroupFetchError::Missing("Msaa"))?;
    let prepass_textures = world.entity(view).get::<ViewPrepassTextures>();
    // When using WebGL, we can't have a depth texture with multisampling
    if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32"))) || msaa.samples() == 1 {
        prepass::get_bindings(prepass_textures)[1]
            .clone()
            .map(WrappedBindingResource::OwnedTextureView)
            .ok_or(MeshViewBindGroupFetchError::Skipped)
    } else {
        Err(MeshViewBindGroupFetchError::Skipped)
    }
}

pub(super) fn fetch_motion_vector_texture_view_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let msaa = world
        .entity(view)
        .get::<Msaa>()
        .ok_or(MeshViewBindGroupFetchError::Missing("Msaa"))?;
    let prepass_textures = world.entity(view).get::<ViewPrepassTextures>();
    // When using WebGL, we can't have a depth texture with multisampling
    if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32"))) || msaa.samples() == 1 {
        prepass::get_bindings(prepass_textures)[2]
            .clone()
            .map(WrappedBindingResource::OwnedTextureView)
            .ok_or(MeshViewBindGroupFetchError::Skipped)
    } else {
        Err(MeshViewBindGroupFetchError::Skipped)
    }
}

pub(super) fn fetch_deferred_texture_view_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let msaa = world
        .entity(view)
        .get::<Msaa>()
        .ok_or(MeshViewBindGroupFetchError::Missing("Msaa"))?;
    let prepass_textures = world.entity(view).get::<ViewPrepassTextures>();
    // When using WebGL, we can't have a depth texture with multisampling
    if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32"))) || msaa.samples() == 1 {
        prepass::get_bindings(prepass_textures)[3]
            .clone()
            .map(WrappedBindingResource::OwnedTextureView)
            .ok_or(MeshViewBindGroupFetchError::Skipped)
    } else {
        Err(MeshViewBindGroupFetchError::Skipped)
    }
}

pub(super) fn fetch_transmission_texture_view_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let transmission_texture = world.entity(view).get::<ViewTransmissionTexture>();
    let fallback_image_zero = world
        .get_resource::<FallbackImageZero>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImageZero"))?;

    let transmission_view = transmission_texture
        .map(|transmission| &transmission.view)
        .unwrap_or(&fallback_image_zero.texture_view);

    Ok(transmission_view.into_binding().into())
}

pub(super) fn fetch_transmission_sampler_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let transmission_texture = world.entity(view).get::<ViewTransmissionTexture>();
    let fallback_image_zero = world
        .get_resource::<FallbackImageZero>()
        .ok_or(MeshViewBindGroupFetchError::Missing("FallbackImageZero"))?;

    let transmission_sampler = transmission_texture
        .map(|transmission| &transmission.sampler)
        .unwrap_or(&fallback_image_zero.sampler);

    Ok(transmission_sampler.into_binding().into())
}

pub(super) fn fetch_order_independet_transparency_layers_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let has_oit = world
        .entity(view)
        .contains::<OrderIndependentTransparencySettings>();
    if has_oit {
        let Some(oit_buffers) = world.get_resource::<OitBuffers>() else {
            unreachable!("If view has OrderIndependentTransparencySettings then resource OitBuffers must exist");
        };
        if let (Some(binding), Some(_), Some(_)) = (
            oit_buffers.layers.binding(),
            oit_buffers.layer_ids.binding(),
            oit_buffers.settings.binding(),
        ) {
            Ok(binding.into())
        } else {
            Err(MeshViewBindGroupFetchError::Skipped)
        }
    } else {
        Err(MeshViewBindGroupFetchError::Skipped)
    }
}

pub(super) fn fetch_order_independet_transparency_layer_ids_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let has_oit = world
        .entity(view)
        .contains::<OrderIndependentTransparencySettings>();
    if has_oit {
        let Some(oit_buffers) = world.get_resource::<OitBuffers>() else {
            unreachable!("If view has OrderIndependentTransparencySettings then resource OitBuffers must exist");
        };
        if let (Some(_), Some(binding), Some(_)) = (
            oit_buffers.layers.binding(),
            oit_buffers.layer_ids.binding(),
            oit_buffers.settings.binding(),
        ) {
            Ok(binding.into())
        } else {
            Err(MeshViewBindGroupFetchError::Skipped)
        }
    } else {
        Err(MeshViewBindGroupFetchError::Skipped)
    }
}

pub(super) fn fetch_order_independet_transparency_settings_bind_group<'b>(
    world: &'b World,
    view: Entity,
) -> Result<WrappedBindingResource<'b>, MeshViewBindGroupFetchError> {
    let has_oit = world
        .entity(view)
        .contains::<OrderIndependentTransparencySettings>();
    if has_oit {
        let Some(oit_buffers) = world.get_resource::<OitBuffers>() else {
            unreachable!("If view has OrderIndependentTransparencySettings then resource OitBuffers must exist");
        };
        if let (Some(_), Some(_), Some(binding)) = (
            oit_buffers.layers.binding(),
            oit_buffers.layer_ids.binding(),
            oit_buffers.settings.binding(),
        ) {
            Ok(binding.into())
        } else {
            Err(MeshViewBindGroupFetchError::Skipped)
        }
    } else {
        Err(MeshViewBindGroupFetchError::Skipped)
    }
}
