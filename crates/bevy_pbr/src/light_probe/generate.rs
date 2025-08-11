//! Like [`EnvironmentMapLight`], but filtered in realtime from a cubemap.
//!
//! An environment map needs to be processed to be able to support uses beyond a simple skybox,
//! such as reflections, and ambient light contribution.
//! This process is called filtering, and can either be done ahead of time (prefiltering), or
//! in realtime, although at a reduced quality. Prefiltering is preferred, but not always possible:
//! sometimes you only gain access to an environment map at runtime, for whatever reason.
//! Typically this is from realtime reflection probes, but can also be from other sources.
//!
//! In any case, Bevy supports both modes of filtering.
//! This module provides realtime filtering via [`bevy_light::GeneratedEnvironmentMapLight`].
//! For prefiltered environment maps, see [`bevy_light::EnvironmentMapLight`].
//! These components are intended to be added to a camera.
use bevy_app::{App, Plugin, Update};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Assets};
use bevy_core_pipeline::core_3d::graph::{Core3d, Node3d};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryState, With, Without},
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_image::Image;
use bevy_math::{Quat, UVec2, Vec2};
use bevy_render::{
    diagnostic::RecordDiagnostics,
    render_asset::{RenderAssetUsages, RenderAssets},
    render_graph::{Node, NodeRunError, RenderGraphContext, RenderGraphExt, RenderLabel},
    render_resource::{
        binding_types::*, AddressMode, BindGroup, BindGroupEntries, BindGroupLayout,
        BindGroupLayoutEntries, CachedComputePipelineId, ComputePassDescriptor,
        ComputePipelineDescriptor, DownlevelFlags, Extent3d, FilterMode, PipelineCache, Sampler,
        SamplerBindingType, SamplerDescriptor, ShaderStages, ShaderType, StorageTextureAccess,
        Texture, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
        TextureFormatFeatureFlags, TextureSampleType, TextureUsages, TextureView,
        TextureViewDescriptor, TextureViewDimension, UniformBuffer,
    },
    renderer::{RenderAdapter, RenderContext, RenderDevice, RenderQueue},
    settings::WgpuFeatures,
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
    texture::{CachedTexture, GpuImage, TextureCache},
    Extract, ExtractSchedule, Render, RenderApp, RenderStartup, RenderSystems,
};

// Implementation: generate diffuse and specular cubemaps required by PBR
// from a given high-res cubemap by
//
// 1. Copying the base mip (level 0) of the source cubemap into an intermediate
//    storage texture.
// 2. Generating mipmaps using [single-pass down-sampling] (SPD).
// 3. Convolving the mip chain twice:
//    * a [Lambertian convolution] for the 32 × 32 diffuse cubemap
//    * a [GGX convolution], once per mip level, for the specular cubemap.
//
// [single-pass down-sampling]: https://gpuopen.com/fidelityfx-spd/
// [Lambertian convolution]: https://bruop.github.io/ibl/#:~:text=Lambertian%20Diffuse%20Component
// [GGX convolution]: https://gpuopen.com/download/Bounded_VNDF_Sampling_for_Smith-GGX_Reflections.pdf

use bevy_light::{EnvironmentMapLight, GeneratedEnvironmentMapLight};
use bevy_shader::ShaderDefVal;
use core::cmp::min;
use tracing::info;

use crate::Bluenoise;

/// Labels for the environment map generation nodes
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, RenderLabel)]
pub enum GeneratorNode {
    Downsampling,
    Filtering,
}

/// Stores the bind group layouts for the environment map generation pipelines
#[derive(Resource)]
pub struct GeneratorBindGroupLayouts {
    pub downsampling_first: BindGroupLayout,
    pub downsampling_second: BindGroupLayout,
    pub radiance: BindGroupLayout,
    pub irradiance: BindGroupLayout,
    pub copy: BindGroupLayout,
}

/// Samplers for the environment map generation pipelines
#[derive(Resource)]
pub struct GeneratorSamplers {
    pub linear: Sampler,
}

/// Pipelines for the environment map generation pipelines
#[derive(Resource)]
pub struct GeneratorPipelines {
    pub downsample_first: CachedComputePipelineId,
    pub downsample_second: CachedComputePipelineId,
    pub copy: CachedComputePipelineId,
    pub radiance: CachedComputePipelineId,
    pub irradiance: CachedComputePipelineId,
}

