use std::array;

use bevy_core_pipeline::{
    core_3d::ViewTransmissionTexture,
    prepass::ViewPrepassTextures,
    tonemapping::{
        get_lut_bind_group_layout_entries, get_lut_bindings, Tonemapping, TonemappingLuts,
    },
};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Commands, Query, Res},
};
use bevy_render::{
    globals::{GlobalsBuffer, GlobalsUniform},
    render_asset::RenderAssets,
    render_resource::{
        BindGroup, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
        BufferBindingType, DynamicBindGroupEntries, SamplerBindingType, ShaderStages, ShaderType,
        TextureFormat, TextureSampleType, TextureViewDimension,
    },
    renderer::RenderDevice,
    texture::{BevyDefault, FallbackImageCubemap, FallbackImageMsaa, FallbackImageZero, Image},
    view::{Msaa, ViewUniform, ViewUniforms},
};

use crate::{
    environment_map, prepass, EnvironmentMapLight, FogMeta, GlobalLightMeta, GpuFog, GpuLights,
    GpuPointLights, LightMeta, MeshPipeline, MeshPipelineKey, ScreenSpaceAmbientOcclusionTextures,
    ShadowSamplers, ViewClusterBindings, ViewShadowBindings,
};

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
        const MULTISAMPLED                = (1 << 0);
        const DEPTH_PREPASS               = (1 << 1);
        const NORMAL_PREPASS              = (1 << 2);
        const MOTION_VECTOR_PREPASS       = (1 << 3);
        const DEFERRED_PREPASS            = (1 << 4);
    }
}

impl MeshPipelineViewLayoutKey {
    // The number of possible layouts
    pub const COUNT: usize = Self::all().bits() as usize + 1;

    /// Builds a unique label for each layout based on the flags
    pub fn label(&self) -> String {
        use MeshPipelineViewLayoutKey as Key;

        format!(
            "mesh_view_layout{}{}{}{}{}",
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

/// Returns the appropriate bind group layout vec based on the parameters
fn layout_entries(
    clustered_forward_buffer_binding_type: BufferBindingType,
    layout_key: MeshPipelineViewLayoutKey,
) -> Vec<BindGroupLayoutEntry> {
    let mut entries = vec![
        // View
        BindGroupLayoutEntry {
            binding: 0,
            visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(ViewUniform::min_size()),
            },
            count: None,
        },
        // Lights
        BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(GpuLights::min_size()),
            },
            count: None,
        },
        // Point Shadow Texture Cube Array
        BindGroupLayoutEntry {
            binding: 2,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                sample_type: TextureSampleType::Depth,
                #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
                view_dimension: TextureViewDimension::CubeArray,
                #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
                view_dimension: TextureViewDimension::Cube,
            },
            count: None,
        },
        // Point Shadow Texture Array Sampler
        BindGroupLayoutEntry {
            binding: 3,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Comparison),
            count: None,
        },
        // Directional Shadow Texture Array
        BindGroupLayoutEntry {
            binding: 4,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                sample_type: TextureSampleType::Depth,
                #[cfg(any(not(feature = "webgl"), not(target_arch = "wasm32")))]
                view_dimension: TextureViewDimension::D2Array,
                #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        // Directional Shadow Texture Array Sampler
        BindGroupLayoutEntry {
            binding: 5,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Comparison),
            count: None,
        },
        // PointLights
        BindGroupLayoutEntry {
            binding: 6,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: clustered_forward_buffer_binding_type,
                has_dynamic_offset: false,
                min_binding_size: Some(GpuPointLights::min_size(
                    clustered_forward_buffer_binding_type,
                )),
            },
            count: None,
        },
        // ClusteredLightIndexLists
        BindGroupLayoutEntry {
            binding: 7,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: clustered_forward_buffer_binding_type,
                has_dynamic_offset: false,
                min_binding_size: Some(ViewClusterBindings::min_size_cluster_light_index_lists(
                    clustered_forward_buffer_binding_type,
                )),
            },
            count: None,
        },
        // ClusterOffsetsAndCounts
        BindGroupLayoutEntry {
            binding: 8,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: clustered_forward_buffer_binding_type,
                has_dynamic_offset: false,
                min_binding_size: Some(ViewClusterBindings::min_size_cluster_offsets_and_counts(
                    clustered_forward_buffer_binding_type,
                )),
            },
            count: None,
        },
        // Globals
        BindGroupLayoutEntry {
            binding: 9,
            visibility: ShaderStages::VERTEX_FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: Some(GlobalsUniform::min_size()),
            },
            count: None,
        },
        // Fog
        BindGroupLayoutEntry {
            binding: 10,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(GpuFog::min_size()),
            },
            count: None,
        },
        // Screen space ambient occlusion texture
        BindGroupLayoutEntry {
            binding: 11,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                multisampled: false,
                sample_type: TextureSampleType::Float { filterable: false },
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
    ];

    // EnvironmentMapLight
    let environment_map_entries = environment_map::get_bind_group_layout_entries([12, 13, 14]);
    entries.extend_from_slice(&environment_map_entries);

    // Tonemapping
    let tonemapping_lut_entries = get_lut_bind_group_layout_entries([15, 16]);
    entries.extend_from_slice(&tonemapping_lut_entries);

    // Prepass
    if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32")))
        || (cfg!(all(feature = "webgl", target_arch = "wasm32"))
            && !layout_key.contains(MeshPipelineViewLayoutKey::MULTISAMPLED))
    {
        entries.extend_from_slice(&prepass::get_bind_group_layout_entries(
            [17, 18, 19, 20],
            layout_key,
        ));
    }

    // View Transmission Texture
    entries.extend_from_slice(&[
        BindGroupLayoutEntry {
            binding: 21,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                multisampled: false,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        },
        BindGroupLayoutEntry {
            binding: 22,
            visibility: ShaderStages::FRAGMENT,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            count: None,
        },
    ]);

    entries
}

