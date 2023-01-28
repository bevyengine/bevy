use crate::{MAX_CASCADES_PER_LIGHT, MAX_DIRECTIONAL_LIGHTS};
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_core_pipeline::{
    prelude::Camera3d,
    prepass::{DepthPrepass, NormalPrepass, ViewPrepassTextures},
};
use bevy_ecs::{
    prelude::{Bundle, Component, Entity},
    query::{QueryState, With},
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    camera::{ExtractedCamera, TemporalJitter},
    globals::{GlobalsBuffer, GlobalsUniform},
    prelude::Camera,
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
    render_resource::{
        AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
        BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
        BufferBindingType, CachedComputePipelineId, ComputePassDescriptor,
        ComputePipelineDescriptor, Extent3d, FilterMode, PipelineCache, Sampler,
        SamplerBindingType, SamplerDescriptor, Shader, ShaderDefVal, ShaderStages, ShaderType,
        SpecializedComputePipeline, SpecializedComputePipelines, StorageTextureAccess,
        TextureDescriptor, TextureDimension, TextureFormat, TextureSampleType, TextureUsages,
        TextureView, TextureViewDescriptor, TextureViewDimension,
    },
    renderer::{RenderAdapter, RenderContext, RenderDevice, RenderQueue},
    texture::{CachedTexture, TextureCache},
    view::{ViewUniform, ViewUniformOffset, ViewUniforms},
    Extract, RenderApp, RenderStage,
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::{prelude::default, tracing::warn};
use std::{mem, num::NonZeroU32};

pub mod draw_3d_graph {
    pub mod node {
        /// Label for the screen space ambient occlusion render node.
        pub const SCREEN_SPACE_AMBIENT_OCCLUSION: &str = "screen_space_ambient_occlusion";
    }
}

const PREPROCESS_DEPTH_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 102258915420479);
const GTAO_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 253938746510568);
const SPATIAL_DENOISE_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 466162052558226);
const GTAO_UTILS_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 366465052568786);

// TODO: Support MSAA

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

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return };

        if !render_app
            .world
            .resource::<RenderAdapter>()
            .get_texture_format_features(TextureFormat::R16Float)
            .allowed_usages
            .contains(TextureUsages::STORAGE_BINDING)
        {
            warn!(
                "ScreenSpaceAmbientOcclusionPlugin not loaded. TextureFormat::R16Float does not support TextureUsages::STORAGE_BINDING."
            );
            return;
        }

        render_app
            .init_resource::<SSAOPipelines>()
            .init_resource::<SpecializedComputePipelines<SSAOPipelines>>()
            .add_system_to_stage(RenderStage::Extract, extract_ssao_settings)
            .add_system_to_stage(RenderStage::Prepare, prepare_ssao_textures)
            .add_system_to_stage(RenderStage::Prepare, prepare_ssao_pipelines)
            .add_system_to_stage(RenderStage::Queue, queue_ssao_bind_groups);

        let ssao_node = SSAONode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        let draw_3d_graph = graph
            .get_sub_graph_mut(bevy_core_pipeline::core_3d::graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(
            draw_3d_graph::node::SCREEN_SPACE_AMBIENT_OCCLUSION,
            ssao_node,
        );
        draw_3d_graph.add_slot_edge(
            draw_3d_graph.input_node().id,
            bevy_core_pipeline::core_3d::graph::input::VIEW_ENTITY,
            draw_3d_graph::node::SCREEN_SPACE_AMBIENT_OCCLUSION,
            SSAONode::IN_VIEW,
        );
        // PREPASS -> SCREEN_SPACE_AMBIENT_OCCLUSION -> MAIN_PASS
        draw_3d_graph.add_node_edge(
            bevy_core_pipeline::core_3d::graph::node::PREPASS,
            draw_3d_graph::node::SCREEN_SPACE_AMBIENT_OCCLUSION,
        );
        draw_3d_graph.add_node_edge(
            draw_3d_graph::node::SCREEN_SPACE_AMBIENT_OCCLUSION,
            bevy_core_pipeline::core_3d::graph::node::MAIN_PASS,
        );
    }
}