/// Configuration for downsampling strategy based on device limits
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct DownsamplingConfig {
    // can bind ≥12 storage textures and use read-write storage textures
    pub combine_bind_group: bool,
}

pub struct EnvironmentMapGenerationPlugin;

impl Plugin for EnvironmentMapGenerationPlugin {
    fn build(&self, _: &mut App) {}
    fn finish(&self, app: &mut App) {
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            let adapter = render_app.world().resource::<RenderAdapter>();
            let device = render_app.world().resource::<RenderDevice>();

            // Cubemap SPD requires at least 6 storage textures
            let limit_support = device.limits().max_storage_textures_per_shader_stage >= 6
                && device.limits().max_compute_workgroup_storage_size != 0
                && device.limits().max_compute_workgroup_size_x != 0;

            let downlevel_support = adapter
                .get_downlevel_capabilities()
                .flags
                .contains(DownlevelFlags::COMPUTE_SHADERS);

            if !limit_support || !downlevel_support {
                info!("Disabling EnvironmentMapGenerationPlugin because compute is not supported on this platform. This is safe to ignore if you are not using EnvironmentMapGenerationPlugin.");
                return;
            }
        } else {
            return;
        }

        embedded_asset!(app, "environment_filter.wgsl");
        embedded_asset!(app, "downsample.wgsl");
        embedded_asset!(app, "copy.wgsl");

        app.add_plugins(SyncComponentPlugin::<GeneratedEnvironmentMapLight>::default())
            .add_systems(Update, generate_environment_map_light);

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .add_render_graph_node::<DownsamplingNode>(Core3d, GeneratorNode::Downsampling)
            .add_render_graph_node::<FilteringNode>(Core3d, GeneratorNode::Filtering)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndPrepasses,
                    GeneratorNode::Downsampling,
                    GeneratorNode::Filtering,
                    Node3d::StartMainPass,
                ),
            )
            .add_systems(
                ExtractSchedule,
                extract_generated_environment_map_entities.after(generate_environment_map_light),
            )
            .add_systems(
                Render,
                prepare_generated_environment_map_bind_groups
                    .in_set(RenderSystems::PrepareBindGroups),
            )
            .add_systems(
                Render,
                prepare_generated_environment_map_intermediate_textures
                    .in_set(RenderSystems::PrepareResources),
            )
            .add_systems(
                RenderStartup,
                initialize_generated_environment_map_resources,
            );
    }
}

// The number of storage textures required to combine the bind group
const REQUIRED_STORAGE_TEXTURES: u32 = 12;