/// Generates all possible view layouts for the mesh pipeline, based on all combinations of
/// [`MeshPipelineViewLayoutKey`] flags.
pub fn generate_view_layouts(
    render_device: &RenderDevice,
    clustered_forward_buffer_binding_type: BufferBindingType,
) -> [MeshPipelineViewLayout; MeshPipelineViewLayoutKey::COUNT] {
    array::from_fn(|i| {
        let key = MeshPipelineViewLayoutKey::from_bits_truncate(i as u32);
        let entries = layout_entries(clustered_forward_buffer_binding_type, key);

        #[cfg(debug_assertions)]
        let texture_count: usize = entries
            .iter()
            .filter(|entry| matches!(entry.ty, BindingType::Texture { .. }))
            .count();

        MeshPipelineViewLayout {
            bind_group_layout: render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some(key.label().as_str()),
                entries: &entries,
            }),
            #[cfg(debug_assertions)]
            texture_count,
        }
    })
}

#[derive(Component)]
pub struct MeshViewBindGroup {
    pub value: BindGroup,
}

#[allow(clippy::too_many_arguments)]
pub fn prepare_mesh_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mesh_pipeline: Res<MeshPipeline>,
    shadow_samplers: Res<ShadowSamplers>,
    light_meta: Res<LightMeta>,
    global_light_meta: Res<GlobalLightMeta>,
    fog_meta: Res<FogMeta>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(
        Entity,
        &ViewShadowBindings,
        &ViewClusterBindings,
        Option<&ScreenSpaceAmbientOcclusionTextures>,
        Option<&ViewPrepassTextures>,
        Option<&ViewTransmissionTexture>,
        Option<&EnvironmentMapLight>,
        &Tonemapping,
    )>,
    (images, mut fallback_images, fallback_cubemap, fallback_image_zero): (
        Res<RenderAssets<Image>>,
        FallbackImageMsaa,
        Res<FallbackImageCubemap>,
        Res<FallbackImageZero>,
    ),
    msaa: Res<Msaa>,
    globals_buffer: Res<GlobalsBuffer>,
    tonemapping_luts: Res<TonemappingLuts>,
) {
    if let (
        Some(view_binding),
        Some(light_binding),
        Some(point_light_binding),
        Some(globals),
        Some(fog_binding),
    ) = (
        view_uniforms.uniforms.binding(),
        light_meta.view_gpu_lights.binding(),
        global_light_meta.gpu_point_lights.binding(),
        globals_buffer.buffer.binding(),
        fog_meta.gpu_fogs.binding(),
    ) {
        for (
            entity,
            shadow_bindings,
            cluster_bindings,
            ssao_textures,
            prepass_textures,
            transmission_texture,
            environment_map,
            tonemapping,
        ) in &views
        {
            let fallback_ssao = fallback_images
                .image_for_samplecount(1, TextureFormat::bevy_default())
                .texture_view
                .clone();
            let ssao_view = ssao_textures
                .map(|t| &t.screen_space_ambient_occlusion_texture.default_view)
                .unwrap_or(&fallback_ssao);

            let layout = &mesh_pipeline.get_view_layout(
                MeshPipelineViewLayoutKey::from(*msaa)
                    | MeshPipelineViewLayoutKey::from(prepass_textures),
            );

            let mut entries = DynamicBindGroupEntries::new_with_indices((
                (0, view_binding.clone()),
                (1, light_binding.clone()),
                (2, &shadow_bindings.point_light_depth_texture_view),
                (3, &shadow_samplers.point_light_sampler),
                (4, &shadow_bindings.directional_light_depth_texture_view),
                (5, &shadow_samplers.directional_light_sampler),
                (6, point_light_binding.clone()),
                (7, cluster_bindings.light_index_lists_binding().unwrap()),
                (8, cluster_bindings.offsets_and_counts_binding().unwrap()),
                (9, globals.clone()),
                (10, fog_binding.clone()),
                (11, ssao_view),
            ));

            let env_map_bindings =
                environment_map::get_bindings(environment_map, &images, &fallback_cubemap);
            entries = entries.extend_with_indices((
                (12, env_map_bindings.0),
                (13, env_map_bindings.1),
                (14, env_map_bindings.2),
            ));

            let lut_bindings = get_lut_bindings(&images, &tonemapping_luts, tonemapping);
            entries = entries.extend_with_indices(((15, lut_bindings.0), (16, lut_bindings.1)));

            // When using WebGL, we can't have a depth texture with multisampling
            let prepass_bindings;
            if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32"))) || msaa.samples() == 1
            {
                prepass_bindings = prepass::get_bindings(prepass_textures);
                for (binding, index) in prepass_bindings
                    .iter()
                    .map(Option::as_ref)
                    .zip([17, 18, 19, 20])
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
                entries.extend_with_indices(((21, transmission_view), (22, transmission_sampler)));

            commands.entity(entity).insert(MeshViewBindGroup {
                value: render_device.create_bind_group("mesh_view_bind_group", layout, &entries),
            });
        }
    }
}
