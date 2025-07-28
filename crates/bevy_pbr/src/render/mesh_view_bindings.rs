use alloc::sync::Arc;
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
    query::Has,
    resource::Resource,
    system::{Commands, Query, Res},
    world::{FromWorld, World},
};
use bevy_image::BevyDefault as _;
use bevy_light::EnvironmentMapLight;
use bevy_math::Vec4;
use bevy_render::{
    globals::{GlobalsBuffer, GlobalsUniform},
    render_asset::RenderAssets,
    render_resource::{binding_types::*, *},
    renderer::{RenderAdapter, RenderDevice},
    texture::{FallbackImage, FallbackImageMsaa, FallbackImageZero, GpuImage},
    view::{
        Msaa, RenderVisibilityRanges, ViewUniform, ViewUniforms,
        VISIBILITY_RANGES_STORAGE_BUFFER_COUNT,
    },
};
use core::{array, num::NonZero};

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
    MeshPipeline, MeshPipelineKey, RenderViewLightProbes, ScreenSpaceAmbientOcclusionResources,
    ScreenSpaceReflectionsBuffer, ScreenSpaceReflectionsUniform, ShadowSamplers,
    ViewClusterBindings, ViewShadowBindings, CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT,
};

#[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
use bevy_render::render_resource::binding_types::texture_cube;

#[cfg(debug_assertions)]
use {crate::MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES, bevy_utils::once, tracing::warn};

#[derive(Clone)]
pub struct MeshPipelineViewLayout {
    pub main_layout: BindGroupLayout,
    pub binding_array_layout: BindGroupLayout,
    pub empty_layout: BindGroupLayout,

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
            if self.contains(Key::MULTISAMPLED) {
                "_multisampled"
            } else {
                Default::default()
            },
            if self.contains(Key::DEPTH_PREPASS) {
                "_depth"
            } else {
                Default::default()
            },
            if self.contains(Key::NORMAL_PREPASS) {
                "_normal"
            } else {
                Default::default()
            },
            if self.contains(Key::MOTION_VECTOR_PREPASS) {
                "_motion"
            } else {
                Default::default()
            },
            if self.contains(Key::DEFERRED_PREPASS) {
                "_deferred"
            } else {
                Default::default()
            },
            if self.contains(Key::OIT_ENABLED) {
                "_oit"
            } else {
                Default::default()
            },
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
) -> [Vec<BindGroupLayoutEntry>; 2] {
    // EnvironmentMapLight
    let environment_map_entries =
        environment_map::get_bind_group_layout_entries(render_device, render_adapter);

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
            (17, environment_map_entries[3]),
        ),
    );

    // Tonemapping
    let tonemapping_lut_entries = get_lut_bind_group_layout_entries();
    entries = entries.extend_with_indices((
        (18, tonemapping_lut_entries[0]),
        (19, tonemapping_lut_entries[1]),
    ));

    // Prepass
    if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32")))
        || (cfg!(all(feature = "webgl", target_arch = "wasm32"))
            && !layout_key.contains(MeshPipelineViewLayoutKey::MULTISAMPLED))
    {
        for (entry, binding) in prepass::get_bind_group_layout_entries(layout_key)
            .iter()
            .zip([20, 21, 22, 23])
        {
            if let Some(entry) = entry {
                entries = entries.extend_with_indices(((binding as u32, *entry),));
            }
        }
    }

    // View Transmission Texture
    entries = entries.extend_with_indices((
        (
            24,
            texture_2d(TextureSampleType::Float { filterable: true }),
        ),
        (25, sampler(SamplerBindingType::Filtering)),
    ));

    // OIT
    if layout_key.contains(MeshPipelineViewLayoutKey::OIT_ENABLED) {
        // Check if we can use OIT. This is a hack to avoid errors on webgl --
        // the OIT plugin will warn the user that OIT is not supported on their
        // platform, so we don't need to do it here.
        if is_oit_supported(render_adapter, render_device, false) {
            entries = entries.extend_with_indices((
                // oit_layers
                (26, storage_buffer_sized(false, None)),
                // oit_layer_ids,
                (27, storage_buffer_sized(false, None)),
                // oit_layer_count
                (
                    28,
                    uniform_buffer::<OrderIndependentTransparencySettings>(true),
                ),
            ));
        }
    }

    let mut binding_array_entries = DynamicBindGroupLayoutEntries::new(ShaderStages::FRAGMENT);
    binding_array_entries = binding_array_entries.extend_with_indices((
        (0, environment_map_entries[0]),
        (1, environment_map_entries[1]),
        (2, environment_map_entries[2]),
    ));

    // Irradiance volumes
    if IRRADIANCE_VOLUMES_ARE_USABLE {
        let irradiance_volume_entries =
            irradiance_volume::get_bind_group_layout_entries(render_device, render_adapter);
        binding_array_entries = binding_array_entries.extend_with_indices((
            (3, irradiance_volume_entries[0]),
            (4, irradiance_volume_entries[1]),
        ));
    }

    // Clustered decals
    if let Some(clustered_decal_entries) =
        decal::clustered::get_bind_group_layout_entries(render_device, render_adapter)
    {
        binding_array_entries = binding_array_entries.extend_with_indices((
            (5, clustered_decal_entries[0]),
            (6, clustered_decal_entries[1]),
            (7, clustered_decal_entries[2]),
        ));
    }

    [entries.to_vec(), binding_array_entries.to_vec()]
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
                .flat_map(|e| {
                    e.iter()
                        .filter(|entry| matches!(entry.ty, BindingType::Texture { .. }))
                })
                .count();

            MeshPipelineViewLayout {
                main_layout: render_device
                    .create_bind_group_layout(key.label().as_str(), &entries[0]),
                binding_array_layout: render_device.create_bind_group_layout(
                    format!("{}_binding_array", key.label()).as_str(),
                    &entries[1],
                ),
                empty_layout: render_device
                    .create_bind_group_layout(format!("{}_empty", key.label()).as_str(), &[]),
                #[cfg(debug_assertions)]
                texture_count,
            }
        })))
    }
}