/// Initializes all render-world resources used by the environment-map generator once on
/// [`bevy_render::RenderStartup`].
pub fn initialize_generated_environment_map_resources(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_adapter: Res<RenderAdapter>,
    pipeline_cache: Res<PipelineCache>,
    asset_server: Res<AssetServer>,
) {
    // Determine whether we can use a single, large bind group for all mip outputs
    let storage_texture_limit = render_device.limits().max_storage_textures_per_shader_stage;

    // Determine whether we can read and write to the same rgba16f storage texture
    let read_write_support = render_adapter
        .get_texture_format_features(TextureFormat::Rgba16Float)
        .flags
        .contains(TextureFormatFeatureFlags::STORAGE_READ_WRITE);

    // Combine the bind group and use read-write storage if it is supported
    let combine_bind_group =
        storage_texture_limit >= REQUIRED_STORAGE_TEXTURES && read_write_support;

    // Output mips are write-only
    let mips =
        texture_storage_2d_array(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly);

    // Bind group layouts
    let (downsampling_first, downsampling_second) = if combine_bind_group {
        // One big bind group layout containing all outputs 1–12
        let downsampling = render_device.create_bind_group_layout(
            "downsampling_bind_group_layout_combined",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<DownsamplingConstants>(false),
                    texture_2d_array(TextureSampleType::Float { filterable: true }),
                    mips, // 1
                    mips, // 2
                    mips, // 3
                    mips, // 4
                    mips, // 5
                    texture_storage_2d_array(
                        TextureFormat::Rgba16Float,
                        StorageTextureAccess::ReadWrite,
                    ), // 6
                    mips, // 7
                    mips, // 8
                    mips, // 9
                    mips, // 10
                    mips, // 11
                    mips, // 12
                ),
            ),
        );

        (downsampling.clone(), downsampling)
    } else {
        // Split layout: first pass outputs 1–6, second pass outputs 7–12 (input mip6 read-only)

        let downsampling_first = render_device.create_bind_group_layout(
            "downsampling_first_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<DownsamplingConstants>(false),
                    // Input mip 0
                    texture_2d_array(TextureSampleType::Float { filterable: true }),
                    mips, // 1
                    mips, // 2
                    mips, // 3
                    mips, // 4
                    mips, // 5
                    mips, // 6
                ),
            ),
        );

        let downsampling_second = render_device.create_bind_group_layout(
            "downsampling_second_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<DownsamplingConstants>(false),
                    // Input mip 6
                    texture_2d_array(TextureSampleType::Float { filterable: true }),
                    mips, // 7
                    mips, // 8
                    mips, // 9
                    mips, // 10
                    mips, // 11
                    mips, // 12
                ),
            ),
        );

        (downsampling_first, downsampling_second)
    };
    let radiance = render_device.create_bind_group_layout(
        "radiance_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                // Source environment cubemap
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering), // Source sampler
                // Output specular map
                texture_storage_2d_array(
                    TextureFormat::Rgba16Float,
                    StorageTextureAccess::WriteOnly,
                ),
                uniform_buffer::<FilteringConstants>(false), // Uniforms
                texture_2d_array(TextureSampleType::Float { filterable: true }), // Blue noise texture
            ),
        ),
    );

    let irradiance = render_device.create_bind_group_layout(
        "irradiance_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                // Source environment cubemap
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering), // Source sampler
                // Output irradiance map
                texture_storage_2d_array(
                    TextureFormat::Rgba16Float,
                    StorageTextureAccess::WriteOnly,
                ),
                uniform_buffer::<FilteringConstants>(false), // Uniforms
                texture_2d_array(TextureSampleType::Float { filterable: true }), // Blue noise texture
            ),
        ),
    );

    let copy = render_device.create_bind_group_layout(
        "copy_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                // Source cubemap
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                // Destination mip0
                texture_storage_2d_array(
                    TextureFormat::Rgba16Float,
                    StorageTextureAccess::WriteOnly,
                ),
            ),
        ),
    );

    let layouts = GeneratorBindGroupLayouts {
        downsampling_first,
        downsampling_second,
        radiance,
        irradiance,
        copy,
    };

    // Samplers
    let linear = render_device.create_sampler(&SamplerDescriptor {
        label: Some("generator_linear_sampler"),
        address_mode_u: AddressMode::ClampToEdge,
        address_mode_v: AddressMode::ClampToEdge,
        address_mode_w: AddressMode::ClampToEdge,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mipmap_filter: FilterMode::Linear,
        ..Default::default()
    });

    let samplers = GeneratorSamplers { linear };

    // Pipelines
    let features = render_device.features();
    let mut shader_defs = vec![];
    if features.contains(WgpuFeatures::SUBGROUP) {
        shader_defs.push(ShaderDefVal::Int("SUBGROUP_SUPPORT".into(), 1));
    }
    if combine_bind_group {
        shader_defs.push(ShaderDefVal::Int("COMBINE_BIND_GROUP".into(), 1));
    }
    #[cfg(feature = "bluenoise_texture")]
    {
        shader_defs.push(ShaderDefVal::Int("HAS_BLUE_NOISE".into(), 1));
    }

    let downsampling_shader = load_embedded_asset!(asset_server.as_ref(), "downsample.wgsl");
    let env_filter_shader = load_embedded_asset!(asset_server.as_ref(), "environment_filter.wgsl");
    let copy_shader = load_embedded_asset!(asset_server.as_ref(), "copy.wgsl");

    // First pass for base mip Levels (0-5)
    let downsample_first = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("downsampling_first_pipeline".into()),
        layout: vec![layouts.downsampling_first.clone()],
        push_constant_ranges: vec![],
        shader: downsampling_shader.clone(),
        shader_defs: {
            let mut defs = shader_defs.clone();
            if !combine_bind_group {
                defs.push(ShaderDefVal::Int("FIRST_PASS".into(), 1));
            }
            defs
        },
        entry_point: Some("downsample_first".into()),
        zero_initialize_workgroup_memory: false,
    });

    let downsample_second = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("downsampling_second_pipeline".into()),
        layout: vec![layouts.downsampling_second.clone()],
        push_constant_ranges: vec![],
        shader: downsampling_shader,
        shader_defs: {
            let mut defs = shader_defs.clone();
            if !combine_bind_group {
                defs.push(ShaderDefVal::Int("SECOND_PASS".into(), 1));
            }
            defs
        },
        entry_point: Some("downsample_second".into()),
        zero_initialize_workgroup_memory: false,
    });

    // Radiance map for specular environment maps
    let radiance = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("radiance_pipeline".into()),
        layout: vec![layouts.radiance.clone()],
        push_constant_ranges: vec![],
        shader: env_filter_shader.clone(),
        shader_defs: shader_defs.clone(),
        entry_point: Some("generate_radiance_map".into()),
        zero_initialize_workgroup_memory: false,
    });

    // Irradiance map for diffuse environment maps
    let irradiance = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("irradiance_pipeline".into()),
        layout: vec![layouts.irradiance.clone()],
        push_constant_ranges: vec![],
        shader: env_filter_shader,
        shader_defs: shader_defs.clone(),
        entry_point: Some("generate_irradiance_map".into()),
        zero_initialize_workgroup_memory: false,
    });

    // Copy pipeline handles format conversion and populates mip0 when formats differ
    let copy_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("copy_pipeline".into()),
        layout: vec![layouts.copy.clone()],
        push_constant_ranges: vec![],
        shader: copy_shader,
        shader_defs: vec![],
        entry_point: Some("copy".into()),
        zero_initialize_workgroup_memory: false,
    });

    let pipelines = GeneratorPipelines {
        downsample_first,
        downsample_second,
        radiance,
        irradiance,
        copy: copy_pipeline,
    };

    // Insert all resources into the render world
    commands.insert_resource(layouts);
    commands.insert_resource(samplers);
    commands.insert_resource(pipelines);
    commands.insert_resource(DownsamplingConfig { combine_bind_group });
}

