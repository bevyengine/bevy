use crate::NodePbr;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, Handle};
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    prelude::Camera3d,
    prepass::{DepthPrepass, NormalPrepass, ViewPrepassTextures},
};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::{Has, QueryItem, With},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::{ExtractedCamera, TemporalJitter},
    extract_component::ExtractComponent,
    frame_graph::{
        BindGroupHandle, EncoderCommandBuilder, FrameGraph, FrameGraphTexture, ResourceMaterial,
        ResourceMeta, TextureInfo, TextureViewInfo,
    },
    globals::{GlobalsBuffer, GlobalsUniform},
    prelude::Camera,
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::{
        binding_types::{
            sampler, texture_2d, texture_depth_2d, texture_storage_2d, uniform_buffer,
        },
        *,
    },
    renderer::{RenderAdapter, RenderDevice, RenderQueue},
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
    view::{Msaa, ViewUniform, ViewUniformOffset, ViewUniforms},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_utils::default;
use core::mem;
use tracing::{error, warn};

const PREPROCESS_DEPTH_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("b7f2cc3d-c935-4f5c-9ae2-43d6b0d5659a");
const SSAO_SHADER_HANDLE: Handle<Shader> = weak_handle!("9ea355d7-37a2-4cc4-b4d1-5d8ab47b07f5");
const SPATIAL_DENOISE_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("0f2764a0-b343-471b-b7ce-ef5d636f4fc3");
const SSAO_UTILS_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("da53c78d-f318-473e-bdff-b388bc50ada2");

/// Plugin for screen space ambient occlusion.
pub struct ScreenSpaceAmbientOcclusionPlugin;

impl Plugin for ScreenSpaceAmbientOcclusionPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PREPROCESS_DEPTH_SHADER_HANDLE,
            "preprocess_depth.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(app, SSAO_SHADER_HANDLE, "ssao.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            SPATIAL_DENOISE_SHADER_HANDLE,
            "spatial_denoise.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            SSAO_UTILS_SHADER_HANDLE,
            "ssao_utils.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<ScreenSpaceAmbientOcclusion>();

        app.add_plugins(SyncComponentPlugin::<ScreenSpaceAmbientOcclusion>::default());
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        if !render_app
            .world()
            .resource::<RenderAdapter>()
            .get_texture_format_features(TextureFormat::R16Float)
            .allowed_usages
            .contains(TextureUsages::STORAGE_BINDING)
        {
            warn!("ScreenSpaceAmbientOcclusionPlugin not loaded. GPU lacks support: TextureFormat::R16Float does not support TextureUsages::STORAGE_BINDING.");
            return;
        }

        if render_app
            .world()
            .resource::<RenderDevice>()
            .limits()
            .max_storage_textures_per_shader_stage
            < 5
        {
            warn!("ScreenSpaceAmbientOcclusionPlugin not loaded. GPU lacks support: Limits::max_storage_textures_per_shader_stage is less than 5.");
            return;
        }

        render_app
            .init_resource::<SsaoPipelines>()
            .init_resource::<SpecializedComputePipelines<SsaoPipelines>>()
            .add_systems(ExtractSchedule, extract_ssao_settings)
            .add_systems(
                Render,
                (
                    prepare_ssao_pipelines.in_set(RenderSet::Prepare),
                    prepare_ssao_textures.in_set(RenderSet::PrepareResources),
                    prepare_ssao_bind_groups.in_set(RenderSet::PrepareBindGroups),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<SsaoNode>>(
                Core3d,
                NodePbr::ScreenSpaceAmbientOcclusion,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    // END_PRE_PASSES -> SCREEN_SPACE_AMBIENT_OCCLUSION -> MAIN_PASS
                    Node3d::EndPrepasses,
                    NodePbr::ScreenSpaceAmbientOcclusion,
                    Node3d::StartMainPass,
                ),
            );
    }
}