/// Bundle to apply screen space ambient occlusion.
#[derive(Bundle, Default)]
pub struct ScreenSpaceAmbientOcclusionBundle {
    pub settings: ScreenSpaceAmbientOcclusionSettings,
    pub depth_prepass: DepthPrepass,
    pub normal_prepass: NormalPrepass,
}

/// Component to apply screen space ambient occlusion to a 3d camera.
///
/// Screen space ambient occlusion (SSAO) approximates small-scale,
/// local occlusion of _indirect_ diffuse light between objects. SSAO does
/// not apply to direct lighting, such as point or directional lights.
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
/// TAA ([`bevy_core_pipeline::taa::TemporalAntialiasSettings`]).
/// Doing so greatly reduces SSAO noise.
#[derive(Component, Reflect, PartialEq, Eq, Hash, Clone)]
pub enum ScreenSpaceAmbientOcclusionSettings {
    Low,
    Medium,
    High,
    Ultra,
    Custom {
        /// Higher slice count means less noise, but worse performance.
        slice_count: u32,
        /// Samples per slice side is also tweakable, but recommended to be left at 2 or 3.
        samples_per_slice_side: u32,
    },
}

impl Default for ScreenSpaceAmbientOcclusionSettings {
    fn default() -> Self {
        Self::High
    }
}

impl ScreenSpaceAmbientOcclusionSettings {
    fn sample_counts(&self) -> (u32, u32) {
        match self {
            ScreenSpaceAmbientOcclusionSettings::Low => (1, 2), // 4 spp (1 * (2 * 2)), plus optional temporal samples
            ScreenSpaceAmbientOcclusionSettings::Medium => (2, 2), // 8 spp (2 * (2 * 2)), plus optional temporal samples
            ScreenSpaceAmbientOcclusionSettings::High => (3, 3), // 18 spp (3 * (3 * 2)), plus optional temporal samples
            ScreenSpaceAmbientOcclusionSettings::Ultra => (9, 3), // 54 spp (9 * (3 * 2)), plus optional temporal samples
            ScreenSpaceAmbientOcclusionSettings::Custom {
                slice_count: slices,
                samples_per_slice_side,
            } => (*slices, *samples_per_slice_side),
        }
    }
}

struct SSAONode {
    view_query: QueryState<(
        &'static ExtractedCamera,
        &'static SSAOPipelineId,
        &'static SSAOBindGroups,
        &'static ViewUniformOffset,
    )>,
}

impl SSAONode {
    const IN_VIEW: &'static str = "view";

    fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for SSAONode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        #[cfg(feature = "trace")]
        let _ssao_span = info_span!("screen_space_ambient_occlusion").entered();

        let pipelines = world.resource::<SSAOPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (
            Ok((camera, pipeline_id, bind_groups, view_uniform_offset)),
            Some(preprocess_depth_pipeline),
            Some(spatial_denoise_pipeline),
        ) = (
            self.view_query.get_manual(world, view_entity),
            pipeline_cache.get_compute_pipeline(pipelines.preprocess_depth_pipeline),
            pipeline_cache.get_compute_pipeline(pipelines.spatial_denoise_pipeline),
        ) else {
            return Ok(());
        };
        let Some(gtao_pipeline) = pipeline_cache.get_compute_pipeline(pipeline_id.0) else { return Ok(()) };
        let Some(camera_size) = camera.physical_viewport_size else { return Ok(()) };

