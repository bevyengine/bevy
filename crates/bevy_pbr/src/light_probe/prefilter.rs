use bevy_asset::{weak_handle, Handle};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryState, With},
    resource::Resource,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_math::{UVec2, Vec2};
use bevy_render::{
    extract_component::ExtractComponent,
    render_graph::{Node, NodeRunError, RenderGraphContext, RenderLabel},
    render_resource::{
        binding_types::*, AddressMode, BindGroup, BindGroupEntries, BindGroupLayout,
        BindGroupLayoutEntries, BindingResource, BufferBinding, BufferInitDescriptor, BufferUsages,
        CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor, Extent3d,
        FilterMode, PipelineCache, Sampler, SamplerBindingType, SamplerDescriptor, Shader,
        ShaderDefVal, ShaderStages, ShaderType, StorageTextureAccess, TextureAspect,
        TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
        TextureView, TextureViewDescriptor, TextureViewDimension, UniformBuffer,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    settings::WgpuFeatures,
    texture::{CachedTexture, TextureCache},
    Extract,
};

/// A handle to the SPD (Single Pass Downsampling) shader.
pub const SPD_SHADER_HANDLE: Handle<Shader> = weak_handle!("5dcf400c-bcb3-49b9-8b7e-80f4117eaf82");

/// A handle to the importance sample shader.
pub const IMPORTANCE_SAMPLE_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("3110b545-78e0-48fc-b86e-8bc0ea50fc67");

/// Labels for the prefiltering nodes
#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, RenderLabel)]
pub enum PrefilterNode {
    GenerateMipmap,
    GenerateMipmapSecond,
    ImportanceSample,
    IrradianceMap,
}

/// Stores the bind group layouts for the prefiltering process
#[derive(Resource)]
pub struct PrefilterBindGroupLayouts {
    pub spd: BindGroupLayout,
    pub importance_sample: BindGroupLayout,
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

        // Importance sampling bind group layout
        let importance_sample = render_device.create_bind_group_layout(
            "importance_sample_bind_group_layout",
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
                    (3, uniform_buffer::<ImportanceSamplingConstants>(false)), // Uniforms
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
                    (3, uniform_buffer::<IrradianceConstants>(false)), // Uniforms
                ),
            ),
        );

        Self {
            spd,
            importance_sample,
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
    pub importance_sample: CachedComputePipelineId,
    pub irradiance: CachedComputePipelineId,
}

impl FromWorld for PrefilterPipelines {
    fn from_world(world: &mut World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let layouts = world.resource::<PrefilterBindGroupLayouts>();

        let render_device = world.resource::<RenderDevice>();
        let features = render_device.features();
        let shader_defs = if features.contains(WgpuFeatures::SUBGROUP) {
            println!("Subgroup support is enabled");
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

        // Importance Sampling for Specular Environment Maps
        let importance_sample = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("importance_sample_pipeline".into()),
            layout: vec![layouts.importance_sample.clone()],
            push_constant_ranges: vec![],
            shader: IMPORTANCE_SAMPLE_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "generate_radiance_map".into(),
            zero_initialize_workgroup_memory: false,
        });

        // Irradiance Convolution
        let irradiance = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("irradiance_pipeline".into()),
            layout: vec![layouts.irradiance.clone()],
            push_constant_ranges: vec![],
            shader: IMPORTANCE_SAMPLE_SHADER_HANDLE,
            shader_defs: vec![],
            entry_point: "generate_irradiance_map".into(),
            zero_initialize_workgroup_memory: false,
        });

        Self {
            spd_first,
            spd_second,
            importance_sample,
            irradiance,
        }
    }
}

#[derive(Component, Clone, ExtractComponent)]
pub struct FilteredEnvironmentMapLight {
    pub environment_map: CachedTexture,
}

pub fn extract_prefilter_entities(
    prefilter_query: Extract<Query<(Entity, &FilteredEnvironmentMapLight)>>,
    mut commands: Commands,
) {
    for (_, prefilter) in prefilter_query.iter() {
        commands.spawn((prefilter.clone(),));
    }
}