/// Component to apply screen space ambient occlusion to a 3d camera.
///
/// Screen space ambient occlusion (SSAO) approximates small-scale,
/// local occlusion of _indirect_ diffuse light between objects, based on what's visible on-screen.
/// SSAO does not apply to direct lighting, such as point or directional lights.
///
/// This darkens creases, e.g. on staircases, and gives nice contact shadows
/// where objects meet, giving entities a more "grounded" feel.
///
/// # Usage Notes
///
/// Requires that you add [`ScreenSpaceAmbientOcclusionPlugin`] to your app.
///
/// It strongly recommended that you use SSAO in conjunction with
/// TAA (`TemporalAntiAliasing`).
/// Doing so greatly reduces SSAO noise.
///
/// SSAO is not supported on `WebGL2`, and is not currently supported on `WebGPU`.
#[derive(Component, ExtractComponent, Reflect, PartialEq, Clone, Debug)]
#[reflect(Component, Debug, Default, PartialEq, Clone)]
#[require(DepthPrepass, NormalPrepass)]
#[doc(alias = "Ssao")]
pub struct ScreenSpaceAmbientOcclusion {
    /// Quality of the SSAO effect.
    pub quality_level: ScreenSpaceAmbientOcclusionQualityLevel,
    /// A constant estimated thickness of objects.
    ///
    /// This value is used to decide how far behind an object a ray of light needs to be in order
    /// to pass behind it. Any ray closer than that will be occluded.
    pub constant_object_thickness: f32,
}

impl Default for ScreenSpaceAmbientOcclusion {
    fn default() -> Self {
        Self {
            quality_level: ScreenSpaceAmbientOcclusionQualityLevel::default(),
            constant_object_thickness: 0.25,
        }
    }
}

#[derive(Reflect, PartialEq, Eq, Hash, Clone, Copy, Default, Debug)]
#[reflect(PartialEq, Hash, Clone, Default)]
pub enum ScreenSpaceAmbientOcclusionQualityLevel {
    Low,
    Medium,
    #[default]
    High,
    Ultra,
    Custom {
        /// Higher slice count means less noise, but worse performance.
        slice_count: u32,
        /// Samples per slice side is also tweakable, but recommended to be left at 2 or 3.
        samples_per_slice_side: u32,
    },
}

impl ScreenSpaceAmbientOcclusionQualityLevel {
    fn sample_counts(&self) -> (u32, u32) {
        match self {
            Self::Low => (1, 2),    // 4 spp (1 * (2 * 2)), plus optional temporal samples
            Self::Medium => (2, 2), // 8 spp (2 * (2 * 2)), plus optional temporal samples
            Self::High => (3, 3),   // 18 spp (3 * (3 * 2)), plus optional temporal samples
            Self::Ultra => (9, 3),  // 54 spp (9 * (3 * 2)), plus optional temporal samples
            Self::Custom {
                slice_count: slices,
                samples_per_slice_side,
            } => (*slices, *samples_per_slice_side),
        }
    }
}

#[derive(Default)]
struct SsaoNode {}

