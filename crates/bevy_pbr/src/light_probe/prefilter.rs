use bevy_asset::{weak_handle, Assets, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryState, With, Without},
    resource::Resource,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_image::Image;
use bevy_math::{Quat, Vec2};
use bevy_reflect::Reflect;
use bevy_render::{
    render_asset::{RenderAssetUsages, RenderAssets},
    render_graph::{Node, NodeRunError, RenderGraphContext, RenderLabel},
    render_resource::{
        binding_types::*, AddressMode, BindGroup, BindGroupEntries, BindGroupLayout,
        BindGroupLayoutEntries, CachedComputePipelineId, ComputePassDescriptor,
        ComputePipelineDescriptor, Extent3d, FilterMode, PipelineCache, Sampler,
        SamplerBindingType, SamplerDescriptor, Shader, ShaderDefVal, ShaderStages, ShaderType,
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

use crate::atmosphere;
use crate::light_probe::environment_map::EnvironmentMapLight;

/// A handle to the SPD (Single Pass Downsampling) shader.
pub const SPD_SHADER_HANDLE: Handle<Shader> = weak_handle!("5dcf400c-bcb3-49b9-8b7e-80f4117eaf82");

/// A handle to the environment filter shader.
pub const ENVIRONMENT_FILTER_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("3110b545-78e0-48fc-b86e-8bc0ea50fc67");

/// Labels for the prefiltering nodes
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, RenderLabel)]
pub enum PrefilterNode {
    GenerateMipmap,
    GenerateMipmapSecond,
    RadianceMap,
    IrradianceMap,
}

/// Stores the bind group layouts for the prefiltering process
#[derive(Resource)]
pub struct PrefilterBindGroupLayouts {
    pub spd: BindGroupLayout,
    pub radiance: BindGroupLayout,
    pub irradiance: BindGroupLayout,
}

impl FromWorld for PrefilterBindGroupLayouts {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // SPD (Single Pass Downsampling) bind group layout
        let spd = render_device.create_bind_group_layout(
            "spd_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (
                        0,
                        texture_2d_array(TextureSampleType::Float { filterable: true }),
                    ), // Source texture
                    (
                        1,
                        texture_storage_2d_array(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ), // Output mip 1
                    (
                        2,
                        texture_storage_2d_array(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ), // Output mip 2
                    (
                        3,
                        texture_storage_2d_array(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ), // Output mip 3
                    (
                        4,
                        texture_storage_2d_array(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ), // Output mip 4
                    (
                        5,
                        texture_storage_2d_array(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ), // Output mip 5
                    (
                        6,
                        texture_storage_2d_array(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::ReadWrite,
                        ),
                    ), // Output mip 6
                    (
                        7,
                        texture_storage_2d_array(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ), // Output mip 7
                    (
                        8,
                        texture_storage_2d_array(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ), // Output mip 8
                    // (
                    //     9,
                    //     texture_storage_2d_array(
                    //         TextureFormat::Rgba16Float,
                    //         StorageTextureAccess::WriteOnly,
                    //     ),
                    // ), // Output mip 9
                    // (
                    //     10,
                    //     texture_storage_2d_array(
                    //         TextureFormat::Rgba16Float,
                    //         StorageTextureAccess::WriteOnly,
                    //     ),
                    // ), // Output mip 10
                    // (
                    //     11,
                    //     texture_storage_2d_array(
                    //         TextureFormat::Rgba16Float,
                    //         StorageTextureAccess::WriteOnly,
                    //     ),
                    // ), // Output mip 11
                    // (
                    //     12,
                    //     texture_storage_2d_array(
                    //         TextureFormat::Rgba16Float,
                    //         StorageTextureAccess::WriteOnly,
                    //     ),
                    // ), // Output mip 12
                    (13, sampler(SamplerBindingType::Filtering)), // Linear sampler
                    (14, uniform_buffer::<SpdConstants>(false)),  // Uniforms
                ),
            ),
        );

        // Radiance map bind group layout
        let radiance = render_device.create_bind_group_layout(
            "radiance_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (
                        0,
                        texture_2d_array(TextureSampleType::Float { filterable: true }),
                    ), // Source environment cubemap
                    (1, sampler(SamplerBindingType::Filtering)), // Source sampler
                    (
                        2,
                        texture_storage_2d_array(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ), // Output specular map
                    (3, uniform_buffer::<PrefilterConstants>(false)), // Uniforms
                    (4, texture_2d(TextureSampleType::Float { filterable: true })), // Blue noise texture
                ),
            ),
        );

        // Irradiance convolution bind group layout
        let irradiance = render_device.create_bind_group_layout(
            "irradiance_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (
                        0,
                        texture_2d_array(TextureSampleType::Float { filterable: true }),
                    ), // Source environment cubemap
                    (1, sampler(SamplerBindingType::Filtering)), // Source sampler
                    (
                        2,
                        texture_storage_2d_array(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ), // Output irradiance map
                    (3, uniform_buffer::<PrefilterConstants>(false)), // Uniforms
                    (4, texture_2d(TextureSampleType::Float { filterable: true })), // Blue noise texture
                ),
            ),
        );

