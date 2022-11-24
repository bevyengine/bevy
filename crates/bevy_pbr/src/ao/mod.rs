// Ground Truth-based Ambient Occlusion (GTAO)
// Paper: https://www.activision.com/cdn/research/Practical_Real_Time_Strategies_for_Accurate_Indirect_Occlusion_NEW%20VERSION_COLOR.pdf
// Presentation: https://blog.selfshadow.com/publications/s2016-shading-course/activision/s2016_pbs_activision_occlusion.pdf

// Source code heavily based on XeGTAO v1.30 from Intel
// https://github.com/GameTechDev/XeGTAO/blob/0d177ce06bfa642f64d8af4de1197ad1bcb862d4/Source/Rendering/Shaders/XeGTAO.hlsli

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_core_pipeline::{
    prelude::Camera3d,
    prepass::{PrepassSettings, ViewPrepassTextures},
};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::{QueryState, With},
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::UVec2;
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    camera::ExtractedCamera,
    prelude::Camera,
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
    render_resource::{
        AddressMode, BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout,
        BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingResource, BindingType,
        BufferBindingType, CachedComputePipelineId, ComputePassDescriptor,
        ComputePipelineDescriptor, DynamicUniformBuffer, Extent3d, FilterMode, PipelineCache,
        Sampler, SamplerBindingType, SamplerDescriptor, Shader, ShaderStages, ShaderType,
        StorageTextureAccess, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
        TextureSampleType, TextureUsages, TextureViewDescriptor, TextureViewDimension,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::{CachedTexture, TextureCache},
    view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
    Extract, RenderApp, RenderStage,
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::{prelude::default, HashMap};
use std::num::NonZeroU32;

pub mod draw_3d_graph {
    pub mod node {
        /// Label for the ambient occlusion render node.
        pub const AMBIENT_OCCLUSION: &str = "ambient_occlusion";
    }
}

const PREFILTER_DEPTH_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 549599446926908);

// TODO: Support MSAA

pub struct AmbientOcclusionPlugin;

impl Plugin for AmbientOcclusionPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PREFILTER_DEPTH_SHADER_HANDLE,
            "prefilter_depth.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<AmbientOcclusionSettings>();

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<AmbientOcclusionPipelines>()
            .init_resource::<AmbientOcclusionUniforms>()
            .add_system_to_stage(RenderStage::Extract, extract_ambient_occlusion_settings)
            .add_system_to_stage(RenderStage::Prepare, prepare_ambient_occlusion_textures)
            .add_system_to_stage(RenderStage::Prepare, prepare_ambient_occlusion_uniforms)
            .add_system_to_stage(RenderStage::Queue, queue_ambient_occlusion_bind_groups);

        let ao_node = AmbientOcclusionNode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        let draw_3d_graph = graph
            .get_sub_graph_mut(bevy_core_pipeline::core_3d::graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(draw_3d_graph::node::AMBIENT_OCCLUSION, ao_node);
        draw_3d_graph
            .add_slot_edge(
                draw_3d_graph.input_node().unwrap().id,
                bevy_core_pipeline::core_3d::graph::input::VIEW_ENTITY,
                draw_3d_graph::node::AMBIENT_OCCLUSION,
                AmbientOcclusionNode::IN_VIEW,
            )
            .unwrap();
        // PREPASS -> AMBIENT_OCCLUSION -> MAIN_PASS
        draw_3d_graph
            .add_node_edge(
                bevy_core_pipeline::core_3d::graph::node::PREPASS,
                draw_3d_graph::node::AMBIENT_OCCLUSION,
            )
            .unwrap();
        draw_3d_graph
            .add_node_edge(
                draw_3d_graph::node::AMBIENT_OCCLUSION,
                bevy_core_pipeline::core_3d::graph::node::MAIN_PASS,
            )
            .unwrap();
    }
}

#[derive(Component, Reflect, ShaderType, Clone)]
pub struct AmbientOcclusionSettings {
    effect_radius: f32,
    effect_falloff_range: f32,
}

impl Default for AmbientOcclusionSettings {
    fn default() -> Self {
        // TODO: Document defaults
        Self {
            effect_radius: 0.5,
            effect_falloff_range: 0.615,
        }
    }
}