impl MeshPipelineViewLayouts {
    pub fn get_view_layout(
        &self,
        layout_key: MeshPipelineViewLayoutKey,
    ) -> &MeshPipelineViewLayout {
        let index = layout_key.bits() as usize;
        let layout = &self[index];

        #[cfg(debug_assertions)]
        if layout.texture_count > MESH_PIPELINE_VIEW_LAYOUT_SAFE_MAX_TEXTURES {
            // Issue our own warning here because Naga's error message is a bit cryptic in this situation
            once!(warn!("Too many textures in mesh pipeline view layout, this might cause us to hit `wgpu::Limits::max_sampled_textures_per_shader_stage` in some environments."));
        }

        layout
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
            .flat_map(|e| {
                e.iter()
                    .filter(|entry| matches!(entry.ty, BindingType::Texture { .. }))
            })
            .count();

        MeshPipelineViewLayout {
            main_layout: render_device.create_bind_group_layout(key.label().as_str(), &entries[0]),
            binding_array_layout: render_device.create_bind_group_layout(
                format!("{}_binding_array", key.label()).as_str(),
                &entries[1],
            ),
            empty_layout: render_device
                .create_bind_group_layout(format!("{}_empty", key.label()).as_str(), &[]),
            #[cfg(debug_assertions)]
            texture_count,
        }
    })
}

#[derive(Component)]
pub struct MeshViewBindGroup {
    pub main: BindGroup,
    pub binding_array: BindGroup,
    pub empty: BindGroup,
}

