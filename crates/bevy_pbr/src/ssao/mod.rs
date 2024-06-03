use crate::NodePbr;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_core_pipeline::{
    core_3d::graph::{Core3d, Node3d},
    prelude::Camera3d,
    prepass::{DepthPrepass, NormalPrepass, ViewPrepassTextures},
};
use bevy_ecs::{
    prelude::{Bundle, Component, Entity},
    query::{Has, QueryItem, With},
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs,
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_reflect::Reflect;
use bevy_render::{
    camera::{ExtractedCamera, TemporalJitter},
    extract_component::ExtractComponent,
    globals::{GlobalsBuffer, GlobalsUniform},
    prelude::Camera,
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::{
        binding_types::{
            sampler, texture_2d, texture_depth_2d, texture_storage_2d, uniform_buffer,
        },
        *,
    },
    renderer::{RenderAdapter, RenderContext, RenderDevice, RenderQueue},
    texture::{CachedTexture, TextureCache},
    view::{Msaa, ViewUniform, ViewUniformOffset, ViewUniforms},
    Extract, ExtractSchedule, Render, RenderApp, RenderSet,
};
use bevy_utils::{
    prelude::default,
    tracing::{error, warn},
};
use std::mem;

const PREPROCESS_DEPTH_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(102258915420479);
const GTAO_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(253938746510568);
const SPATIAL_DENOISE_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(466162052558226);
const GTAO_UTILS_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(366465052568786);

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
        load_internal_asset!(app, GTAO_SHADER_HANDLE, "gtao.wgsl", Shader::from_wgsl);
        load_internal_asset!(
            app,
            SPATIAL_DENOISE_SHADER_HANDLE,
            "spatial_denoise.wgsl",
            Shader::from_wgsl
        );
        load_internal_asset!(
            app,
            GTAO_UTILS_SHADER_HANDLE,
            "gtao_utils.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<ScreenSpaceAmbientOcclusionSettings>();
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

/// Bundle to apply screen space ambient occlusion.
#[derive(Bundle, Default, Clone)]
pub struct ScreenSpaceAmbientOcclusionBundle {
    pub settings: ScreenSpaceAmbientOcclusionSettings,
    pub depth_prepass: DepthPrepass,
    pub normal_prepass: NormalPrepass,
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
/// Requires that you add [`ScreenSpaceAmbientOcclusionPlugin`] to your app,
/// and add the [`DepthPrepass`] and [`NormalPrepass`] components to your camera.
///
/// It strongly recommended that you use SSAO in conjunction with
/// TAA ([`bevy_core_pipeline::experimental::taa::TemporalAntiAliasSettings`]).
/// Doing so greatly reduces SSAO noise.
///
/// SSAO is not supported on `WebGL2`, and is not currently supported on `WebGPU` or `DirectX12`.
#[derive(Component, ExtractComponent, Reflect, PartialEq, Eq, Hash, Clone, Default, Debug)]
#[reflect(Component)]
pub struct ScreenSpaceAmbientOcclusionSettings {
    pub quality_level: ScreenSpaceAmbientOcclusionQualityLevel,
}

#[derive(Reflect, PartialEq, Eq, Hash, Clone, Copy, Default, Debug)]
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
            Some(gtao_pipeline),
        ) = (
            camera.physical_viewport_size,
            pipeline_cache.get_compute_pipeline(pipelines.preprocess_depth_pipeline),
            pipeline_cache.get_compute_pipeline(pipelines.spatial_denoise_pipeline),
            pipeline_cache.get_compute_pipeline(pipeline_id.0),
        )
        else {
            return Ok(());
        };

        render_context.command_encoder().push_debug_group("ssao");

        {
            let mut preprocess_depth_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("ssao_preprocess_depth_pass"),
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
                div_ceil(camera_size.x, 16),
                div_ceil(camera_size.y, 16),
                1,
            );
        }

        {
            let mut gtao_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("ssao_gtao_pass"),
                        timestamp_writes: None,
                    });
            gtao_pass.set_pipeline(gtao_pipeline);
            gtao_pass.set_bind_group(0, &bind_groups.gtao_bind_group, &[]);
            gtao_pass.set_bind_group(
                1,
                &bind_groups.common_bind_group,
                &[view_uniform_offset.offset],
            );
            gtao_pass.dispatch_workgroups(
                div_ceil(camera_size.x, 8),
                div_ceil(camera_size.y, 8),
                1,
            );
        }

        {
            let mut spatial_denoise_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("ssao_spatial_denoise_pass"),
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
                div_ceil(camera_size.x, 8),
                div_ceil(camera_size.y, 8),
                1,
            );
        }

        render_context.command_encoder().pop_debug_group();
        Ok(())
    }
}

#[derive(Resource)]
struct SsaoPipelines {
    preprocess_depth_pipeline: CachedComputePipelineId,
    spatial_denoise_pipeline: CachedComputePipelineId,

    common_bind_group_layout: BindGroupLayout,
    preprocess_depth_bind_group_layout: BindGroupLayout,
    gtao_bind_group_layout: BindGroupLayout,
    spatial_denoise_bind_group_layout: BindGroupLayout,

    hilbert_index_lut: TextureView,
    point_clamp_sampler: Sampler,
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

        let common_bind_group_layout = render_device.create_bind_group_layout(
            "ssao_common_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    sampler(SamplerBindingType::NonFiltering),
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

        let gtao_bind_group_layout = render_device.create_bind_group_layout(
            "ssao_gtao_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_2d(TextureSampleType::Float { filterable: false }),
                    texture_2d(TextureSampleType::Uint),
                    texture_storage_2d(TextureFormat::R16Float, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::R32Uint, StorageTextureAccess::WriteOnly),
                    uniform_buffer::<GlobalsUniform>(false),
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
            });