        Self {
            spd,
            radiance,
            irradiance,
        }
    }
}

/// Samplers for the prefiltering process
#[derive(Resource)]
pub struct PrefilterSamplers {
    pub linear: Sampler,
}

impl FromWorld for PrefilterSamplers {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let linear = render_device.create_sampler(&SamplerDescriptor {
            label: Some("prefilter_linear_sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Linear,
            ..Default::default()
        });

        Self { linear }
    }
}

/// Pipelines for the prefiltering process
#[derive(Resource)]
pub struct PrefilterPipelines {
    pub spd_first: CachedComputePipelineId,
    pub spd_second: CachedComputePipelineId,
    pub radiance: CachedComputePipelineId,
    pub irradiance: CachedComputePipelineId,
}

impl FromWorld for PrefilterPipelines {
    fn from_world(world: &mut World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let layouts = world.resource::<PrefilterBindGroupLayouts>();

        let render_device = world.resource::<RenderDevice>();
        let features = render_device.features();
        let shader_defs = if features.contains(WgpuFeatures::SUBGROUP) {
            vec![ShaderDefVal::Int("SUBGROUP_SUPPORT".into(), 1)]
        } else {
            vec![]
        };

        // Single Pass Downsampling for Base Mip Levels (0-5)
        let spd_first = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("spd_first_pipeline".into()),
            layout: vec![layouts.spd.clone()],
            push_constant_ranges: vec![],
            shader: SPD_SHADER_HANDLE,
            shader_defs: shader_defs.clone(),
            entry_point: "spd_downsample_first".into(),
            zero_initialize_workgroup_memory: false,
        });

        // Single Pass Downsampling for Remaining Mip Levels (6-12)
        let spd_second = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("spd_second_pipeline".into()),
            layout: vec![layouts.spd.clone()],
            push_constant_ranges: vec![],
            shader: SPD_SHADER_HANDLE,
            shader_defs,
            entry_point: "spd_downsample_second".into(),
            zero_initialize_workgroup_memory: false,
        });

        // Radiance map for Specular Environment Maps
        let radiance = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("radiance_pipeline".into()),
            layout: vec![layouts.radiance.clone()],
            push_constant_ranges: vec![],
            shader: ENVIRONMENT_FILTER_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "generate_radiance_map".into(),
            zero_initialize_workgroup_memory: false,
        });

        // Irradiance map for Diffuse Environment Maps
        let irradiance = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("irradiance_pipeline".into()),
            layout: vec![layouts.irradiance.clone()],
            push_constant_ranges: vec![],
            shader: ENVIRONMENT_FILTER_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "generate_irradiance_map".into(),
            zero_initialize_workgroup_memory: false,
        });

        Self {
            spd_first,
            spd_second,
            radiance,
            irradiance,
        }
    }
}