impl ViewNode for SsaoNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static SsaoPipelineId,
        &'static SsaoBindGroups,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (camera, pipeline_id, bind_groups, view_uniform_offset): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipelines = world.resource::<SsaoPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let (Some(camera_size), Some(_), Some(_), Some(_)) = (
            camera.physical_viewport_size,
            pipeline_cache.get_compute_pipeline(pipelines.preprocess_depth_pipeline),
            pipeline_cache.get_compute_pipeline(pipelines.spatial_denoise_pipeline),
            pipeline_cache.get_compute_pipeline(pipeline_id.0),
        ) else {
            return Ok(());
        };

        let mut pass_builder = frame_graph.create_pass_builder("ssao_node");

        pass_builder.push_debug_group("ssao");

        {
            pass_builder
                .create_compute_pass_builder("ssao_preprocess_depth_pass")
                .set_compute_pipeline(pipelines.preprocess_depth_pipeline)
                .set_bind_group_handle(0, &bind_groups.preprocess_depth_bind_group, &[])
                .set_bind_group_handle(
                    1,
                    &bind_groups.common_bind_group,
                    &[view_uniform_offset.offset],
                )
                .dispatch_workgroups(camera_size.x.div_ceil(16), camera_size.y.div_ceil(16), 1);
        }

        {
            pass_builder
                .create_compute_pass_builder("ssao_ssao_pass")
                .set_compute_pipeline(pipeline_id.0)
                .set_bind_group_handle(0, &bind_groups.ssao_bind_group, &[])
                .set_bind_group_handle(
                    1,
                    &bind_groups.common_bind_group,
                    &[view_uniform_offset.offset],
                )
                .dispatch_workgroups(camera_size.x.div_ceil(8), camera_size.y.div_ceil(8), 1);
        }

        {
            pass_builder
                .create_compute_pass_builder("ssao_spatial_denoise_pass")
                .set_compute_pipeline(pipelines.spatial_denoise_pipeline)
                .set_bind_group_handle(0, &bind_groups.spatial_denoise_bind_group, &[])
                .set_bind_group_handle(
                    1,
                    &bind_groups.common_bind_group,
                    &[view_uniform_offset.offset],
                )
                .dispatch_workgroups(camera_size.x.div_ceil(8), camera_size.y.div_ceil(8), 1);
        }

        pass_builder.pop_debug_group();

        Ok(())
    }
}

#[derive(Resource)]
struct SsaoPipelines {
    preprocess_depth_pipeline: CachedComputePipelineId,
    spatial_denoise_pipeline: CachedComputePipelineId,

    common_bind_group_layout: BindGroupLayout,
    preprocess_depth_bind_group_layout: BindGroupLayout,
    ssao_bind_group_layout: BindGroupLayout,
    spatial_denoise_bind_group_layout: BindGroupLayout,

    hilbert_index_lut: Texture,
    point_clamp_sampler: Sampler,
    linear_clamp_sampler: Sampler,
}

impl FromWorld for SsaoPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let hilbert_index_lut = render_device.create_texture_with_data(
            render_queue,
            &(TextureDescriptor {
                label: Some("ssao_hilbert_index_lut"),
                size: Extent3d {
                    width: HILBERT_WIDTH as u32,
                    height: HILBERT_WIDTH as u32,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R16Uint,
                usage: TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            }),
            TextureDataOrder::default(),
            bytemuck::cast_slice(&generate_hilbert_index_lut()),
        );

        let point_clamp_sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });
        let linear_clamp_sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        let common_bind_group_layout = render_device.create_bind_group_layout(
            "ssao_common_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    sampler(SamplerBindingType::NonFiltering),
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<ViewUniform>(true),
                ),
            ),
        );

        let preprocess_depth_bind_group_layout = render_device.create_bind_group_layout(
            "ssao_preprocess_depth_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_depth_2d(),
                    texture_storage_2d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        );

        let ssao_bind_group_layout = render_device.create_bind_group_layout(
            "ssao_ssao_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_2d(TextureSampleType::Uint),
                    texture_storage_2d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::R32Uint, StorageTextureAccess::WriteOnly),
                    uniform_buffer::<GlobalsUniform>(false),
                    uniform_buffer::<f32>(false),
                ),
            ),
        );

        let spatial_denoise_bind_group_layout = render_device.create_bind_group_layout(
            "ssao_spatial_denoise_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_2d(TextureSampleType::Uint),
                    texture_storage_2d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        );

        let preprocess_depth_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("ssao_preprocess_depth_pipeline".into()),
                layout: vec![
                    preprocess_depth_bind_group_layout.clone(),
                    common_bind_group_layout.clone(),
                ],
                push_constant_ranges: vec![],
                shader: PREPROCESS_DEPTH_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "preprocess_depth".into(),
                zero_initialize_workgroup_memory: false,
            });

        let spatial_denoise_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("ssao_spatial_denoise_pipeline".into()),
                layout: vec![
                    spatial_denoise_bind_group_layout.clone(),
                    common_bind_group_layout.clone(),
                ],
                push_constant_ranges: vec![],
                shader: SPATIAL_DENOISE_SHADER_HANDLE,
                shader_defs: Vec::new(),
                entry_point: "spatial_denoise".into(),
                zero_initialize_workgroup_memory: false,
            });

        Self {
            preprocess_depth_pipeline,
            spatial_denoise_pipeline,

            common_bind_group_layout,
            preprocess_depth_bind_group_layout,
            ssao_bind_group_layout,
            spatial_denoise_bind_group_layout,

            hilbert_index_lut,
            point_clamp_sampler,
            linear_clamp_sampler,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct SsaoPipelineKey {
    quality_level: ScreenSpaceAmbientOcclusionQualityLevel,
    temporal_jitter: bool,
}

impl SpecializedComputePipeline for SsaoPipelines {
    type Key = SsaoPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let (slice_count, samples_per_slice_side) = key.quality_level.sample_counts();

        let mut shader_defs = vec![
            ShaderDefVal::Int("SLICE_COUNT".to_string(), slice_count as i32),
            ShaderDefVal::Int(
                "SAMPLES_PER_SLICE_SIDE".to_string(),
                samples_per_slice_side as i32,
            ),
        ];

        if key.temporal_jitter {
            shader_defs.push("TEMPORAL_JITTER".into());
        }

        ComputePipelineDescriptor {
            label: Some("ssao_ssao_pipeline".into()),
            layout: vec![
                self.ssao_bind_group_layout.clone(),
                self.common_bind_group_layout.clone(),
            ],
            push_constant_ranges: vec![],
            shader: SSAO_SHADER_HANDLE,
            shader_defs,
            entry_point: "ssao".into(),
            zero_initialize_workgroup_memory: false,
        }
    }
}