        {
            let mut preprocess_depth_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("ssao_preprocess_depth_pass"),
                    });
            preprocess_depth_pass.set_pipeline(preprocess_depth_pipeline);
            preprocess_depth_pass.set_bind_group(0, &bind_groups.preprocess_depth_bind_group, &[]);
            preprocess_depth_pass.set_bind_group(
                1,
                &bind_groups.common_bind_group,
                &[view_uniform_offset.offset],
            );
            preprocess_depth_pass.dispatch_workgroups(
                (camera_size.x + 15) / 16,
                (camera_size.y + 15) / 16,
                1,
            );
        }

        {
            let mut gtao_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("ssao_gtao_pass"),
                    });
            gtao_pass.set_pipeline(gtao_pipeline);
            gtao_pass.set_bind_group(0, &bind_groups.gtao_bind_group, &[]);
            gtao_pass.set_bind_group(
                1,
                &bind_groups.common_bind_group,
                &[view_uniform_offset.offset],
            );
            gtao_pass.dispatch_workgroups((camera_size.x + 7) / 8, (camera_size.y + 7) / 8, 1);
        }

        {
            let mut spatial_denoise_pass =
                render_context
                    .command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("ssao_spatial_denoise_pass"),
                    });
            spatial_denoise_pass.set_pipeline(spatial_denoise_pipeline);
            spatial_denoise_pass.set_bind_group(0, &bind_groups.spatial_denoise_bind_group, &[]);
            spatial_denoise_pass.set_bind_group(
                1,
                &bind_groups.common_bind_group,
                &[view_uniform_offset.offset],
            );
            spatial_denoise_pass.dispatch_workgroups(
                (camera_size.x + 7) / 8,
                (camera_size.y + 7) / 8,
                1,
            );
        }

        Ok(())
    }
}

#[derive(Resource)]
struct SSAOPipelines {
    preprocess_depth_pipeline: CachedComputePipelineId,
    spatial_denoise_pipeline: CachedComputePipelineId,

    common_bind_group_layout: BindGroupLayout,
    preprocess_depth_bind_group_layout: BindGroupLayout,
    gtao_bind_group_layout: BindGroupLayout,
    spatial_denoise_bind_group_layout: BindGroupLayout,

    hilbert_index_texture: TextureView,
    point_clamp_sampler: Sampler,
}