        Self {
            preprocess_depth_pipeline,
            spatial_denoise_pipeline,

            common_bind_group_layout,
            preprocess_depth_bind_group_layout,
            gtao_bind_group_layout,
            spatial_denoise_bind_group_layout,

            hilbert_index_lut,
            point_clamp_sampler,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct SsaoPipelineKey {
    ssao_settings: ScreenSpaceAmbientOcclusionSettings,
    temporal_jitter: bool,
}

impl SpecializedComputePipeline for SsaoPipelines {
    type Key = SsaoPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let (slice_count, samples_per_slice_side) = key.ssao_settings.quality_level.sample_counts();

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
            label: Some("ssao_gtao_pipeline".into()),
            layout: vec![
                self.gtao_bind_group_layout.clone(),
                self.common_bind_group_layout.clone(),
            ],
            push_constant_ranges: vec![],
            shader: GTAO_SHADER_HANDLE,
            shader_defs,
            entry_point: "gtao".into(),
        }
    }
}

fn extract_ssao_settings(
    mut commands: Commands,
    cameras: Extract<
        Query<
            (Entity, &Camera, &ScreenSpaceAmbientOcclusionSettings),
            (With<Camera3d>, With<DepthPrepass>, With<NormalPrepass>),
        >,
    >,
    msaa: Extract<Res<Msaa>>,
) {
    for (entity, camera, ssao_settings) in &cameras {
        if **msaa != Msaa::Off {
            error!(
                "SSAO is being used which requires Msaa::Off, but Msaa is currently set to Msaa::{:?}",
                **msaa
            );
            return;
        }

        if camera.is_active {
            commands.get_or_spawn(entity).insert(ssao_settings.clone());
        }
    }
}

#[derive(Component)]
pub struct ScreenSpaceAmbientOcclusionTextures {
    preprocessed_depth_texture: CachedTexture,
    ssao_noisy_texture: CachedTexture, // Pre-spatially denoised texture
    pub screen_space_ambient_occlusion_texture: CachedTexture, // Spatially denoised texture
    depth_differences_texture: CachedTexture,
}

fn prepare_ssao_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera), With<ScreenSpaceAmbientOcclusionSettings>>,
) {
    for (entity, camera) in &views {
        let Some(physical_viewport_size) = camera.physical_viewport_size else {
            continue;
        };
        let size = Extent3d {
            width: physical_viewport_size.x,
            height: physical_viewport_size.y,
            depth_or_array_layers: 1,
        };

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

        commands
            .entity(entity)
            .insert(ScreenSpaceAmbientOcclusionTextures {
                preprocessed_depth_texture,
                ssao_noisy_texture,
                screen_space_ambient_occlusion_texture: ssao_texture,
                depth_differences_texture,
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
    views: Query<(
        Entity,
        &ScreenSpaceAmbientOcclusionSettings,
        Has<TemporalJitter>,
    )>,
) {
    for (entity, ssao_settings, temporal_jitter) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            SsaoPipelineKey {
                ssao_settings: ssao_settings.clone(),
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
    gtao_bind_group: BindGroup,
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
        &ScreenSpaceAmbientOcclusionTextures,
        &ViewPrepassTextures,
    )>,
) {
    let (Some(view_uniforms), Some(globals_uniforms)) = (
        view_uniforms.uniforms.binding(),
        global_uniforms.buffer.binding(),
    ) else {
        return;
    };

    for (entity, ssao_textures, prepass_textures) in &views {
        let common_bind_group = render_device.create_bind_group(
            "ssao_common_bind_group",
            &pipelines.common_bind_group_layout,
            &BindGroupEntries::sequential((&pipelines.point_clamp_sampler, view_uniforms.clone())),
        );

        let create_depth_view = |mip_level| {
            ssao_textures
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

        let gtao_bind_group = render_device.create_bind_group(
            "ssao_gtao_bind_group",
            &pipelines.gtao_bind_group_layout,
            &BindGroupEntries::sequential((
                &ssao_textures.preprocessed_depth_texture.default_view,
                prepass_textures.normal_view().unwrap(),
                &pipelines.hilbert_index_lut,
                &ssao_textures.ssao_noisy_texture.default_view,
                &ssao_textures.depth_differences_texture.default_view,
                globals_uniforms.clone(),
            )),
        );

        let spatial_denoise_bind_group = render_device.create_bind_group(
            "ssao_spatial_denoise_bind_group",
            &pipelines.spatial_denoise_bind_group_layout,
            &BindGroupEntries::sequential((
                &ssao_textures.ssao_noisy_texture.default_view,
                &ssao_textures.depth_differences_texture.default_view,
                &ssao_textures
                    .screen_space_ambient_occlusion_texture
                    .default_view,
            )),
        );

        commands.entity(entity).insert(SsaoBindGroups {
            common_bind_group,
            preprocess_depth_bind_group,
            gtao_bind_group,
            spatial_denoise_bind_group,
        });
    }
}

#[allow(clippy::needless_range_loop)]
fn generate_hilbert_index_lut() -> [[u16; 64]; 64] {
    let mut t = [[0; 64]; 64];

    for x in 0..64 {
        for y in 0..64 {
            t[x][y] = hilbert_index(x as u16, y as u16);
        }
    }

    t
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

/// Divide `numerator` by `denominator`, rounded up to the nearest multiple of `denominator`.
fn div_ceil(numerator: u32, denominator: u32) -> u32 {
    (numerator + denominator - 1) / denominator
}