#[derive(Component, Clone, Reflect)]
pub struct FilteredEnvironmentMapLight {
    pub environment_map: Handle<Image>,
    pub intensity: f32,
    pub rotation: Quat,
    pub affects_lightmapped_mesh_diffuse: bool,
}

pub fn extract_prefilter_entities(
    prefilter_query: Extract<
        Query<(
            RenderEntity,
            &FilteredEnvironmentMapLight,
            &EnvironmentMapLight,
        )>,
    >,
    mut commands: Commands,
    render_images: Res<RenderAssets<GpuImage>>,
) {
    for (entity, filtered_env_map, env_map_light) in prefilter_query.iter() {
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
pub struct PrefilterTextures {
    pub environment_map: CachedTexture,
}

/// Prepares textures needed for prefiltering
pub fn prepare_prefilter_textures(
    light_probes: Query<Entity, With<RenderEnvironmentMap>>,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    mut commands: Commands,
) {
    for entity in &light_probes {
        // Create environment map with 8 mip levels (512x512 -> 1x1)
        let environment_map = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("prefilter_environment_map"),
                size: Extent3d {
                    width: 512,
                    height: 512,
                    depth_or_array_layers: 6, // Cubemap faces
                },
                mip_level_count: 9, // 512, 256, 128, 64, 32, 16, 8, 4, 2, 1
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
            .insert(PrefilterTextures { environment_map });
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

/// Constants for prefiltering
#[derive(Clone, Copy, ShaderType)]
#[repr(C)]
pub struct PrefilterConstants {
    mip_level: f32,
    sample_count: u32,
    roughness: f32,
    blue_noise_size: Vec2,
}

/// Stores bind groups for the prefiltering process
#[derive(Component)]
pub struct PrefilterBindGroups {
    pub spd: BindGroup,
    pub radiance: Vec<BindGroup>, // One per mip level
    pub irradiance: BindGroup,
}

/// Prepares bind groups for prefiltering
pub fn prepare_prefilter_bind_groups(
    light_probes: Query<
        (Entity, &PrefilterTextures, &RenderEnvironmentMap),
        With<RenderEnvironmentMap>,
    >,
    render_device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    layouts: Res<PrefilterBindGroupLayouts>,
    samplers: Res<PrefilterSamplers>,
    render_images: Res<RenderAssets<GpuImage>>,
    mut commands: Commands,
) {
    // Get blue noise texture
    let blue_noise_texture = render_images
        .get(&atmosphere::shaders::BLUENOISE_TEXTURE)
        .expect("Blue noise texture not loaded");

    for (entity, textures, env_map_light) in &light_probes {
        // Create SPD bind group
        let spd_constants = SpdConstants {
            mips: 8,                                                 // Number of mip levels
            inverse_input_size: Vec2::new(1.0 / 512.0, 1.0 / 512.0), // 1.0 / input size
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

        let spd_bind_group = render_device.create_bind_group(
            "spd_bind_group",
            &layouts.spd,
            &BindGroupEntries::with_indices((
                (0, &input_env_map),
                (
                    1,
                    &create_storage_view(&textures.environment_map.texture, 1, &render_device),
                ),
                (
                    2,
                    &create_storage_view(&textures.environment_map.texture, 2, &render_device),
                ),
                (
                    3,
                    &create_storage_view(&textures.environment_map.texture, 3, &render_device),
                ),
                (
                    4,
                    &create_storage_view(&textures.environment_map.texture, 4, &render_device),
                ),
                (
                    5,
                    &create_storage_view(&textures.environment_map.texture, 5, &render_device),
                ),
                (
                    6,
                    &create_storage_view(&textures.environment_map.texture, 6, &render_device),
                ),
                (
                    7,
                    &create_storage_view(&textures.environment_map.texture, 7, &render_device),
                ),
                (
                    8,
                    &create_storage_view(&textures.environment_map.texture, 8, &render_device),
                ),
                // (
                //     9,
                //     &create_storage_view(&textures.environment_map.texture, 9, &render_device),
                // ),
                // (
                //     10,
                //     &create_storage_view(&textures.environment_map.texture, 10, &render_device),
                // ),
                // (
                //     11,
                //     &create_storage_view(&textures.environment_map.texture, 11, &render_device),
                // ),
                // (
                //     12,
                //     &create_storage_view(&textures.environment_map.texture, 12, &render_device),
                // ),
                (13, &samplers.linear),
                (14, &spd_constants_buffer),
            )),
        );

        // Create radiance map bind groups for each mip level
        let num_mips = 9;
        let mut radiance_bind_groups = Vec::with_capacity(num_mips);

        for mip in 0..num_mips {
            let roughness = mip as f32 / (num_mips - 1) as f32;

            // For higher roughness values, use importance sampling with optimized sample count
            let sample_count = if roughness < 0.01 {
                1 // Mirror reflection
            } else if roughness < 0.25 {
                16
            } else if roughness < 0.5 {
                32
            } else if roughness < 0.75 {
                64
            } else {
                128
            };

            let radiance_constants = PrefilterConstants {
                mip_level: mip as f32,
                sample_count,
                roughness,
                blue_noise_size: Vec2::new(
                    blue_noise_texture.size.width as f32,
                    blue_noise_texture.size.height as f32,
                ),
            };

            let mut radiance_constants_buffer = UniformBuffer::from(radiance_constants);
            radiance_constants_buffer.write_buffer(&render_device, &queue);

            let mip_storage_view = create_storage_view(
                &env_map_light.specular_map.texture,
                mip as u32,
                &render_device,
            );
            let bind_group = render_device.create_bind_group(
                Some(format!("radiance_bind_group_mip_{}", mip).as_str()),
                &layouts.radiance,
                &BindGroupEntries::with_indices((
                    (0, &textures.environment_map.default_view),
                    (1, &samplers.linear),
                    (2, &mip_storage_view),
                    (3, &radiance_constants_buffer),
                    (4, &blue_noise_texture.texture_view),
                )),
            );

            radiance_bind_groups.push(bind_group);
        }

        // Create irradiance bind group
        let irradiance_constants = PrefilterConstants {
            mip_level: 0.0,
            sample_count: 64,
            roughness: 1.0,
            blue_noise_size: Vec2::new(
                blue_noise_texture.size.width as f32,
                blue_noise_texture.size.height as f32,
            ),
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
            &BindGroupEntries::with_indices((
                (0, &textures.environment_map.default_view),
                (1, &samplers.linear),
                (2, &irradiance_map),
                (3, &irradiance_constants_buffer),
                (4, &blue_noise_texture.texture_view),
            )),
        );

        commands.entity(entity).insert(PrefilterBindGroups {
            spd: spd_bind_group,
            radiance: radiance_bind_groups,
            irradiance: irradiance_bind_group,
        });
    }
}

/// Helper function to create a storage texture view for a specific mip level
fn create_storage_view(texture: &Texture, mip: u32, _render_device: &RenderDevice) -> TextureView {
    texture.create_view(&TextureViewDescriptor {
        label: Some(format!("storage_view_mip_{}", mip).as_str()),
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

/// SPD Node implementation for the first part (mips 0-5)
pub struct SpdFirstNode {
    query: QueryState<(
        Entity,
        Read<PrefilterBindGroups>,
        Read<RenderEnvironmentMap>,
    )>,
}

impl FromWorld for SpdFirstNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for SpdFirstNode {
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
        let pipelines = world.resource::<PrefilterPipelines>();

        let Some(spd_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.spd_first) else {
            return Ok(());
        };

        for (entity, bind_groups, env_map_light) in self.query.iter_manual(world) {
            // Copy original environment map to mip 0 of the intermediate environment map
            let textures = world.get::<PrefilterTextures>(entity).unwrap();

            render_context.command_encoder().copy_texture_to_texture(
                env_map_light.environment_map.texture.as_image_copy(),
                textures.environment_map.texture.as_image_copy(),
                Extent3d {
                    width: 512,
                    height: 512,
                    depth_or_array_layers: 6,
                },
            );

            let mut compute_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("spd_first_pass"),
                        timestamp_writes: None,
                    });

            compute_pass.set_pipeline(spd_pipeline);
            compute_pass.set_bind_group(0, &bind_groups.spd, &[]);

            // Calculate the optimal dispatch size based on our shader's workgroup size and thread mapping
            // The workgroup size is 256x1x1, and our remap_for_wave_reduction maps these threads to a 8x8 block
            // For a 512x512 texture, we need 512/64 = 8 workgroups in X and 512/64 = 8 workgroups in Y
            // Each workgroup processes 64x64 pixels (256 threads each handling 16 pixels)
            compute_pass.dispatch_workgroups(8, 8, 6); // 6 faces of cubemap
        }

        Ok(())
    }
}

/// SPD Node implementation for the second part (mips 6-12)
pub struct SpdSecondNode {
    query: QueryState<(Entity, Read<PrefilterBindGroups>)>,
}

impl FromWorld for SpdSecondNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for SpdSecondNode {
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
        let pipelines = world.resource::<PrefilterPipelines>();

        let Some(spd_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.spd_second) else {
            return Ok(());
        };

        for (_, bind_groups) in self.query.iter_manual(world) {
            let mut compute_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("spd_second_pass"),
                        timestamp_writes: None,
                    });

            compute_pass.set_pipeline(spd_pipeline);
            compute_pass.set_bind_group(0, &bind_groups.spd, &[]);

            // Dispatch workgroups - for each face
            compute_pass.dispatch_workgroups(2, 2, 6);
        }

        Ok(())
    }
}

/// Radiance map node for generating specular environment maps
pub struct RadianceMapNode {
    query: QueryState<(Entity, Read<PrefilterBindGroups>)>,
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
        let pipelines = world.resource::<PrefilterPipelines>();

        let Some(radiance_pipeline) = pipeline_cache.get_compute_pipeline(pipelines.radiance)
        else {
            return Ok(());
        };

        for (_, bind_groups) in self.query.iter_manual(world) {
            let mut compute_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("radiance_map_pass"),
                        timestamp_writes: None,
                    });

            compute_pass.set_pipeline(radiance_pipeline);

            // Process each mip level
            for (mip, bind_group) in bind_groups.radiance.iter().enumerate() {
                compute_pass.set_bind_group(0, bind_group, &[]);

                // Calculate dispatch size based on mip level
                let mip_size = 512u32 >> mip;
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
    query: QueryState<(Entity, Read<PrefilterBindGroups>)>,
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
        let pipelines = world.resource::<PrefilterPipelines>();

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

/// System that creates an `EnvironmentMapLight` component from the prefiltered textures
pub fn create_environment_map_from_prefilter(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    query: Query<(Entity, &FilteredEnvironmentMapLight), Without<EnvironmentMapLight>>,
) {
    for (entity, filtered_env_map) in &query {
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

        // Create a placeholder for the specular map
        let mut specular = Image::new_fill(
            Extent3d {
                width: 512,
                height: 512,
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
        specular.texture_descriptor.mip_level_count = 9;

        // When setting mip_level_count, we need to allocate appropriate data size
        // For GPU-generated mipmaps, we can set data to None since the GPU will generate the data
        specular.data = None;

        specular.texture_view_descriptor = Some(TextureViewDescriptor {
            dimension: Some(TextureViewDimension::Cube),
            mip_level_count: Some(9),
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
