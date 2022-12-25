use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_core::FrameCount;
use bevy_core_pipeline::{
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::Camera3d,
    prepass::{DepthPrepass, VelocityPrepass, ViewPrepassTextures},
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
    prelude::Camera,
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
    render_phase::TrackedRenderPass,
    render_resource::{
        BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntry, BindingResource, BindingType, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, Extent3d, FilterMode, FragmentState, MultisampleState,
        Operations, PipelineCache, PrimitiveState, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, Shader,
        ShaderStages, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureDescriptor,
        TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureViewDimension,
    },
    renderer::{RenderContext, RenderDevice},
    texture::{BevyDefault, CachedTexture, TextureCache},
    view::{ExtractedView, Msaa, ViewTarget},
    Extract, RenderApp, RenderStage,
};

mod draw_3d_graph {
    pub mod node {
        /// Label for the TAA render node.
        pub const TAA: &str = "taa";
    }
}

const TAA_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 656865235226276);

pub struct TemporalAntialiasPlugin;

impl Plugin for TemporalAntialiasPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, TAA_SHADER_HANDLE, "taa.wgsl", Shader::from_wgsl);

        app.insert_resource(Msaa { samples: 1 })
            .register_type::<TemporalAntialiasSettings>();

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else { return };

        render_app
            .init_resource::<TAAPipeline>()
            .init_resource::<SpecializedRenderPipelines<TAAPipeline>>()
            .add_system_to_stage(RenderStage::Extract, extract_taa_settings)
            .add_system_to_stage(RenderStage::Prepare, prepare_taa_history_textures)
            .add_system_to_stage(RenderStage::Prepare, prepare_taa_pipelines);

        let taa_node = TAANode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        let draw_3d_graph = graph
            .get_sub_graph_mut(bevy_core_pipeline::core_3d::graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(draw_3d_graph::node::TAA, taa_node);
        draw_3d_graph.add_slot_edge(
            draw_3d_graph.input_node().id,
            bevy_core_pipeline::core_3d::graph::input::VIEW_ENTITY,
            draw_3d_graph::node::TAA,
            TAANode::IN_VIEW,
        );
        // MAIN_PASS -> TAA -> BLOOM -> TONEMAPPING
        draw_3d_graph.add_node_edge(
            bevy_core_pipeline::core_3d::graph::node::MAIN_PASS,
            draw_3d_graph::node::TAA,
        );
        draw_3d_graph.add_node_edge(
            draw_3d_graph::node::TAA,
            bevy_core_pipeline::core_3d::graph::node::BLOOM,
        );
        draw_3d_graph.add_node_edge(
            draw_3d_graph::node::TAA,
            bevy_core_pipeline::core_3d::graph::node::TONEMAPPING,
        );
    }
}

#[derive(Bundle, Default)]
pub struct TemporalAntialiasBundle {
    pub settings: TemporalAntialiasSettings,
    pub depth_prepass: DepthPrepass,
    pub velocity_prepass: VelocityPrepass,
    pub jitter: TemporalJitter,
}

#[derive(Component, Reflect, Clone)]
pub struct TemporalAntialiasSettings {
    pub depth_rejection_enabled: bool,
    pub velocity_rejection_enabled: bool,
}

impl Default for TemporalAntialiasSettings {
    fn default() -> Self {
        Self {
            velocity_rejection_enabled: false,
            depth_rejection_enabled: false,
        }
    }
}

struct TAANode {
    view_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static TAAHistoryTextures,
        &'static ViewPrepassTextures,
        &'static TAAPipelineId,
    )>,
}

impl TAANode {
    const IN_VIEW: &'static str = "view";

    fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for TAANode {
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
        let _taa_span = info_span!("taa").entered();

        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (
            Ok((camera, view_target, taa_history_textures, prepass_textures, taa_pipeline_id)),
            Some(pipelines),
            Some(pipeline_cache),
        ) = (
            self.view_query.get_manual(world, view_entity),
            world.get_resource::<TAAPipeline>(),
            world.get_resource::<PipelineCache>(),
        ) else {
            return Ok(());
        };
        let (
            Some(taa_pipeline),
            Some(prepass_velocity_texture),
            Some(prepass_depth_texture),
            Some(prepass_previous_velocity_texture),
            Some(prepass_previous_depth_texture)
        ) = (
            pipeline_cache.get_render_pipeline(taa_pipeline_id.0),
            &prepass_textures.velocity,
            &prepass_textures.depth,
            &prepass_textures.previous_velocity,
            &prepass_textures.previous_depth,
        ) else {
            return Ok(());
        };
        let view_target = view_target.post_process_write();

        let taa_bind_group = render_context
            .render_device
            .create_bind_group(&BindGroupDescriptor {
                label: Some("taa_bind_group"),
                layout: &pipelines.taa_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&view_target.source),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::TextureView(
                            &taa_history_textures.read.default_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: BindingResource::TextureView(
                            &prepass_velocity_texture.default_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&prepass_depth_texture.default_view),
                    },
                    BindGroupEntry {
                        binding: 4,
                        resource: BindingResource::TextureView(
                            &prepass_previous_velocity_texture.default_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 5,
                        resource: BindingResource::TextureView(
                            &prepass_previous_depth_texture.default_view,
                        ),
                    },
                    BindGroupEntry {
                        binding: 6,
                        resource: BindingResource::Sampler(&pipelines.nearest_sampler),
                    },
                    BindGroupEntry {
                        binding: 7,
                        resource: BindingResource::Sampler(&pipelines.linear_sampler),
                    },
                ],
            });

