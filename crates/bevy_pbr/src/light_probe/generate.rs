//! Generated environment map filtering.
//!
//! A *generated environment map* converts a single, high-resolution cubemap
//! into the pair of diffuse and specular cubemaps required by the PBR
//! shader. Add [`bevy_light::GeneratedEnvironmentMapLight`] to a camera
//! and Bevy will, each frame, generate the diffuse and specular cubemaps
//! required by the PBR shader.
//!
//! 1. Copy the base mip (level 0) of the source cubemap into an intermediate
//!    storage texture.
//! 2. Generate mipmaps using [single-pass down-sampling] (SPD).
//! 3. Convolve the mip chain twice:
//!    * a [Lambertian convolution] for the 32 × 32 diffuse cubemap
//!    * a [GGX convolution], once per mip level, for the specular cubemap.
//!
//! The filtered results are then consumed exactly like the textures supplied
//! by [`bevy_light::EnvironmentMapLight`]. This is useful when you only have a
//! raw HDR environment map or when you need reflections generated at run time.
//!
//! [single-pass down-sampling]: https://gpuopen.com/fidelityfx-spd/
//! [Lambertian convolution]: https://bruop.github.io/ibl/#:~:text=Lambertian%20Diffuse%20Component
//! [GGX convolution]: https://gpuopen.com/download/Bounded_VNDF_Sampling_for_Smith-GGX_Reflections.pdf
use bevy_asset::{load_embedded_asset, uuid_handle, AssetServer, Assets, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryState, With, Without},
    resource::Resource,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_image::Image;
use bevy_math::{Quat, UVec2, Vec2};
use bevy_render::{
    render_asset::{RenderAssetUsages, RenderAssets},
    render_graph::{Node, NodeRunError, RenderGraphContext, RenderLabel},
    render_resource::{
        binding_types::*, AddressMode, BindGroup, BindGroupEntries, BindGroupLayout,
        BindGroupLayoutEntries, CachedComputePipelineId, ComputePassDescriptor,
        ComputePipelineDescriptor, Extent3d, FilterMode, PipelineCache, Sampler,
        SamplerBindingType, SamplerDescriptor, ShaderDefVal, ShaderStages, ShaderType,
        StorageTextureAccess, Texture, TextureAspect, TextureDescriptor, TextureDimension,
        TextureFormat, TextureSampleType, TextureUsages, TextureView, TextureViewDescriptor,
        TextureViewDimension, UniformBuffer,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    settings::WgpuFeatures,
    sync_world::RenderEntity,
    texture::{CachedTexture, GpuImage, TextureCache},
    Extract,
};

use bevy_light::{EnvironmentMapLight, GeneratedEnvironmentMapLight};
use core::cmp::min;

/// Handle for Spatio-Temporal Blue Noise texture
pub const STBN: Handle<Image> = uuid_handle!("3110b545-78e0-48fc-b86e-8bc0ea50fc67");

/// Labels for the environment map generation nodes
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, RenderLabel)]
pub enum GeneratorNode {
    Mipmap,
    Radiance,
    Irradiance,
}

/// Stores the bind group layouts for the environment map generation pipelines
#[derive(Resource)]
pub struct GeneratorBindGroupLayouts {
    pub spd: BindGroupLayout,
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
    pub spd_first: CachedComputePipelineId,
    pub spd_second: CachedComputePipelineId,
    pub radiance: CachedComputePipelineId,
    pub irradiance: CachedComputePipelineId,
    pub copy: CachedComputePipelineId,
}