struct AmbientOcclusionNode {
    view_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static AmbientOcclusionTextures,
        &'static AmbientOcclusionBindGroups,
        &'static AmbientOcclusionUniformOffset,
        &'static ViewUniformOffset,
    )>,
}

impl AmbientOcclusionNode {
    const IN_VIEW: &'static str = "view";

    fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for AmbientOcclusionNode {
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
        let _ao_span = info_span!("ambient_occlusion").entered();

        let pipelines = world.resource::<AmbientOcclusionPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (camera, view_target, textures, bind_groups, ao_uniform_offset, view_uniform_offset) =
            match self.view_query.get_manual(world, view_entity) {
                Ok(result) => result,
                _ => return Ok(()),
            };
        let camera_size = match camera.physical_viewport_size {
            Some(size) => size,
            None => return Ok(()),
        };
        let prefilter_depth_pipeline =
            match pipeline_cache.get_compute_pipeline(pipelines.prefilter_depth_pipeline) {
                Some(p1) => p1,
                _ => return Ok(()),
            };

        {
            let mut prefilter_depth_pass =
                render_context
                    .command_encoder
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("ambient_occlusion_prefilter_depth_pass"),
                    });
            prefilter_depth_pass.set_pipeline(prefilter_depth_pipeline);
            prefilter_depth_pass.set_bind_group(
                0,
                &bind_groups.prefilter_depth_bind_group,
                &[ao_uniform_offset.offset, view_uniform_offset.offset],
            );
            prefilter_depth_pass.dispatch_workgroups(
                (camera_size.x + 15) / 16,
                (camera_size.y + 15) / 16,
                1,
            );
        }

        Ok(())
    }
}

#[derive(Resource)]
struct AmbientOcclusionPipelines {
    prefilter_depth_pipeline: CachedComputePipelineId,
    prefilter_depth_bind_group_layout: BindGroupLayout,
    point_clamp_sampler: Sampler,
}

impl FromWorld for AmbientOcclusionPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let point_clamp_sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        let mip_texture_entry = BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: TextureFormat::R32Float,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        };
        let prefilter_depth_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("ambient_occlusion_prefilter_depth_bind_group_layout"),
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
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(AmbientOcclusionSettings::min_size()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 8,
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

        let mut pipeline_cache = world.resource_mut::<PipelineCache>();

        let prefilter_depth_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("ambient_occlusion_prefilter_depth_pipeline".into()),
                layout: Some(vec![prefilter_depth_bind_group_layout.clone()]),
                shader: PREFILTER_DEPTH_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "prefilter_depth".into(),
            });

        Self {
            prefilter_depth_pipeline,
            prefilter_depth_bind_group_layout,
            point_clamp_sampler,
        }
    }
}

fn extract_ambient_occlusion_settings(
    mut commands: Commands,
    cameras: Extract<
        Query<(Entity, &Camera, &AmbientOcclusionSettings, &PrepassSettings), With<Camera3d>>,
    >,
) {
    for (entity, camera, ao_settings, prepass_settings) in &cameras {
        if camera.is_active && prepass_settings.output_depth && prepass_settings.output_normals {
            commands.get_or_spawn(entity).insert(ao_settings.clone());
        }
    }
}

#[derive(Component)]
struct AmbientOcclusionTextures {
    prefiltered_depth_texture: CachedTexture,
}

fn prepare_ambient_occlusion_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera), With<AmbientOcclusionSettings>>,
) {
    let mut prefiltered_depth_textures = HashMap::default();
    for (entity, camera) in &views {
        if let Some(UVec2 {
            x: width,
            y: height,
        }) = camera.physical_viewport_size
        {
            let texture_descriptor = TextureDescriptor {
                label: Some("ambient_occlusion_prefiltered_depth_texture"),
                size: Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 5,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::STORAGE_BINDING,
            };
            let prefiltered_depth_texture = prefiltered_depth_textures
                .entry(camera.target.clone())
                .or_insert_with(|| texture_cache.get(&render_device, texture_descriptor))
                .clone();

            commands.entity(entity).insert(AmbientOcclusionTextures {
                prefiltered_depth_texture,
            });
        }
    }
}

#[derive(Resource, Default)]
struct AmbientOcclusionUniforms {
    uniforms: DynamicUniformBuffer<AmbientOcclusionSettings>,
}

#[derive(Component)]
struct AmbientOcclusionUniformOffset {
    offset: u32,
}

