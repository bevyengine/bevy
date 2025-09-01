use crate::NodePbr;
use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, Handle};
use bevy_camera::{Camera, Camera3d};
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
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
use bevy_image::ToExtents;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::{ExtractedCamera, TemporalJitter},
    diagnostic::RecordDiagnostics,
    extract_component::ExtractComponent,
    globals::{GlobalsBuffer, GlobalsUniform},
    render_graph::{NodeRunError, RenderGraphContext, RenderGraphExt, ViewNode, ViewNodeRunner},
    render_resource::{
        binding_types::{
            sampler, texture_2d, texture_depth_2d, texture_storage_2d, uniform_buffer,
        },
        *,
    },
    renderer::{RenderAdapter, RenderContext, RenderDevice, RenderQueue},
    sync_component::SyncComponentPlugin,
    sync_world::RenderEntity,
    texture::{CachedTexture, TextureCache},
    view::{Msaa, ViewUniform, ViewUniformOffset, ViewUniforms},
    Extract, ExtractSchedule, Render, RenderApp, RenderSystems,
};
use bevy_shader::{load_shader_library, Shader, ShaderDefVal};
use bevy_utils::prelude::default;
use core::mem;
use tracing::{error, warn};

/// Plugin for screen space ambient occlusion.
pub struct ScreenSpaceAmbientOcclusionPlugin;

impl Plugin for ScreenSpaceAmbientOcclusionPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "ssao_utils.wgsl");

        embedded_asset!(app, "preprocess_depth.wgsl");
        embedded_asset!(app, "ssao.wgsl");
        embedded_asset!(app, "spatial_denoise.wgsl");

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
                    prepare_ssao_pipelines.in_set(RenderSystems::Prepare),
                    prepare_ssao_textures.in_set(RenderSystems::PrepareResources),
                    prepare_ssao_bind_groups.in_set(RenderSystems::PrepareBindGroups),
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
        render_context: &mut RenderContext,
        (camera, pipeline_id, bind_groups, view_uniform_offset): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipelines = world.resource::<SsaoPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let (
            Some(camera_size),
            Some(preprocess_depth_pipeline),
            Some(spatial_denoise_pipeline),
            Some(ssao_pipeline),
        ) = (
            camera.physical_viewport_size,
            pipeline_cache.get_compute_pipeline(pipelines.preprocess_depth_pipeline),
            pipeline_cache.get_compute_pipeline(pipelines.spatial_denoise_pipeline),
            pipeline_cache.get_compute_pipeline(pipeline_id.0),
        )
        else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let command_encoder = render_context.command_encoder();
        command_encoder.push_debug_group("ssao");
        let time_span = diagnostics.time_span(command_encoder, "ssao");

        {
            let mut preprocess_depth_pass =
                command_encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("ssao_preprocess_depth"),
                    timestamp_writes: None,
                });
            preprocess_depth_pass.set_pipeline(preprocess_depth_pipeline);
            preprocess_depth_pass.set_bind_group(0, &bind_groups.preprocess_depth_bind_group, &[]);
            preprocess_depth_pass.set_bind_group(
                1,
                &bind_groups.common_bind_group,
                &[view_uniform_offset.offset],
            );
            preprocess_depth_pass.dispatch_workgroups(
                camera_size.x.div_ceil(16),
                camera_size.y.div_ceil(16),
                1,
            );
        }

        {
            let mut ssao_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("ssao"),
                timestamp_writes: None,
            });
            ssao_pass.set_pipeline(ssao_pipeline);
            ssao_pass.set_bind_group(0, &bind_groups.ssao_bind_group, &[]);
            ssao_pass.set_bind_group(
                1,
                &bind_groups.common_bind_group,
                &[view_uniform_offset.offset],
            );
            ssao_pass.dispatch_workgroups(camera_size.x.div_ceil(8), camera_size.y.div_ceil(8), 1);
        }

        {
            let mut spatial_denoise_pass =
                command_encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("ssao_spatial_denoise"),
                    timestamp_writes: None,
                });
            spatial_denoise_pass.set_pipeline(spatial_denoise_pipeline);
            spatial_denoise_pass.set_bind_group(0, &bind_groups.spatial_denoise_bind_group, &[]);
            spatial_denoise_pass.set_bind_group(
                1,
                &bind_groups.common_bind_group,
                &[view_uniform_offset.offset],
            );
            spatial_denoise_pass.dispatch_workgroups(
                camera_size.x.div_ceil(8),
                camera_size.y.div_ceil(8),
                1,
            );
        }

        time_span.end(command_encoder);
        command_encoder.pop_debug_group();
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

    hilbert_index_lut: TextureView,
    point_clamp_sampler: Sampler,
    linear_clamp_sampler: Sampler,

    shader: Handle<Shader>,
}