/// Initializes all render-world resources used by the environment-map generator once on
/// [`bevy_render::RenderStartup`].
pub fn init_generator_resources(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    asset_server: Res<AssetServer>,
) {
    let mips =
        texture_storage_2d_array(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly);
    // Bind group layouts
    let spd = render_device.create_bind_group_layout(
        "spd_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                // Source texture
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                mips, // Output mip 1
                mips, // Output mip 2
                mips, // Output mip 3
                mips, // Output mip 4
                mips, // Output mip 5
                // Output mip 6
                texture_storage_2d_array(
                    TextureFormat::Rgba16Float,
                    StorageTextureAccess::ReadWrite,
                ),
                mips, // Output mip 7
                mips, // Output mip 8
                mips, // Output mip 9
                mips, // Output mip 10
                mips, // Output mip 11
                mips, // Output mip 12
                // Linear sampler
                sampler(SamplerBindingType::Filtering),
                // Uniforms
                uniform_buffer::<SpdConstants>(false),
            ),
        ),
    );

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
                texture_2d(TextureSampleType::Float { filterable: true }), // Blue noise texture
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
                texture_2d(TextureSampleType::Float { filterable: true }), // Blue noise texture
            ),
        ),
    );

    let copy = render_device.create_bind_group_layout(
        "copy_mip0_bind_group_layout",
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
        spd,
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
    let shader_defs = if features.contains(WgpuFeatures::SUBGROUP) {
        vec![ShaderDefVal::Int("SUBGROUP_SUPPORT".into(), 1)]
    } else {
        vec![]
    };

    let spd_shader = load_embedded_asset!(asset_server.as_ref(), "spd.wgsl");
    let env_filter_shader = load_embedded_asset!(asset_server.as_ref(), "environment_filter.wgsl");
    let copy_shader = load_embedded_asset!(asset_server.as_ref(), "copy_mip0.wgsl");

    // Single Pass Downsampling for Base Mip Levels (0-5)
    let spd_first = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("spd_first_pipeline".into()),
        layout: vec![layouts.spd.clone()],
        push_constant_ranges: vec![],
        shader: spd_shader.clone(),
        shader_defs: shader_defs.clone(),
        entry_point: Some("spd_downsample_first".into()),
        zero_initialize_workgroup_memory: false,
    });

    // Single Pass Downsampling for Remaining Mip Levels (6-12)
    let spd_second = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("spd_second_pipeline".into()),
        layout: vec![layouts.spd.clone()],
        push_constant_ranges: vec![],
        shader: spd_shader,
        shader_defs: shader_defs.clone(),
        entry_point: Some("spd_downsample_second".into()),
        zero_initialize_workgroup_memory: false,
    });

    // Radiance map for Specular Environment Maps
    let radiance = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("radiance_pipeline".into()),
        layout: vec![layouts.radiance.clone()],
        push_constant_ranges: vec![],
        shader: env_filter_shader.clone(),
        shader_defs: vec![],
        entry_point: Some("generate_radiance_map".into()),
        zero_initialize_workgroup_memory: false,
    });

    // Irradiance map for Diffuse Environment Maps
    let irradiance = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("irradiance_pipeline".into()),
        layout: vec![layouts.irradiance.clone()],
        push_constant_ranges: vec![],
        shader: env_filter_shader,
        shader_defs: vec![],
        entry_point: Some("generate_irradiance_map".into()),
        zero_initialize_workgroup_memory: false,
    });

    // Copy pipeline handles format conversion and populates mip0 when formats differ
    let copy_pipeline = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
        label: Some("copy_mip0_pipeline".into()),
        layout: vec![layouts.copy.clone()],
        push_constant_ranges: vec![],
        shader: copy_shader,
        shader_defs: vec![],
        entry_point: Some("copy_mip0".into()),
        zero_initialize_workgroup_memory: false,
    });

    let pipelines = GeneratorPipelines {
        spd_first,
        spd_second,
        radiance,
        irradiance,
        copy: copy_pipeline,
    };

    // Insert all resources into the render world
    commands.insert_resource(layouts);
    commands.insert_resource(samplers);
    commands.insert_resource(pipelines);
}