fn extract_ssao_settings(
    mut commands: Commands,
    cameras: Extract<
        Query<
            (RenderEntity, &Camera, &ScreenSpaceAmbientOcclusion, &Msaa),
            (With<Camera3d>, With<DepthPrepass>, With<NormalPrepass>),
        >,
    >,
) {
    for (entity, camera, ssao_settings, msaa) in &cameras {
        if *msaa != Msaa::Off {
            error!(
                "SSAO is being used which requires Msaa::Off, but Msaa is currently set to Msaa::{:?}",
                *msaa
            );
            return;
        }
        let mut entity_commands = commands
            .get_entity(entity)
            .expect("SSAO entity wasn't synced.");
        if camera.is_active {
            entity_commands.insert(ssao_settings.clone());
        } else {
            entity_commands.remove::<ScreenSpaceAmbientOcclusion>();
        }
    }
}

#[derive(Component)]
pub struct ScreenSpaceAmbientOcclusionResources {
    pub preprocessed_depth_texture: ResourceMeta<FrameGraphTexture>,
    pub ssao_noisy_texture: ResourceMeta<FrameGraphTexture>, // Pre-spatially denoised texture
    pub screen_space_ambient_occlusion_texture: ResourceMeta<FrameGraphTexture>, // Spatially denoised texture
    pub depth_differences_texture: ResourceMeta<FrameGraphTexture>,
    pub thickness_buffer: Buffer,
}

impl ScreenSpaceAmbientOcclusionResources {
    pub fn get_preprocessed_depth_texture_key(entity: Entity) -> String {
        format!("preprocessed_depth_texture_{}", entity)
    }

    pub fn get_ssao_noisy_texture_key(entity: Entity) -> String {
        format!("ssao_noisy_texture_{}", entity)
    }

    pub fn get_ssao_texture_key(entity: Entity) -> String {
        format!("ssao_texture_{}", entity)
    }

    pub fn get_ssao_depth_differences_texture_key(entity: Entity) -> String {
        format!("ssao_depth_differences_texture_{}", entity)
    }
}