pub fn extract_generated_environment_map_entities(
    query: Extract<
        Query<(
            RenderEntity,
            &GeneratedEnvironmentMapLight,
            &EnvironmentMapLight,
        )>,
    >,
    mut commands: Commands,
    render_images: Res<RenderAssets<GpuImage>>,
) {
    for (entity, filtered_env_map, env_map_light) in query.iter() {
        let Some(env_map) = render_images.get(&filtered_env_map.environment_map) else {
            continue;
        };

        let diffuse_map = render_images.get(&env_map_light.diffuse_map);
        let specular_map = render_images.get(&env_map_light.specular_map);

        // continue if the diffuse map is not found
        if diffuse_map.is_none() || specular_map.is_none() {
            continue;
        }

        let diffuse_map = diffuse_map.unwrap();
        let specular_map = specular_map.unwrap();

        let render_filtered_env_map = RenderEnvironmentMap {
            environment_map: env_map.clone(),
            diffuse_map: diffuse_map.clone(),
            specular_map: specular_map.clone(),
            intensity: filtered_env_map.intensity,
            rotation: filtered_env_map.rotation,
            affects_lightmapped_mesh_diffuse: filtered_env_map.affects_lightmapped_mesh_diffuse,
        };
        commands
            .get_entity(entity)
            .expect("Entity not synced to render world")
            .insert(render_filtered_env_map);
    }
}

// A render-world specific version of FilteredEnvironmentMapLight that uses CachedTexture
#[derive(Component, Clone)]
pub struct RenderEnvironmentMap {
    pub environment_map: GpuImage,
    pub diffuse_map: GpuImage,
    pub specular_map: GpuImage,
    pub intensity: f32,
    pub rotation: Quat,
    pub affects_lightmapped_mesh_diffuse: bool,
}

#[derive(Component)]
pub struct IntermediateTextures {
    pub environment_map: CachedTexture,
}

/// Returns the total number of mip levels for the provided square texture size.
/// `size` must be a power of two greater than zero. For example, `size = 512` → `9`.
#[inline]
fn compute_mip_count(size: u32) -> u32 {
    debug_assert!(size.is_power_of_two());
    32 - size.leading_zeros()
}

/// Prepares textures needed for single pass downsampling
pub fn prepare_generated_environment_map_intermediate_textures(
    light_probes: Query<(Entity, &RenderEnvironmentMap)>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut commands: Commands,
) {
    for (entity, env_map_light) in &light_probes {
        let base_size = env_map_light.environment_map.size.width;
        let mip_level_count = compute_mip_count(base_size);

        let environment_map = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("intermediate_environment_map"),
                size: Extent3d {
                    width: base_size,
                    height: base_size,
                    depth_or_array_layers: 6, // Cubemap faces
                },
                mip_level_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::STORAGE_BINDING
                    | TextureUsages::COPY_DST,
                view_formats: &[],
            },
        );

        commands
            .entity(entity)
            .insert(IntermediateTextures { environment_map });
    }
}