        {
            let mut taa_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &(RenderPassDescriptor {
                        label: Some("taa_pass"),
                        color_attachments: &[
                            Some(RenderPassColorAttachment {
                                view: view_target.destination,
                                resolve_target: None,
                                ops: Operations::default(),
                            }),
                            Some(RenderPassColorAttachment {
                                view: &taa_history_textures.write.default_view,
                                resolve_target: None,
                                ops: Operations::default(),
                            }),
                        ],
                        depth_stencil_attachment: None,
                    }),
                ));
            taa_pass.set_render_pipeline(taa_pipeline);
            taa_pass.set_bind_group(0, &taa_bind_group, &[]);
            if let Some(viewport) = camera.viewport.as_ref() {
                taa_pass.set_camera_viewport(viewport);
            }
            taa_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}

#[derive(Resource)]
struct TAAPipeline {
    taa_bind_group_layout: BindGroupLayout,
    nearest_sampler: Sampler,
    linear_sampler: Sampler,
}

impl FromWorld for TAAPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let nearest_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("taa_nearest_sampler"),
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Nearest,
            ..SamplerDescriptor::default()
        });
        let linear_sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("taa_linear_sampler"),
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..SamplerDescriptor::default()
        });

        let taa_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("taa_bind_group_layout"),
                entries: &[
                    // View target (read)
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // TAA History (read)
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Velocity
                    BindGroupLayoutEntry {
                        binding: 2,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Depth
                    BindGroupLayoutEntry {
                        binding: 3,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Previous Velocity
                    BindGroupLayoutEntry {
                        binding: 4,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Previous Depth
                    BindGroupLayoutEntry {
                        binding: 5,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Depth,
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    // Nearest sampler
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    // Linear sampler
                    BindGroupLayoutEntry {
                        binding: 7,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        TAAPipeline {
            taa_bind_group_layout,
            nearest_sampler,
            linear_sampler,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct TAAPipelineKey {
    hdr: bool,
    velocity_rejection_enabled: bool,
    depth_rejection_enabled: bool,
}

impl SpecializedRenderPipeline for TAAPipeline {
    type Key = TAAPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![];

        let format = if key.hdr {
            shader_defs.push("TONEMAP".into());
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            TextureFormat::bevy_default()
        };

        if key.depth_rejection_enabled {
            shader_defs.push("DEPTH_REJECTION".into());
        }
        if key.velocity_rejection_enabled {
            shader_defs.push("VELOCITY_REJECTION".into());
        }

        RenderPipelineDescriptor {
            label: Some("taa_pipeline".into()),
            layout: Some(vec![self.taa_bind_group_layout.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: TAA_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "taa".into(),
                targets: vec![
                    Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                    Some(ColorTargetState {
                        format,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }),
                ],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
        }
    }
}

fn extract_taa_settings(
    mut commands: Commands,
    cameras_3d: Extract<
        Query<
            (Entity, &Camera, &TemporalAntialiasSettings),
            (
                With<Camera3d>,
                With<TemporalJitter>,
                With<DepthPrepass>,
                With<VelocityPrepass>,
            ),
        >,
    >,
) {
    for (entity, camera, taa_settings) in &cameras_3d {
        // TODO: Check prepass history against TAA settings
        if camera.is_active {
            commands.get_or_spawn(entity).insert(taa_settings.clone());
        }
    }
}

#[derive(Component)]
struct TAAHistoryTextures {
    write: CachedTexture,
    read: CachedTexture,
}

fn prepare_taa_history_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    frame_count: Res<FrameCount>,
    views: Query<(Entity, &ExtractedCamera, &ExtractedView), With<TemporalAntialiasSettings>>,
) {
    for (entity, camera, view) in &views {
        if let Some(physical_viewport_size) = camera.physical_viewport_size {
            let mut texture_descriptor = TextureDescriptor {
                label: None,
                size: Extent3d {
                    depth_or_array_layers: 1,
                    width: physical_viewport_size.x,
                    height: physical_viewport_size.y,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
            };

            texture_descriptor.label = Some("taa_history_1_texture");
            let history_1_texture = texture_cache.get(&render_device, texture_descriptor.clone());

            texture_descriptor.label = Some("taa_history_2_texture");
            let history_2_texture = texture_cache.get(&render_device, texture_descriptor);

            let textures = if frame_count.0 % 2 == 0 {
                TAAHistoryTextures {
                    write: history_1_texture,
                    read: history_2_texture,
                }
            } else {
                TAAHistoryTextures {
                    write: history_2_texture,
                    read: history_1_texture,
                }
            };

            commands.entity(entity).insert(textures);
        }
    }
}

#[derive(Component)]
struct TAAPipelineId(CachedRenderPipelineId);

fn prepare_taa_pipelines(
    mut commands: Commands,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<TAAPipeline>>,
    pipeline: Res<TAAPipeline>,
    views: Query<(Entity, &ExtractedView, &TemporalAntialiasSettings)>,
) {
    for (entity, view, taa_settings) in &views {
        let pipeline_key = TAAPipelineKey {
            hdr: view.hdr,
            velocity_rejection_enabled: taa_settings.velocity_rejection_enabled,
            depth_rejection_enabled: taa_settings.depth_rejection_enabled,
        };

        let pipeline_id = pipelines.specialize(&mut pipeline_cache, &pipeline, pipeline_key);

        commands.entity(entity).insert(TAAPipelineId(pipeline_id));
    }
}