fn prepare_ambient_occlusion_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut ao_uniforms: ResMut<AmbientOcclusionUniforms>,
    au_query: Query<(Entity, &AmbientOcclusionSettings)>,
) {
    ao_uniforms.uniforms.clear();

    let entities = au_query
        .iter()
        .map(|(entity, settings)| {
            let offset = ao_uniforms.uniforms.push(settings.clone());
            (entity, (AmbientOcclusionUniformOffset { offset }))
        })
        .collect::<Vec<_>>();
    commands.insert_or_spawn_batch(entities);

    ao_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

#[derive(Component)]
struct AmbientOcclusionBindGroups {
    prefilter_depth_bind_group: BindGroup,
}

fn queue_ambient_occlusion_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Res<AmbientOcclusionPipelines>,
    ao_uniforms: Res<AmbientOcclusionUniforms>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<(Entity, &AmbientOcclusionTextures, &ViewPrepassTextures)>,
) {
    if let Some(ao_uniforms) = ao_uniforms.uniforms.binding() {
        if let Some(view_uniforms) = view_uniforms.uniforms.binding() {
            for (entity, ao_textures, prepass_textures) in &views {
                let prefilter_depth_mip_view_descriptor = TextureViewDescriptor {
                    format: Some(TextureFormat::R32Float),
                    dimension: Some(TextureViewDimension::D2),
                    mip_level_count: NonZeroU32::new(1),
                    ..default()
                };
                let prefilter_depth_bind_group = render_device
                    .create_bind_group(&BindGroupDescriptor {
                    label: Some("ambient_occlusion_prefilter_depth_bind_group"),
                    layout: &pipelines.prefilter_depth_bind_group_layout,
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
                                &ao_textures
                                    .prefiltered_depth_texture
                                    .texture
                                    .create_view(&TextureViewDescriptor {
                                    label: Some(
                                        "ambient_occlusion_prefiltered_depth_texture_mip_view_0",
                                    ),
                                    base_mip_level: 0,
                                    ..prefilter_depth_mip_view_descriptor
                                }),
                            ),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: BindingResource::TextureView(
                                &ao_textures
                                    .prefiltered_depth_texture
                                    .texture
                                    .create_view(&TextureViewDescriptor {
                                    label: Some(
                                        "ambient_occlusion_prefiltered_depth_texture_mip_view_1",
                                    ),
                                    base_mip_level: 1,
                                    ..prefilter_depth_mip_view_descriptor
                                }),
                            ),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: BindingResource::TextureView(
                                &ao_textures
                                    .prefiltered_depth_texture
                                    .texture
                                    .create_view(&TextureViewDescriptor {
                                    label: Some(
                                        "ambient_occlusion_prefiltered_depth_texture_mip_view_2",
                                    ),
                                    base_mip_level: 2,
                                    ..prefilter_depth_mip_view_descriptor
                                }),
                            ),
                        },
                        BindGroupEntry {
                            binding: 4,
                            resource: BindingResource::TextureView(
                                &ao_textures
                                    .prefiltered_depth_texture
                                    .texture
                                    .create_view(&TextureViewDescriptor {
                                    label: Some(
                                        "ambient_occlusion_prefiltered_depth_texture_mip_view_3",
                                    ),
                                    base_mip_level: 3,
                                    ..prefilter_depth_mip_view_descriptor
                                }),
                            ),
                        },
                        BindGroupEntry {
                            binding: 5,
                            resource: BindingResource::TextureView(
                                &ao_textures
                                    .prefiltered_depth_texture
                                    .texture
                                    .create_view(&TextureViewDescriptor {
                                    label: Some(
                                        "ambient_occlusion_prefiltered_depth_texture_mip_view_4",
                                    ),
                                    base_mip_level: 4,
                                    ..prefilter_depth_mip_view_descriptor
                                }),
                            ),
                        },
                        BindGroupEntry {
                            binding: 6,
                            resource: BindingResource::Sampler(&pipelines.point_clamp_sampler),
                        },
                        BindGroupEntry {
                            binding: 7,
                            resource: ao_uniforms.clone(),
                        },
                        BindGroupEntry {
                            binding: 8,
                            resource: view_uniforms.clone(),
                        },
                    ],
                });

                commands.entity(entity).insert(AmbientOcclusionBindGroups {
                    prefilter_depth_bind_group,
                });
            }
        }
    }
}