/// Shader constants for downsampling algorithm
#[derive(Clone, Copy, ShaderType)]
#[repr(C)]
pub struct DownsamplingConstants {
    mips: u32,
    inverse_input_size: Vec2,
    _padding: u32,
}

/// Constants for filtering
#[derive(Clone, Copy, ShaderType)]
#[repr(C)]
pub struct FilteringConstants {
    mip_level: f32,
    sample_count: u32,
    roughness: f32,
    noise_size_bits: UVec2,
}

/// Stores bind groups for the environment map generation pipelines
#[derive(Component)]
pub struct GeneratorBindGroups {
    pub downsampling_first: BindGroup,
    pub downsampling_second: BindGroup,
    pub radiance: Vec<BindGroup>, // One per mip level
    pub irradiance: BindGroup,
    pub copy: BindGroup,
}

/// Prepares bind groups for environment map generation pipelines
pub fn prepare_generated_environment_map_bind_groups(
    light_probes: Query<
        (Entity, &IntermediateTextures, &RenderEnvironmentMap),
        With<RenderEnvironmentMap>,
    >,
    render_device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    layouts: Res<GeneratorBindGroupLayouts>,
    samplers: Res<GeneratorSamplers>,
    render_images: Res<RenderAssets<GpuImage>>,
    bluenoise: Res<Bluenoise>,
    config: Res<DownsamplingConfig>,
    mut commands: Commands,
) {
    // Skip until the blue-noise texture is available to avoid panicking.
    // The system will retry next frame once the asset has loaded.
    let Some(stbn_texture) = render_images.get(&bluenoise.texture) else {
        return;
    };

    assert!(stbn_texture.size.width.is_power_of_two());
    assert!(stbn_texture.size.height.is_power_of_two());
    let noise_size_bits = UVec2::new(
        stbn_texture.size.width.trailing_zeros(),
        stbn_texture.size.height.trailing_zeros(),
    );

    for (entity, textures, env_map_light) in &light_probes {
        // Determine mip chain based on input size
        let base_size = env_map_light.environment_map.size.width;
        let mip_count = compute_mip_count(base_size);
        let last_mip = mip_count - 1;
        let env_map_texture = env_map_light.environment_map.texture.clone();

        // Create downsampling constants
        let downsampling_constants = DownsamplingConstants {
            mips: mip_count - 1, // Number of mips we are generating (excluding mip 0)
            inverse_input_size: Vec2::new(1.0 / base_size as f32, 1.0 / base_size as f32),
            _padding: 0,
        };

        let mut downsampling_constants_buffer = UniformBuffer::from(downsampling_constants);
        downsampling_constants_buffer.write_buffer(&render_device, &queue);

        let input_env_map_first = env_map_texture.clone().create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        });

        // Utility closure to get a unique storage view for a given mip level.
        let mip_storage = |level: u32| {
            if level <= last_mip {
                create_storage_view(&textures.environment_map.texture, level, &render_device)
            } else {
                // Return a fresh 1×1 placeholder view so each binding has its own sub-resource and cannot alias.
                create_placeholder_storage_view(&render_device)
            }
        };

        // Depending on device limits, build either a combined or split bind group layout
        let (downsampling_first_bind_group, downsampling_second_bind_group) =
            if config.combine_bind_group {
                // Combined layout expects destinations 1–12 in both bind groups
                let bind_group = render_device.create_bind_group(
                    "downsampling_bind_group_combined_first",
                    &layouts.downsampling_first,
                    &BindGroupEntries::sequential((
                        &samplers.linear,
                        &downsampling_constants_buffer,
                        &input_env_map_first,
                        &mip_storage(1),
                        &mip_storage(2),
                        &mip_storage(3),
                        &mip_storage(4),
                        &mip_storage(5),
                        &mip_storage(6),
                        &mip_storage(7),
                        &mip_storage(8),
                        &mip_storage(9),
                        &mip_storage(10),
                        &mip_storage(11),
                        &mip_storage(12),
                    )),
                );

                (bind_group.clone(), bind_group)
            } else {
                // Split path requires a separate view for mip6 input
                let input_env_map_second = env_map_texture.create_view(&TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::D2Array),
                    base_mip_level: min(6, last_mip),
                    mip_level_count: Some(1),
                    ..Default::default()
                });

                // Split layout (current behavior)
                let first = render_device.create_bind_group(
                    "downsampling_first_bind_group",
                    &layouts.downsampling_first,
                    &BindGroupEntries::sequential((
                        &samplers.linear,
                        &downsampling_constants_buffer,
                        &input_env_map_first,
                        &mip_storage(1),
                        &mip_storage(2),
                        &mip_storage(3),
                        &mip_storage(4),
                        &mip_storage(5),
                        &mip_storage(6),
                    )),
                );

                let second = render_device.create_bind_group(
                    "downsampling_second_bind_group",
                    &layouts.downsampling_second,
                    &BindGroupEntries::sequential((
                        &samplers.linear,
                        &downsampling_constants_buffer,
                        &input_env_map_second,
                        &mip_storage(7),
                        &mip_storage(8),
                        &mip_storage(9),
                        &mip_storage(10),
                        &mip_storage(11),
                        &mip_storage(12),
                    )),
                );

                (first, second)
            };

        // create a 2d array view of the bluenoise texture
        let stbn_texture_view = stbn_texture
            .texture
            .clone()
            .create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2Array),
                ..Default::default()
            });

        // Create radiance map bind groups for each mip level
        let num_mips = mip_count as usize;
        let mut radiance_bind_groups = Vec::with_capacity(num_mips);

        for mip in 0..num_mips {
            // Calculate roughness from 0.0 (mip 0) to 0.889 (mip 8)
            // We don't need roughness=1.0 as a mip level because it's handled by the separate diffuse irradiance map
            let roughness = mip as f32 / (num_mips - 1) as f32;
            let sample_count = 32u32 * 2u32.pow((roughness * 4.0) as u32);

            let radiance_constants = FilteringConstants {
                mip_level: mip as f32,
                sample_count,
                roughness,
                noise_size_bits,
            };

            let mut radiance_constants_buffer = UniformBuffer::from(radiance_constants);
            radiance_constants_buffer.write_buffer(&render_device, &queue);

            let mip_storage_view = create_storage_view(
                &env_map_light.specular_map.texture,
                mip as u32,
                &render_device,
            );
            let bind_group = render_device.create_bind_group(
                Some(format!("radiance_bind_group_mip_{mip}").as_str()),
                &layouts.radiance,
                &BindGroupEntries::sequential((
                    &textures.environment_map.default_view,
                    &samplers.linear,
                    &mip_storage_view,
                    &radiance_constants_buffer,
                    &stbn_texture_view,
                )),
            );

            radiance_bind_groups.push(bind_group);
        }

        // Create irradiance bind group
        let irradiance_constants = FilteringConstants {
            mip_level: 0.0,
            // 32 phi, 32 theta = 1024 samples total
            sample_count: 1024,
            roughness: 1.0,
            noise_size_bits,
        };

        let mut irradiance_constants_buffer = UniformBuffer::from(irradiance_constants);
        irradiance_constants_buffer.write_buffer(&render_device, &queue);

        // create a 2d array view
        let irradiance_map =
            env_map_light
                .diffuse_map
                .texture
                .create_view(&TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::D2Array),
                    ..Default::default()
                });

        let irradiance_bind_group = render_device.create_bind_group(
            "irradiance_bind_group",
            &layouts.irradiance,
            &BindGroupEntries::sequential((
                &textures.environment_map.default_view,
                &samplers.linear,
                &irradiance_map,
                &irradiance_constants_buffer,
                &stbn_texture_view,
            )),
        );

        // Create copy bind group (source env map → destination mip0)
        let src_view = env_map_light
            .environment_map
            .texture
            .create_view(&TextureViewDescriptor {
                dimension: Some(TextureViewDimension::D2Array),
                ..Default::default()
            });

        let dst_view = create_storage_view(&textures.environment_map.texture, 0, &render_device);

        let copy_bind_group = render_device.create_bind_group(
            "copy_bind_group",
            &layouts.copy,
            &BindGroupEntries::with_indices(((0, &src_view), (1, &dst_view))),
        );

        commands.entity(entity).insert(GeneratorBindGroups {
            downsampling_first: downsampling_first_bind_group,
            downsampling_second: downsampling_second_bind_group,
            radiance: radiance_bind_groups,
            irradiance: irradiance_bind_group,
            copy: copy_bind_group,
        });
    }
}