#[derive(Component)]
pub struct PrefilterTextures {
    pub environment_map: CachedTexture,
    pub specular_map: CachedTexture,
    pub irradiance_map: CachedTexture,
}

/// Prepares textures needed for prefiltering
pub fn prepare_prefilter_textures(
    light_probes: Query<Entity, With<FilteredEnvironmentMapLight>>,
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
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            },
        );

        // Create specular prefiltered maps
        let specular_map = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("prefilter_specular_map"),
                size: Extent3d {
                    width: 512,
                    height: 512,
                    depth_or_array_layers: 6, // Cubemap faces
                },
                mip_level_count: 9, // Different roughness values
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            },
        );

        // Create irradiance map (32x32 is enough for diffuse)
        let irradiance_map = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("prefilter_irradiance_map"),
                size: Extent3d {
                    width: 32,
                    height: 32,
                    depth_or_array_layers: 6, // Cubemap faces
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            },
        );

        commands.entity(entity).insert(PrefilterTextures {
            environment_map,
            specular_map,
            irradiance_map,
        });
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

/// Constants for importance sampling
#[derive(Clone, Copy, ShaderType)]
#[repr(C)]
pub struct ImportanceSamplingConstants {
    mip_level: f32,
    sample_count: u32,
    roughness: f32,
    _padding: u32,
}

/// Constants for irradiance convolution
#[derive(Clone, Copy, ShaderType)]
#[repr(C)]
pub struct IrradianceConstants {
    sample_count: u32,
    _padding1: u32,
    _padding2: u32,
    _padding3: u32,
}

/// Stores bind groups for the prefiltering process
#[derive(Component)]
pub struct PrefilterBindGroups {
    pub spd: BindGroup,
    pub importance_sample: Vec<BindGroup>, // One per mip level
    pub irradiance: BindGroup,
}

/// Prepares bind groups for prefiltering
pub fn prepare_prefilter_bind_groups(
    light_probes: Query<
        (Entity, &PrefilterTextures, &FilteredEnvironmentMapLight),
        With<FilteredEnvironmentMapLight>,
    >,
    render_device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    layouts: Res<PrefilterBindGroupLayouts>,
    samplers: Res<PrefilterSamplers>,
    mut commands: Commands,
) {
    for (entity, textures, prefilter) in &light_probes {
        // Create SPD bind group
        let spd_constants = SpdConstants {
            mips: 8,                                                 // Number of mip levels
            inverse_input_size: Vec2::new(1.0 / 512.0, 1.0 / 512.0), // 1.0 / input size
            _padding: 0,
        };

        let mut spd_constants_buffer = UniformBuffer::from(spd_constants);
        spd_constants_buffer.write_buffer(&render_device, &queue);

        let storage_view = prefilter
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
                (0, &storage_view),
                (
                    1,
                    &create_storage_view(&textures.specular_map, 1, &render_device),
                ),
                (
                    2,
                    &create_storage_view(&textures.specular_map, 2, &render_device),
                ),
                (
                    3,
                    &create_storage_view(&textures.specular_map, 3, &render_device),
                ),
                (
                    4,
                    &create_storage_view(&textures.specular_map, 4, &render_device),
                ),
                (
                    5,
                    &create_storage_view(&textures.specular_map, 5, &render_device),
                ),
                (
                    6,
                    &create_storage_view(&textures.specular_map, 6, &render_device),
                ),
                (
                    7,
                    &create_storage_view(&textures.specular_map, 7, &render_device),
                ),
                (
                    8,
                    &create_storage_view(&textures.specular_map, 8, &render_device),
                ),
                // (
                //     9,
                //     &create_storage_view(&textures.specular_map, 9, &render_device),
                // ),
                // (
                //     10,
                //     &create_storage_view(&textures.specular_map, 10, &render_device),
                // ),
                // (
                //     11,
                //     &create_storage_view(&textures.specular_map, 11, &render_device),
                // ),
                // (
                //     12,
                //     &create_storage_view(&textures.specular_map, 12, &render_device),
                // ),
                (13, &samplers.linear),
                (14, &spd_constants_buffer),
            )),
        );

        // Create importance sampling bind groups for each mip level
        let mut importance_sample_bind_groups = Vec::with_capacity(9);

        for mip in 0..9 {
            let roughness = if mip == 0 { 0.0 } else { (mip as f32) / 8.0 };

            let importance_constants = ImportanceSamplingConstants {
                mip_level: mip as f32,
                sample_count: 32, // Must match SAMPLE_COUNT in the shader
                roughness,
                _padding: 0,
            };

            let mut importance_constants_buffer = UniformBuffer::from(importance_constants);
            importance_constants_buffer.write_buffer(&render_device, &queue);

            let mip_storage_view =
                create_storage_view(&textures.specular_map, mip as u32, &render_device);

            let bind_group = render_device.create_bind_group(
                Some(format!("importance_sample_bind_group_mip_{}", mip).as_str()),
                &layouts.importance_sample,
                &BindGroupEntries::with_indices((
                    (0, &textures.environment_map.default_view),
                    (1, &samplers.linear),
                    (2, &mip_storage_view),
                    (3, &importance_constants_buffer),
                )),
            );

            importance_sample_bind_groups.push(bind_group);
        }

        // Create irradiance bind group
        let irradiance_constants = IrradianceConstants {
            sample_count: 64, // Higher for good diffuse approximation
            _padding1: 0,
            _padding2: 0,
            _padding3: 0,
        };

        let mut irradiance_constants_buffer = UniformBuffer::from(irradiance_constants);
        irradiance_constants_buffer.write_buffer(&render_device, &queue);

        let irradiance_bind_group = render_device.create_bind_group(
            "irradiance_bind_group",
            &layouts.irradiance,
            &BindGroupEntries::with_indices((
                (0, &textures.environment_map.default_view),
                (1, &samplers.linear),
                (2, &textures.irradiance_map.default_view),
                (
                    3,
                    &irradiance_constants_buffer,
                ),
            )),
        );

        commands.entity(entity).insert(PrefilterBindGroups {
            spd: spd_bind_group,
            importance_sample: importance_sample_bind_groups,
            irradiance: irradiance_bind_group,
        });
    }
}