impl FromWorld for SsaoPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let hilbert_index_lut = render_device
            .create_texture_with_data(
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
            )
            .create_view(&TextureViewDescriptor::default());

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
                shader: load_embedded_asset!(world, "preprocess_depth.wgsl"),
                ..default()
            });

        let spatial_denoise_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("ssao_spatial_denoise_pipeline".into()),
                layout: vec![
                    spatial_denoise_bind_group_layout.clone(),
                    common_bind_group_layout.clone(),
                ],
                shader: load_embedded_asset!(world, "spatial_denoise.wgsl"),
                ..default()
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

            shader: load_embedded_asset!(world, "ssao.wgsl"),
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
            shader: self.shader.clone(),
            shader_defs,
            ..default()
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
    preprocessed_depth_texture: CachedTexture,
    ssao_noisy_texture: CachedTexture, // Pre-spatially denoised texture
    pub screen_space_ambient_occlusion_texture: CachedTexture, // Spatially denoised texture
    depth_differences_texture: CachedTexture,
    thickness_buffer: Buffer,
}

fn prepare_ssao_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera, &ScreenSpaceAmbientOcclusion)>,
) {
    for (entity, camera, ssao_settings) in &views {
        let Some(physical_viewport_size) = camera.physical_viewport_size else {
            continue;
        };
        let size = physical_viewport_size.to_extents();

        let preprocessed_depth_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("ssao_preprocessed_depth_texture"),
                size,
                mip_level_count: 5,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let ssao_noisy_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("ssao_noisy_texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let ssao_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("ssao_texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let depth_differences_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("ssao_depth_differences_texture"),
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Uint,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

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
    common_bind_group: BindGroup,
    preprocess_depth_bind_group: BindGroup,
    ssao_bind_group: BindGroup,
    spatial_denoise_bind_group: BindGroup,
}

fn prepare_ssao_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Res<SsaoPipelines>,
    view_uniforms: Res<ViewUniforms>,
    global_uniforms: Res<GlobalsBuffer>,
    views: Query<(
        Entity,
        &ScreenSpaceAmbientOcclusionResources,
        &ViewPrepassTextures,
    )>,
) {
    let (Some(view_uniforms), Some(globals_uniforms)) = (
        view_uniforms.uniforms.binding(),
        global_uniforms.buffer.binding(),
    ) else {
        return;
    };

    for (entity, ssao_resources, prepass_textures) in &views {
        let common_bind_group = render_device.create_bind_group(
            "ssao_common_bind_group",
            &pipelines.common_bind_group_layout,
            &BindGroupEntries::sequential((
                &pipelines.point_clamp_sampler,
                &pipelines.linear_clamp_sampler,
                view_uniforms.clone(),
            )),
        );

        let create_depth_view = |mip_level| {
            ssao_resources
                .preprocessed_depth_texture
                .texture
                .create_view(&TextureViewDescriptor {
                    label: Some("ssao_preprocessed_depth_texture_mip_view"),
                    base_mip_level: mip_level,
                    format: Some(TextureFormat::R16Float),
                    dimension: Some(TextureViewDimension::D2),
                    mip_level_count: Some(1),
                    ..default()
                })
        };

        let preprocess_depth_bind_group = render_device.create_bind_group(
            "ssao_preprocess_depth_bind_group",
            &pipelines.preprocess_depth_bind_group_layout,
            &BindGroupEntries::sequential((
                prepass_textures.depth_view().unwrap(),
                &create_depth_view(0),
                &create_depth_view(1),
                &create_depth_view(2),
                &create_depth_view(3),
                &create_depth_view(4),
            )),
        );

        let ssao_bind_group = render_device.create_bind_group(
            "ssao_ssao_bind_group",
            &pipelines.ssao_bind_group_layout,
            &BindGroupEntries::sequential((
                &ssao_resources.preprocessed_depth_texture.default_view,
                prepass_textures.normal_view().unwrap(),
                &pipelines.hilbert_index_lut,
                &ssao_resources.ssao_noisy_texture.default_view,
                &ssao_resources.depth_differences_texture.default_view,
                globals_uniforms.clone(),
                ssao_resources.thickness_buffer.as_entire_binding(),
            )),
        );

        let spatial_denoise_bind_group = render_device.create_bind_group(
            "ssao_spatial_denoise_bind_group",
            &pipelines.spatial_denoise_bind_group_layout,
            &BindGroupEntries::sequential((
                &ssao_resources.ssao_noisy_texture.default_view,
                &ssao_resources.depth_differences_texture.default_view,
                &ssao_resources
                    .screen_space_ambient_occlusion_texture
                    .default_view,
            )),
        );

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