/// Helper function to create a storage texture view for a specific mip level
fn create_storage_view(texture: &Texture, mip: u32, _render_device: &RenderDevice) -> TextureView {
    texture.create_view(&TextureViewDescriptor {
        label: Some(format!("storage_view_mip_{mip}").as_str()),
        format: Some(texture.format()),
        dimension: Some(TextureViewDimension::D2Array),
        aspect: TextureAspect::All,
        base_mip_level: mip,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(texture.depth_or_array_layers()),
        usage: Some(TextureUsages::STORAGE_BINDING),
    })
}

/// To ensure compatibility in web browsers, each call returns a unique resource so that multiple missing mip
/// bindings in the same bind-group never alias.
fn create_placeholder_storage_view(render_device: &RenderDevice) -> TextureView {
    let tex = render_device.create_texture(&TextureDescriptor {
        label: Some("lightprobe_placeholder"),
        size: Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 6,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba16Float,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    tex.create_view(&TextureViewDescriptor::default())
}

/// Downsampling node implementation that handles all parts of the mip chain
pub struct DownsamplingNode {
    query: QueryState<(
        Entity,
        Read<GeneratorBindGroups>,
        Read<RenderEnvironmentMap>,
    )>,
}

impl FromWorld for DownsamplingNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for DownsamplingNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<GeneratorPipelines>();

        let Some(downsample_first_pipeline) =
            pipeline_cache.get_compute_pipeline(pipelines.downsample_first)
        else {
            return Ok(());
        };

        let Some(downsample_second_pipeline) =
            pipeline_cache.get_compute_pipeline(pipelines.downsample_second)
        else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        for (_, bind_groups, env_map_light) in self.query.iter_manual(world) {
            // Copy base mip using compute shader with pre-built bind group
            let Some(copy_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.copy) else {
                return Ok(());
            };

            {
                let mut compute_pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("lightprobe_copy"),
                            timestamp_writes: None,
                        });

                let pass_span = diagnostics.pass_span(&mut compute_pass, "lightprobe_copy");

                compute_pass.set_pipeline(copy_pipeline);
                compute_pass.set_bind_group(0, &bind_groups.copy, &[]);

                let tex_size = env_map_light.environment_map.size;
                let wg_x = tex_size.width.div_ceil(8);
                let wg_y = tex_size.height.div_ceil(8);
                compute_pass.dispatch_workgroups(wg_x, wg_y, 6);

                pass_span.end(&mut compute_pass);
            }

            // First pass - process mips 0-5
            {
                let mut compute_pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("lightprobe_downsampling_first_pass"),
                            timestamp_writes: None,
                        });

                let pass_span =
                    diagnostics.pass_span(&mut compute_pass, "lightprobe_downsampling_first_pass");

                compute_pass.set_pipeline(downsample_first_pipeline);
                compute_pass.set_bind_group(0, &bind_groups.downsampling_first, &[]);

                let tex_size = env_map_light.environment_map.size;
                let wg_x = tex_size.width.div_ceil(64);
                let wg_y = tex_size.height.div_ceil(64);
                compute_pass.dispatch_workgroups(wg_x, wg_y, 6); // 6 faces

                pass_span.end(&mut compute_pass);
            }

            // Second pass - process mips 6-12
            {
                let mut compute_pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("lightprobe_downsampling_second_pass"),
                            timestamp_writes: None,
                        });

                let pass_span =
                    diagnostics.pass_span(&mut compute_pass, "lightprobe_downsampling_second_pass");

                compute_pass.set_pipeline(downsample_second_pipeline);
                compute_pass.set_bind_group(0, &bind_groups.downsampling_second, &[]);

                let tex_size = env_map_light.environment_map.size;
                let wg_x = tex_size.width.div_ceil(256);
                let wg_y = tex_size.height.div_ceil(256);
                compute_pass.dispatch_workgroups(wg_x, wg_y, 6);

                pass_span.end(&mut compute_pass);
            }
        }

        Ok(())
    }
}