pub fn prepare_mesh_view_bind_groups(
    mut commands: Commands,
    (render_device, render_adapter): (Res<RenderDevice>, Res<RenderAdapter>),
    mesh_pipeline: Res<MeshPipeline>,
    shadow_samplers: Res<ShadowSamplers>,
    (light_meta, global_light_meta): (Res<LightMeta>, Res<GlobalClusterableObjectMeta>),
    fog_meta: Res<FogMeta>,
    (view_uniforms, environment_map_uniform): (Res<ViewUniforms>, Res<EnvironmentMapUniformBuffer>),
    views: Query<(
        Entity,
        &ViewShadowBindings,
        &ViewClusterBindings,
        &Msaa,
        Option<&ScreenSpaceAmbientOcclusionResources>,
        Option<&ViewPrepassTextures>,
        Option<&ViewTransmissionTexture>,
        &Tonemapping,
        Option<&RenderViewLightProbes<EnvironmentMapLight>>,
        Option<&RenderViewLightProbes<IrradianceVolume>>,
        Has<OrderIndependentTransparencySettings>,
    )>,
    (images, mut fallback_images, fallback_image, fallback_image_zero): (
        Res<RenderAssets<GpuImage>>,
        FallbackImageMsaa,
        Res<FallbackImage>,
        Res<FallbackImageZero>,
    ),
    globals_buffer: Res<GlobalsBuffer>,
    tonemapping_luts: Res<TonemappingLuts>,
    light_probes_buffer: Res<LightProbesBuffer>,
    visibility_ranges: Res<RenderVisibilityRanges>,
    ssr_buffer: Res<ScreenSpaceReflectionsBuffer>,
    oit_buffers: Res<OitBuffers>,
    (decals_buffer, render_decals): (Res<DecalsBuffer>, Res<RenderClusteredDecals>),
) {
    if let (
        Some(view_binding),
        Some(light_binding),
        Some(clusterable_objects_binding),
        Some(globals),
        Some(fog_binding),
        Some(light_probes_binding),
        Some(visibility_ranges_buffer),
        Some(ssr_binding),
        Some(environment_map_binding),
    ) = (
        view_uniforms.uniforms.binding(),
        light_meta.view_gpu_lights.binding(),
        global_light_meta.gpu_clusterable_objects.binding(),
        globals_buffer.buffer.binding(),
        fog_meta.gpu_fogs.binding(),
        light_probes_buffer.binding(),
        visibility_ranges.buffer().buffer(),
        ssr_buffer.binding(),
        environment_map_uniform.binding(),
    ) {
        for (
            entity,
            shadow_bindings,
            cluster_bindings,
            msaa,
            ssao_resources,
            prepass_textures,
            transmission_texture,
            tonemapping,
            render_view_environment_maps,
            render_view_irradiance_volumes,
            has_oit,
        ) in &views
        {
            let fallback_ssao = fallback_images
                .image_for_samplecount(1, TextureFormat::bevy_default())
                .texture_view
                .clone();
            let ssao_view = ssao_resources
                .map(|t| &t.screen_space_ambient_occlusion_texture.default_view)
                .unwrap_or(&fallback_ssao);

            let mut layout_key = MeshPipelineViewLayoutKey::from(*msaa)
                | MeshPipelineViewLayoutKey::from(prepass_textures);
            if has_oit {
                layout_key |= MeshPipelineViewLayoutKey::OIT_ENABLED;
            }

            let layout = mesh_pipeline.get_view_layout(layout_key);

            let mut entries = DynamicBindGroupEntries::new_with_indices((
                (0, view_binding.clone()),
                (1, light_binding.clone()),
                (2, &shadow_bindings.point_light_depth_texture_view),
                (3, &shadow_samplers.point_light_comparison_sampler),
                #[cfg(feature = "experimental_pbr_pcss")]
                (4, &shadow_samplers.point_light_linear_sampler),
                (5, &shadow_bindings.directional_light_depth_texture_view),
                (6, &shadow_samplers.directional_light_comparison_sampler),
                #[cfg(feature = "experimental_pbr_pcss")]
                (7, &shadow_samplers.directional_light_linear_sampler),
                (8, clusterable_objects_binding.clone()),
                (
                    9,
                    cluster_bindings
                        .clusterable_object_index_lists_binding()
                        .unwrap(),
                ),
                (10, cluster_bindings.offsets_and_counts_binding().unwrap()),
                (11, globals.clone()),
                (12, fog_binding.clone()),
                (13, light_probes_binding.clone()),
                (14, visibility_ranges_buffer.as_entire_binding()),
                (15, ssr_binding.clone()),
                (16, ssao_view),
            ));

            entries = entries.extend_with_indices(((17, environment_map_binding.clone()),));

            let lut_bindings =
                get_lut_bindings(&images, &tonemapping_luts, tonemapping, &fallback_image);
            entries = entries.extend_with_indices(((18, lut_bindings.0), (19, lut_bindings.1)));

            // When using WebGL, we can't have a depth texture with multisampling
            let prepass_bindings;
            if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32"))) || msaa.samples() == 1
            {
                prepass_bindings = prepass::get_bindings(prepass_textures);
                for (binding, index) in prepass_bindings
                    .iter()
                    .map(Option::as_ref)
                    .zip([20, 21, 22, 23])
                    .flat_map(|(b, i)| b.map(|b| (b, i)))
                {
                    entries = entries.extend_with_indices(((index, binding),));
                }
            };

            let transmission_view = transmission_texture
                .map(|transmission| &transmission.view)
                .unwrap_or(&fallback_image_zero.texture_view);

            let transmission_sampler = transmission_texture
                .map(|transmission| &transmission.sampler)
                .unwrap_or(&fallback_image_zero.sampler);

            entries =
                entries.extend_with_indices(((24, transmission_view), (25, transmission_sampler)));

            if has_oit {
                if let (
                    Some(oit_layers_binding),
                    Some(oit_layer_ids_binding),
                    Some(oit_settings_binding),
                ) = (
                    oit_buffers.layers.binding(),
                    oit_buffers.layer_ids.binding(),
                    oit_buffers.settings.binding(),
                ) {
                    entries = entries.extend_with_indices((
                        (26, oit_layers_binding.clone()),
                        (27, oit_layer_ids_binding.clone()),
                        (28, oit_settings_binding.clone()),
                    ));
                }
            }

            let mut entries_binding_array = DynamicBindGroupEntries::new();

            let environment_map_bind_group_entries = RenderViewEnvironmentMapBindGroupEntries::get(
                render_view_environment_maps,
                &images,
                &fallback_image,
                &render_device,
                &render_adapter,
            );
            match environment_map_bind_group_entries {
                RenderViewEnvironmentMapBindGroupEntries::Single {
                    diffuse_texture_view,
                    specular_texture_view,
                    sampler,
                } => {
                    entries_binding_array = entries_binding_array.extend_with_indices((
                        (0, diffuse_texture_view),
                        (1, specular_texture_view),
                        (2, sampler),
                    ));
                }
                RenderViewEnvironmentMapBindGroupEntries::Multiple {
                    ref diffuse_texture_views,
                    ref specular_texture_views,
                    sampler,
                } => {
                    entries_binding_array = entries_binding_array.extend_with_indices((
                        (0, diffuse_texture_views.as_slice()),
                        (1, specular_texture_views.as_slice()),
                        (2, sampler),
                    ));
                }
            }

            let irradiance_volume_bind_group_entries = if IRRADIANCE_VOLUMES_ARE_USABLE {
                Some(RenderViewIrradianceVolumeBindGroupEntries::get(
                    render_view_irradiance_volumes,
                    &images,
                    &fallback_image,
                    &render_device,
                    &render_adapter,
                ))
            } else {
                None
            };

            match irradiance_volume_bind_group_entries {
                Some(RenderViewIrradianceVolumeBindGroupEntries::Single {
                    texture_view,
                    sampler,
                }) => {
                    entries_binding_array = entries_binding_array
                        .extend_with_indices(((3, texture_view), (4, sampler)));
                }
                Some(RenderViewIrradianceVolumeBindGroupEntries::Multiple {
                    ref texture_views,
                    sampler,
                }) => {
                    entries_binding_array = entries_binding_array
                        .extend_with_indices(((3, texture_views.as_slice()), (4, sampler)));
                }
                None => {}
            }

            let decal_bind_group_entries = RenderViewClusteredDecalBindGroupEntries::get(
                &render_decals,
                &decals_buffer,
                &images,
                &fallback_image,
                &render_device,
                &render_adapter,
            );

            // Add the decal bind group entries.
            if let Some(ref render_view_decal_bind_group_entries) = decal_bind_group_entries {
                entries_binding_array = entries_binding_array.extend_with_indices((
                    // `clustered_decals`
                    (
                        5,
                        render_view_decal_bind_group_entries
                            .decals
                            .as_entire_binding(),
                    ),
                    // `clustered_decal_textures`
                    (
                        6,
                        render_view_decal_bind_group_entries
                            .texture_views
                            .as_slice(),
                    ),
                    // `clustered_decal_sampler`
                    (7, render_view_decal_bind_group_entries.sampler),
                ));
            }

            commands.entity(entity).insert(MeshViewBindGroup {
                main: render_device.create_bind_group(
                    "mesh_view_bind_group",
                    &layout.main_layout,
                    &entries,
                ),
                binding_array: render_device.create_bind_group(
                    "mesh_view_bind_group_binding_array",
                    &layout.binding_array_layout,
                    &entries_binding_array,
                ),
                empty: render_device.create_bind_group(
                    "mesh_view_bind_group_empty",
                    &layout.empty_layout,
                    &[],
                ),
            });
        }
    }
}