impl FromWorld for SSAOPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let render_queue = world.resource::<RenderQueue>();
        let pipeline_cache = world.resource::<PipelineCache>();

        let hilbert_index_texture = render_device
            .create_texture_with_data(
                render_queue,
                &(TextureDescriptor {
                    label: Some("ssao_hilbert_index_texture"),
                    size: Extent3d {
                        width: 64,
                        height: 64,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::R16Uint,
                    usage: TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                }),
                bytemuck::cast_slice(&generate_hilbert_index_texture()),
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

        let common_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("ssao_common_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(ViewUniform::min_size()),
                        },
                        count: None,
                    },
                ],
            });

        let mip_texture_entry = BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::R16Float,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        };
        let preprocess_depth_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("ssao_preprocess_depth_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    mip_texture_entry,
                    BindGroupLayoutEntry {
                        binding: 2,
                        ..mip_texture_entry
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        ..mip_texture_entry
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        ..mip_texture_entry
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        ..mip_texture_entry
                    },
                ],
            });

        let gtao_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("ssao_gtao_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Uint,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::R16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::R32Uint,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: Some(GlobalsUniform::min_size()),
                        },
                        count: None,
                    },
                ],
            });

        let spatial_denoise_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("ssao_spatial_denoise_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Uint,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: TextureFormat::R16Float,
                            view_dimension: TextureViewDimension::D2,
                        },
                        count: None,
                    },
                ],
            });

        let preprocess_depth_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("ssao_preprocess_depth_pipeline".into()),
                layout: Some(vec![
                    preprocess_depth_bind_group_layout.clone(),
                    common_bind_group_layout.clone(),
                ]),
                shader: PREPROCESS_DEPTH_SHADER_HANDLE.typed(),
                shader_defs: vec![
                    // TODO: Remove this hack
                    ShaderDefVal::Int(
                        "MAX_DIRECTIONAL_LIGHTS".to_string(),
                        MAX_DIRECTIONAL_LIGHTS as i32,
                    ),
                    ShaderDefVal::UInt(
                        "MAX_CASCADES_PER_LIGHT".to_string(),
                        MAX_CASCADES_PER_LIGHT as u32,
                    ),
                ],
                entry_point: "preprocess_depth".into(),
            });

        let spatial_denoise_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("ssao_spatial_denoise_pipeline".into()),
                layout: Some(vec![
                    spatial_denoise_bind_group_layout.clone(),
                    common_bind_group_layout.clone(),
                ]),
                shader: SPATIAL_DENOISE_SHADER_HANDLE.typed(),
                shader_defs: vec![
                    // TODO: Remove this hack
                    ShaderDefVal::Int(
                        "MAX_DIRECTIONAL_LIGHTS".to_string(),
                        MAX_DIRECTIONAL_LIGHTS as i32,
                    ),
                    ShaderDefVal::UInt(
                        "MAX_CASCADES_PER_LIGHT".to_string(),
                        MAX_CASCADES_PER_LIGHT as u32,
                    ),
                ],
                entry_point: "spatial_denoise".into(),
            });

        Self {
            preprocess_depth_pipeline,
            spatial_denoise_pipeline,

            common_bind_group_layout,
            preprocess_depth_bind_group_layout,
            gtao_bind_group_layout,
            spatial_denoise_bind_group_layout,

            hilbert_index_texture,
            point_clamp_sampler,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct AmbientOcclusionPipelineKey {
    ssao_quality: ScreenSpaceAmbientOcclusionSettings,
    temporal_noise: bool,
}

impl SpecializedComputePipeline for SSAOPipelines {
    type Key = AmbientOcclusionPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let (slice_count, samples_per_slice_side) = key.ssao_quality.sample_counts();

        let mut shader_defs = vec![
            ShaderDefVal::Int("SLICE_COUNT".to_string(), slice_count as i32),
            ShaderDefVal::Int(
                "SAMPLES_PER_SLICE_SIDE".to_string(),
                samples_per_slice_side as i32,
            ),
            // TODO: Remove this hack
            ShaderDefVal::Int(
                "MAX_DIRECTIONAL_LIGHTS".to_string(),
                MAX_DIRECTIONAL_LIGHTS as i32,
            ),
            ShaderDefVal::UInt(
                "MAX_CASCADES_PER_LIGHT".to_string(),
                MAX_CASCADES_PER_LIGHT as u32,
            ),
        ];

        if key.temporal_noise {
            shader_defs.push("TEMPORAL_NOISE".into());
        }

        ComputePipelineDescriptor {
            label: Some("ssao_gtao_pipeline".into()),
            layout: Some(vec![
                self.gtao_bind_group_layout.clone(),
                self.common_bind_group_layout.clone(),
            ]),
            shader: GTAO_SHADER_HANDLE.typed(),
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
) {
    for (entity, camera, ssao_settings) in &cameras {
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
        if let Some(physical_viewport_size) = camera.physical_viewport_size {
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
}

#[derive(Component)]
struct SSAOPipelineId(CachedComputePipelineId);

fn prepare_ssao_pipelines(
    mut commands: Commands,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedComputePipelines<SSAOPipelines>>,
    pipeline: Res<SSAOPipelines>,
    views: Query<(
        Entity,
        &ScreenSpaceAmbientOcclusionSettings,
        Option<&TemporalJitter>,
    )>,
) {
    for (entity, ssao_settings, temporal_jitter) in &views {
        let pipeline_id = pipelines.specialize(
            &mut pipeline_cache,
            &pipeline,
            AmbientOcclusionPipelineKey {
                ssao_quality: ssao_settings.clone(),
                temporal_noise: temporal_jitter.is_some(),
            },
        );

        commands.entity(entity).insert(SSAOPipelineId(pipeline_id));
    }
}

#[derive(Component)]
struct SSAOBindGroups {
    common_bind_group: BindGroup,
    preprocess_depth_bind_group: BindGroup,
    gtao_bind_group: BindGroup,
    spatial_denoise_bind_group: BindGroup,
}

fn queue_ssao_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Res<SSAOPipelines>,
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
        let common_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("ssao_common_bind_group"),
            layout: &pipelines.common_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::Sampler(&pipelines.point_clamp_sampler),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: view_uniforms.clone(),
                },
            ],
        });

        let preprocess_depth_mip_view_descriptor = TextureViewDescriptor {
            format: Some(TextureFormat::R16Float),
            dimension: Some(TextureViewDimension::D2),
            mip_level_count: NonZeroU32::new(1),
            ..default()
        };
        let preprocess_depth_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("ssao_preprocess_depth_bind_group"),
            layout: &pipelines.preprocess_depth_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &prepass_textures.depth.as_ref().unwrap().default_view,
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &ssao_textures
                            .preprocessed_depth_texture
                            .texture
                            .create_view(&TextureViewDescriptor {
                                label: Some("ssao_preprocessed_depth_texture_mip_view_0"),
                                base_mip_level: 0,
                                ..preprocess_depth_mip_view_descriptor
                            }),
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &ssao_textures
                            .preprocessed_depth_texture
                            .texture
                            .create_view(&TextureViewDescriptor {
                                label: Some("ssao_preprocessed_depth_texture_mip_view_1"),
                                base_mip_level: 1,
                                ..preprocess_depth_mip_view_descriptor
                            }),
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(
                        &ssao_textures
                            .preprocessed_depth_texture
                            .texture
                            .create_view(&TextureViewDescriptor {
                                label: Some("ssao_preprocessed_depth_texture_mip_view_2"),
                                base_mip_level: 2,
                                ..preprocess_depth_mip_view_descriptor
                            }),
                    ),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(
                        &ssao_textures
                            .preprocessed_depth_texture
                            .texture
                            .create_view(&TextureViewDescriptor {
                                label: Some("ssao_preprocessed_depth_texture_mip_view_3"),
                                base_mip_level: 3,
                                ..preprocess_depth_mip_view_descriptor
                            }),
                    ),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::TextureView(
                        &ssao_textures
                            .preprocessed_depth_texture
                            .texture
                            .create_view(&TextureViewDescriptor {
                                label: Some("ssao_preprocessed_depth_texture_mip_view_4"),
                                base_mip_level: 4,
                                ..preprocess_depth_mip_view_descriptor
                            }),
                    ),
                },
            ],
        });

        let gtao_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("ssao_gtao_bind_group"),
            layout: &pipelines.gtao_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &ssao_textures.preprocessed_depth_texture.default_view,
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &prepass_textures.normal.as_ref().unwrap().default_view,
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(&pipelines.hilbert_index_texture),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::TextureView(
                        &ssao_textures.ssao_noisy_texture.default_view,
                    ),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(
                        &ssao_textures.depth_differences_texture.default_view,
                    ),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: globals_uniforms.clone(),
                },
            ],
        });

        let spatial_denoise_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("ssao_spatial_denoise_bind_group"),
            layout: &pipelines.spatial_denoise_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(
                        &ssao_textures.ssao_noisy_texture.default_view,
                    ),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::TextureView(
                        &ssao_textures.depth_differences_texture.default_view,
                    ),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &ssao_textures
                            .screen_space_ambient_occlusion_texture
                            .default_view,
                    ),
                },
            ],
        });

        commands.entity(entity).insert(SSAOBindGroups {
            common_bind_group,
            preprocess_depth_bind_group,
            gtao_bind_group,
            spatial_denoise_bind_group,
        });
    }
}

#[allow(clippy::needless_range_loop)]
fn generate_hilbert_index_texture() -> [[u16; 64]; 64] {
    let mut t = [[0; 64]; 64];

    for x in 0..64 {
        for y in 0..64 {
            t[x][y] = hilbert_index(x as u16, y as u16);
        }
    }

    t
}

// https://www.shadertoy.com/view/3tB3z3
const HILBERT_WIDTH: u16 = 1 << 6;
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