/// Radiance map node for generating specular environment maps
pub struct FilteringNode {
    query: QueryState<(
        Entity,
        Read<GeneratorBindGroups>,
        Read<RenderEnvironmentMap>,
    )>,
}

impl FromWorld for FilteringNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for FilteringNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipelines = world.resource::<GeneratorPipelines>();

        let Some(radiance_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.radiance)
        else {
            return Ok(());
        };
        let Some(irradiance_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.irradiance)
        else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        for (_, bind_groups, env_map_light) in self.query.iter_manual(world) {
            let mut compute_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("lightprobe_radiance_map"),
                        timestamp_writes: None,
                    });

            let pass_span = diagnostics.pass_span(&mut compute_pass, "lightprobe_radiance_map");

            compute_pass.set_pipeline(radiance_pipeline);

            let base_size = env_map_light.specular_map.size.width;

            // Radiance convolution pass
            // Process each mip at different roughness levels
            for (mip, bind_group) in bind_groups.radiance.iter().enumerate() {
                compute_pass.set_bind_group(0, bind_group, &[]);

                // Calculate dispatch size based on mip level
                let mip_size = base_size >> mip;
                let workgroup_count = mip_size.div_ceil(8);

                // Dispatch for all 6 faces
                compute_pass.dispatch_workgroups(workgroup_count, workgroup_count, 6);
            }
            pass_span.end(&mut compute_pass);
            // End the compute pass before starting the next one
            drop(compute_pass);

            // Irradiance convolution pass
            // Generate the diffuse environment map
            {
                let mut compute_pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("lightprobe_irradiance_map"),
                            timestamp_writes: None,
                        });

                let irr_span =
                    diagnostics.pass_span(&mut compute_pass, "lightprobe_irradiance_map");

                compute_pass.set_pipeline(irradiance_pipeline);
                compute_pass.set_bind_group(0, &bind_groups.irradiance, &[]);

                // 32×32 texture processed with 8×8 workgroups for all 6 faces
                compute_pass.dispatch_workgroups(4, 4, 6);

                irr_span.end(&mut compute_pass);
            }
        }

        Ok(())
    }
}