fn prepare_ssao_textures(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera, &ScreenSpaceAmbientOcclusion)>,
) {
    for (entity, camera, ssao_settings) in &views {
        let Some(physical_viewport_size) = camera.physical_viewport_size else {
            continue;
        };
        let size = Extent3d {
            width: physical_viewport_size.x,
            height: physical_viewport_size.y,
            depth_or_array_layers: 1,
        };

        let preprocessed_depth_texture = ResourceMeta {
            key: ScreenSpaceAmbientOcclusionResources::get_preprocessed_depth_texture_key(entity),
            desc: TextureInfo {
                label: Some("ssao_preprocessed_depth_texture".into()),
                size,
                mip_level_count: 5,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: vec![],
            },
        };

        let ssao_noisy_texture = ResourceMeta {
            key: ScreenSpaceAmbientOcclusionResources::get_ssao_noisy_texture_key(entity),
            desc: TextureInfo {
                label: Some("ssao_noisy_texture".into()),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: vec![],
            },
        };

        let ssao_texture = ResourceMeta {
            key: ScreenSpaceAmbientOcclusionResources::get_ssao_texture_key(entity),
            desc: TextureInfo {
                label: Some("ssao_texture".into()),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: vec![],
            },
        };

        let depth_differences_texture = ResourceMeta {
            key: ScreenSpaceAmbientOcclusionResources::get_ssao_depth_differences_texture_key(
                entity,
            ),
            desc: TextureInfo {
                label: Some("ssao_depth_differences_texture".into()),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Uint,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: vec![],
            },
        };

        let thickness_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("thickness_buffer"),
            contents: &ssao_settings.constant_object_thickness.to_le_bytes(),
            usage: BufferUsages::UNIFORM,
        });

        commands
            .entity(entity)
            .insert(ScreenSpaceAmbientOcclusionResources {
                preprocessed_depth_texture,
                ssao_noisy_texture,
                screen_space_ambient_occlusion_texture: ssao_texture,
                depth_differences_texture,
                thickness_buffer,
            });
    }
}

#[derive(Component)]
struct SsaoPipelineId(CachedComputePipelineId);

fn prepare_ssao_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedComputePipelines<SsaoPipelines>>,
    pipeline: Res<SsaoPipelines>,
    views: Query<(Entity, &ScreenSpaceAmbientOcclusion, Has<TemporalJitter>)>,
) {
    for (entity, ssao_settings, temporal_jitter) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            SsaoPipelineKey {
                quality_level: ssao_settings.quality_level,
                temporal_jitter,
            },
        );

        commands.entity(entity).insert(SsaoPipelineId(pipeline_id));
    }
}

#[derive(Component)]
struct SsaoBindGroups {
    common_bind_group: BindGroupHandle,
    preprocess_depth_bind_group: BindGroupHandle,
    ssao_bind_group: BindGroupHandle,
    spatial_denoise_bind_group: BindGroupHandle,
}

fn create_depth_view(mip_level: u32) -> TextureViewInfo {
    TextureViewInfo {
        label: Some("ssao_preprocessed_depth_texture_mip_view".into()),
        base_mip_level: mip_level,
        format: Some(TextureFormat::R16Float),
        dimension: Some(TextureViewDimension::D2),
        mip_level_count: Some(1),
        ..default()
    }
}