/// Helper function to create a storage texture view for a specific mip level
fn create_storage_view(
    texture: &CachedTexture,
    mip: u32,
    _render_device: &RenderDevice,
) -> TextureView {
    texture.texture.create_view(&TextureViewDescriptor {
        label: Some(format!("storage_view_mip_{}", mip).as_str()),
        format: Some(texture.texture.format()),
        dimension: Some(TextureViewDimension::D2Array),
        aspect: TextureAspect::All,
        base_mip_level: mip,
        mip_level_count: Some(1),
        base_array_layer: 0,
        array_layer_count: Some(texture.texture.depth_or_array_layers()),
        usage: Some(TextureUsages::STORAGE_BINDING),
    })
}

/// SPD Node implementation for the first part (mips 0-5)
pub struct SpdFirstNode {
    query: QueryState<(Entity, Read<PrefilterBindGroups>)>,
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

        for (_, bind_groups) in self.query.iter_manual(world) {
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

/// Importance Sampling Node for generating specular environment maps
pub struct ImportanceSampleNode {
    query: QueryState<(Entity, Read<PrefilterBindGroups>)>,
}

impl FromWorld for ImportanceSampleNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for ImportanceSampleNode {
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

        let Some(importance_sample_pipeline) =
            pipeline_cache.get_compute_pipeline(pipelines.importance_sample)
        else {
            return Ok(());
        };

        for (_, bind_groups) in self.query.iter_manual(world) {
            let mut compute_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("importance_sample_pass"),
                        timestamp_writes: None,
                    });

            compute_pass.set_pipeline(importance_sample_pipeline);

            // Process each mip level
            for (mip, bind_group) in bind_groups.importance_sample.iter().enumerate() {
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
pub struct IrradianceNode {
    query: QueryState<(Entity, Read<PrefilterBindGroups>)>,
}

impl FromWorld for IrradianceNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for IrradianceNode {
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
                        label: Some("irradiance_pass"),
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