/// System that generates an `EnvironmentMapLight` component based on the `GeneratedEnvironmentMapLight` component
pub fn generate_environment_map_light(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    query: Query<(Entity, &GeneratedEnvironmentMapLight), Without<EnvironmentMapLight>>,
) {
    for (entity, filtered_env_map) in &query {
        // Validate and fetch the source cubemap so we can size our targets correctly
        let Some(src_image) = images.get(&filtered_env_map.environment_map) else {
            // Texture not ready yet – try again next frame
            continue;
        };

        let base_size = src_image.texture_descriptor.size.width;

        // Sanity checks – square, power-of-two, ≤ 8192
        if src_image.texture_descriptor.size.height != base_size
            || !base_size.is_power_of_two()
            || base_size > 8192
        {
            panic!(
                "GeneratedEnvironmentMapLight source cubemap must be square power-of-two ≤ 8192, got {}×{}",
                base_size, src_image.texture_descriptor.size.height
            );
        }

        let mip_count = compute_mip_count(base_size);

        // Create a placeholder for the irradiance map
        let mut diffuse = Image::new_fill(
            Extent3d {
                width: 32,
                height: 32,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            &[0; 8],
            TextureFormat::Rgba16Float,
            RenderAssetUsages::all(),
        );

        diffuse.texture_descriptor.usage =
            TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING;

        diffuse.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            ..Default::default()
        });

        let diffuse_handle = images.add(diffuse);

        // Create a placeholder for the specular map. It matches the input cubemap resolution.
        let mut specular = Image::new_fill(
            Extent3d {
                width: base_size,
                height: base_size,
                depth_or_array_layers: 6,
            },
            TextureDimension::D2,
            &[0; 8],
            TextureFormat::Rgba16Float,
            RenderAssetUsages::all(),
        );

        // Set up for mipmaps
        specular.texture_descriptor.usage =
            TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING;
        specular.texture_descriptor.mip_level_count = mip_count;

        // When setting mip_level_count, we need to allocate appropriate data size
        // For GPU-generated mipmaps, we can set data to None since the GPU will generate the data
        specular.data = None;

        specular.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            mip_level_count: Some(mip_count),
            ..Default::default()
        });

        let specular_handle = images.add(specular);

        // Add the EnvironmentMapLight component with the placeholder handles
        commands.entity(entity).insert(EnvironmentMapLight {
            diffuse_map: diffuse_handle,
            specular_map: specular_handle,
            intensity: filtered_env_map.intensity,
            rotation: filtered_env_map.rotation,
            affects_lightmapped_mesh_diffuse: filtered_env_map.affects_lightmapped_mesh_diffuse,
        });
    }
}