fn prepare_ssao_bind_groups(
    mut commands: Commands,
    pipelines: Res<SsaoPipelines>,
    view_uniforms: Res<ViewUniforms>,
    global_uniforms: Res<GlobalsBuffer>,
    views: Query<(
        Entity,
        &ScreenSpaceAmbientOcclusionResources,
        &ViewPrepassTextures,
    )>,
    mut frame_graph: ResMut<FrameGraph>,
) {
    let (Some(view_uniforms_handle), Some(globals_uniforms_handle)) = (
        view_uniforms
            .uniforms
            .make_binding_resource_handle(&mut frame_graph),
        global_uniforms
            .buffer
            .make_binding_resource_handle(&mut frame_graph),
    ) else {
        return;
    };

    for (entity, ssao_resources, prepass_textures) in &views {
        let common_bind_group = frame_graph
            .create_bind_group_handle_builder(
                Some("ssao_common_bind_group".into()),
                &pipelines.common_bind_group_layout,
            )
            .add_handle(0, &pipelines.point_clamp_sampler)
            .add_handle(1, &pipelines.linear_clamp_sampler)
            .add_handle(2, &view_uniforms_handle)
            .build();

        let depth = prepass_textures
            .depth
            .as_ref()
            .unwrap()
            .texture
            .make_binding_resource_handle(&mut frame_graph);

        let preprocessed_depth_texture_handle = ssao_resources
            .preprocessed_depth_texture
            .imported(&mut frame_graph);

        let preprocess_depth_bind_group = frame_graph
            .create_bind_group_handle_builder(
                Some("ssao_preprocess_depth_bind_group".into()),
                &pipelines.preprocess_depth_bind_group_layout,
            )
            .add_handle(0, &depth)
            .add_handle(
                1,
                (&preprocessed_depth_texture_handle, &create_depth_view(0)),
            )
            .add_handle(
                2,
                (&preprocessed_depth_texture_handle, &create_depth_view(1)),
            )
            .add_handle(
                3,
                (&preprocessed_depth_texture_handle, &create_depth_view(2)),
            )
            .add_handle(
                4,
                (&preprocessed_depth_texture_handle, &create_depth_view(3)),
            )
            .add_handle(
                5,
                (&preprocessed_depth_texture_handle, &create_depth_view(4)),
            )
            .build();

        let hilbert_index_lut_handle = pipelines.hilbert_index_lut.imported(&mut frame_graph);

        let thickness_buffer_handle = ssao_resources.thickness_buffer.imported(&mut frame_graph);

        let ssao_bind_group = frame_graph
            .create_bind_group_handle_builder(
                Some("ssao_ssao_bind_group".into()),
                &pipelines.ssao_bind_group_layout,
            )
            .add_handle(0, &preprocessed_depth_texture_handle)
            .add_helper(1, &prepass_textures.normal.as_ref().unwrap().texture)
            .add_handle(2, &hilbert_index_lut_handle)
            .add_helper(3, &ssao_resources.ssao_noisy_texture)
            .add_helper(4, &ssao_resources.depth_differences_texture)
            .add_handle(5, &globals_uniforms_handle)
            .add_handle(6, &thickness_buffer_handle)
            .build();

        let spatial_denoise_bind_group = frame_graph
            .create_bind_group_handle_builder(
                Some("ssao_spatial_denoise_bind_group".into()),
                &pipelines.spatial_denoise_bind_group_layout,
            )
            .add_helper(0, &ssao_resources.ssao_noisy_texture)
            .add_helper(1, &ssao_resources.depth_differences_texture)
            .add_helper(2, &ssao_resources.screen_space_ambient_occlusion_texture)
            .build();

        commands.entity(entity).insert(SsaoBindGroups {
            common_bind_group,
            preprocess_depth_bind_group,
            ssao_bind_group,
            spatial_denoise_bind_group,
        });
    }
}

fn generate_hilbert_index_lut() -> [[u16; 64]; 64] {
    use core::array::from_fn;
    from_fn(|x| from_fn(|y| hilbert_index(x as u16, y as u16)))
}

// https://www.shadertoy.com/view/3tB3z3
const HILBERT_WIDTH: u16 = 64;
fn hilbert_index(mut x: u16, mut y: u16) -> u16 {
    let mut index = 0;

    let mut level: u16 = HILBERT_WIDTH / 2;
    while level > 0 {
        let region_x = (x & level > 0) as u16;
        let region_y = (y & level > 0) as u16;
        index += level * level * ((3 * region_x) ^ region_y);

        if region_y == 0 {
            if region_x == 1 {
                x = HILBERT_WIDTH - 1 - x;
                y = HILBERT_WIDTH - 1 - y;
            }

            mem::swap(&mut x, &mut y);
        }

        level /= 2;
    }

    index
}