pub fn extract_generator_entities(
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
        let env_map = render_images
            .get(&filtered_env_map.environment_map)
            .expect("Environment map not found");

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
pub fn prepare_intermediate_textures(
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

/// Shader constants for SPD algorithm
#[derive(Clone, Copy, ShaderType)]
#[repr(C)]
pub struct SpdConstants {
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
    pub spd: BindGroup,
    pub radiance: Vec<BindGroup>, // One per mip level
    pub irradiance: BindGroup,
    pub copy: BindGroup,
}

/// Prepares bind groups for environment map generation pipelines
pub fn prepare_generator_bind_groups(
    light_probes: Query<
        (Entity, &IntermediateTextures, &RenderEnvironmentMap),
        With<RenderEnvironmentMap>,
    >,
    render_device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    layouts: Res<GeneratorBindGroupLayouts>,
    samplers: Res<GeneratorSamplers>,
    render_images: Res<RenderAssets<GpuImage>>,
    mut commands: Commands,
) {
    let stbn_texture = render_images.get(&STBN).expect("STBN texture not loaded");
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

        // Create SPD constants
        let spd_constants = SpdConstants {
            mips: mip_count - 1, // Number of mips we are generating (excluding mip 0)
            inverse_input_size: Vec2::new(1.0 / base_size as f32, 1.0 / base_size as f32),
            _padding: 0,
        };

        let mut spd_constants_buffer = UniformBuffer::from(spd_constants);
        spd_constants_buffer.write_buffer(&render_device, &queue);

        let input_env_map =
            env_map_light
                .environment_map
                .texture
                .create_view(&TextureViewDescriptor {
                    dimension: Some(TextureViewDimension::D2Array),
                    ..Default::default()
                });

        let mip_storage = |level| {
            create_storage_view(
                &textures.environment_map.texture,
                min(level, last_mip),
                &render_device,
            )
        };

        let spd_bind_group = render_device.create_bind_group(
            "spd_bind_group",
            &layouts.spd,
            &BindGroupEntries::sequential((
                // Source mip0
                &input_env_map,
                // Destination mips 1 – 12 (duplicate the last valid view if the chain is shorter)
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
                &samplers.linear,
                &spd_constants_buffer,
            )),
        );

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
                    &stbn_texture.texture_view,
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
                &stbn_texture.texture_view,
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
            "copy_mip0_bind_group",
            &layouts.copy,
            &BindGroupEntries::with_indices(((0, &src_view), (1, &dst_view))),
        );

        commands.entity(entity).insert(GeneratorBindGroups {
            spd: spd_bind_group,
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

/// SPD Node implementation that handles both parts of the downsampling (mips 0-12)
pub struct SpdNode {
    query: QueryState<(
        Entity,
        Read<GeneratorBindGroups>,
        Read<RenderEnvironmentMap>,
    )>,
}

impl FromWorld for SpdNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for SpdNode {
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

        // First pass (mips 0-5)
        let Some(spd_first_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.spd_first)
        else {
            return Ok(());
        };

        // Second pass (mips 6-12)
        let Some(spd_second_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.spd_second)
        else {
            return Ok(());
        };

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
                            label: Some("copy_mip0_pass"),
                            timestamp_writes: None,
                        });

                compute_pass.set_pipeline(copy_pipeline);
                compute_pass.set_bind_group(0, &bind_groups.copy, &[]);

                let tex_size = env_map_light.environment_map.size;
                let wg_x = (tex_size.width / 8).max(1);
                let wg_y = (tex_size.height / 8).max(1);
                compute_pass.dispatch_workgroups(wg_x, wg_y, 6);
            }

            // First pass - process mips 0-5
            {
                let mut compute_pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("spd_first_pass"),
                            timestamp_writes: None,
                        });

                compute_pass.set_pipeline(spd_first_pipeline);
                compute_pass.set_bind_group(0, &bind_groups.spd, &[]);

                let tex_size = env_map_light.environment_map.size;
                let wg_x = (tex_size.width / 64).max(1);
                let wg_y = (tex_size.height / 64).max(1);
                compute_pass.dispatch_workgroups(wg_x, wg_y, 6); // 6 faces
            }

            // Second pass - process mips 6-12
            {
                let mut compute_pass =
                    render_context
                        .command_encoder()
                        .begin_compute_pass(&ComputePassDescriptor {
                            label: Some("spd_second_pass"),
                            timestamp_writes: None,
                        });

                compute_pass.set_pipeline(spd_second_pipeline);
                compute_pass.set_bind_group(0, &bind_groups.spd, &[]);

                let tex_size = env_map_light.environment_map.size;
                let wg_x = (tex_size.width / 256).max(1);
                let wg_y = (tex_size.height / 256).max(1);
                compute_pass.dispatch_workgroups(wg_x, wg_y, 6);
            }
        }

        Ok(())
    }
}

/// Radiance map node for generating specular environment maps
pub struct RadianceMapNode {
    query: QueryState<(
        Entity,
        Read<GeneratorBindGroups>,
        Read<RenderEnvironmentMap>,
    )>,
}

impl FromWorld for RadianceMapNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for RadianceMapNode {
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

        for (_, bind_groups, env_map_light) in self.query.iter_manual(world) {
            let mut compute_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("radiance_map_pass"),
                        timestamp_writes: None,
                    });

            compute_pass.set_pipeline(radiance_pipeline);

            let base_size = env_map_light.specular_map.size.width;

            // Process each mip level
            for (mip, bind_group) in bind_groups.radiance.iter().enumerate() {
                compute_pass.set_bind_group(0, bind_group, &[]);

                // Calculate dispatch size based on mip level
                let mip_size = base_size >> mip;
                let workgroup_count = mip_size.max(8) / 8;

                // Dispatch for all 6 faces
                compute_pass.dispatch_workgroups(workgroup_count, workgroup_count, 6);
            }
        }

        Ok(())
    }
}

/// Irradiance Convolution Node
pub struct IrradianceMapNode {
    query: QueryState<(Entity, Read<GeneratorBindGroups>)>,
}

impl FromWorld for IrradianceMapNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for IrradianceMapNode {
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

        let Some(irradiance_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.irradiance)
        else {
            return Ok(());
        };

        for (_, bind_groups) in self.query.iter_manual(world) {
            let mut compute_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("irradiance_map_pass"),
                        timestamp_writes: None,
                    });

            compute_pass.set_pipeline(irradiance_pipeline);
            compute_pass.set_bind_group(0, &bind_groups.irradiance, &[]);

            // Dispatch workgroups - 32x32 texture with 8x8 workgroups
            compute_pass.dispatch_workgroups(4, 4, 6); // 6 faces
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
