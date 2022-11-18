use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
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
    render_phase::TrackedRenderPass,
    render_resource::*,
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::{CachedTexture, TextureCache},
    view::ViewTarget,
    Extract, RenderApp, RenderStage,
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::HashMap;
use std::num::NonZeroU32;

pub mod draw_3d_graph {
    pub mod node {
        /// Label for the bloom render node.
        pub const BLOOM: &str = "bloom_3d";
    }
}
pub mod draw_2d_graph {
    pub mod node {
        /// Label for the bloom render node.
        pub const BLOOM: &str = "bloom_2d";
    }
}

const BLOOM_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 929599476923908);

pub struct BloomPlugin;

impl Plugin for BloomPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, BLOOM_SHADER_HANDLE, "bloom.wgsl", Shader::from_wgsl);

        app.register_type::<BloomSettings>();

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<BloomPipelines>()
            .init_resource::<BloomUniforms>()
            .add_system_to_stage(RenderStage::Extract, extract_bloom_settings)
            .add_system_to_stage(RenderStage::Prepare, prepare_bloom_textures)
            .add_system_to_stage(RenderStage::Prepare, prepare_bloom_uniforms)
            .add_system_to_stage(RenderStage::Queue, queue_bloom_bind_groups);

        // Add bloom to the 3d render graph
        {
            let bloom_node = BloomNode::new(&mut render_app.world);
            let mut graph = render_app.world.resource_mut::<RenderGraph>();
            let draw_3d_graph = graph
                .get_sub_graph_mut(crate::core_3d::graph::NAME)
                .unwrap();
            draw_3d_graph.add_node(draw_3d_graph::node::BLOOM, bloom_node);
            draw_3d_graph
                .add_slot_edge(
                    draw_3d_graph.input_node().unwrap().id,
                    crate::core_3d::graph::input::VIEW_ENTITY,
                    draw_3d_graph::node::BLOOM,
                    BloomNode::IN_VIEW,
                )
                .unwrap();
            // MAIN_PASS -> BLOOM -> TONEMAPPING
            draw_3d_graph
                .add_node_edge(
                    crate::core_3d::graph::node::MAIN_PASS,
                    draw_3d_graph::node::BLOOM,
                )
                .unwrap();
            draw_3d_graph
                .add_node_edge(
                    draw_3d_graph::node::BLOOM,
                    crate::core_3d::graph::node::TONEMAPPING,
                )
                .unwrap();
        }

        // Add bloom to the 2d render graph
        {
            let bloom_node = BloomNode::new(&mut render_app.world);
            let mut graph = render_app.world.resource_mut::<RenderGraph>();
            let draw_2d_graph = graph
                .get_sub_graph_mut(crate::core_2d::graph::NAME)
                .unwrap();
            draw_2d_graph.add_node(draw_2d_graph::node::BLOOM, bloom_node);
            draw_2d_graph
                .add_slot_edge(
                    draw_2d_graph.input_node().unwrap().id,
                    crate::core_2d::graph::input::VIEW_ENTITY,
                    draw_2d_graph::node::BLOOM,
                    BloomNode::IN_VIEW,
                )
                .unwrap();
            // MAIN_PASS -> BLOOM -> TONEMAPPING
            draw_2d_graph
                .add_node_edge(
                    crate::core_2d::graph::node::MAIN_PASS,
                    draw_2d_graph::node::BLOOM,
                )
                .unwrap();
            draw_2d_graph
                .add_node_edge(
                    draw_2d_graph::node::BLOOM,
                    crate::core_2d::graph::node::TONEMAPPING,
                )
                .unwrap();
        }
    }
}

/// ## Usage
///
/// Applies a bloom effect to a HDR-enabled 2d or 3d camera.
///
/// Bloom causes bright objects to "glow", emitting a halo of light around them.
///
/// Often used in conjunction with `bevy_pbr::StandardMaterial::emissive`.
///
/// Should be used along with a tonemapping function that maps each RGB component separately, such as ACES Filmic.
///
/// ## Note
///
/// This light is not "real" in the way directional or point lights are.
///
/// Bloom will not cast shadows or bend around other objects - it is purely a post-processing
/// effect overlaid on top of the already-rendered scene.
///
/// See also <https://en.wikipedia.org/wiki/Bloom_(shader_effect)>.
#[derive(Component, ShaderType, Reflect, Clone)]
pub struct BloomSettings {
    /// Intensity of the bloom effect (default: 0.04).
    pub intensity: f32,

    /// Baseline of the quadratic threshold curve (default: 0.0).
    ///
    /// RGB values under the threshold curve will not have bloom applied.
    /// Using a threshold is not physically accurate, but may fit better with your artistic direction.
    pub threshold_base: f32,

    /// Knee of the threshold curve (default: 0.0).
    pub threshold_knee: f32,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            intensity: 0.04,
            threshold_base: 0.0,
            threshold_knee: 0.0,
        }
    }
}

struct BloomNode {
    view_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static BloomTexture,
        &'static BloomBindGroups,
        &'static BloomUniformIndex,
    )>,
}

impl BloomNode {
    const IN_VIEW: &'static str = "view";

    fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for BloomNode {
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
        let _bloom_span = info_span!("bloom").entered();

        let pipelines = world.resource::<BloomPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let bloom_uniforms = world.resource::<BloomUniforms>();
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (camera, view_target, bloom_texture, bind_groups, uniform_index) =
            match self.view_query.get_manual(world, view_entity) {
                Ok(result) => result,
                _ => return Ok(()),
            };
        let (
            downsampling_first_pipeline,
            downsampling_pipeline,
            upsampling_pipeline,
            upsampling_final_pipeline,
            bloom_uniforms,
        ) = match (
            pipeline_cache.get_render_pipeline(pipelines.downsampling_first_pipeline),
            pipeline_cache.get_render_pipeline(pipelines.downsampling_pipeline),
            pipeline_cache.get_render_pipeline(pipelines.upsampling_pipeline),
            pipeline_cache.get_render_pipeline(pipelines.upsampling_final_pipeline),
            bloom_uniforms.buffer.binding(),
        ) {
            (Some(a), Some(b), Some(c), Some(d), Some(e)) => (a, b, c, d, e),
            _ => return Ok(()),
        };
        let view_target = view_target.post_process_write();

        let downsampling_first_bind_group =
            render_context
                .render_device
                .create_bind_group(&BindGroupDescriptor {
                    label: Some("bloom_downsampling_first_bind_group"),
                    layout: &pipelines.main_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(view_target.source),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&pipelines.sampler),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: bloom_uniforms.clone(),
                        },
                    ],
                });
        let upsampling_final_bind_group =
            render_context
                .render_device
                .create_bind_group(&BindGroupDescriptor {
                    label: Some("bloom_upsampling_final_bind_group"),
                    layout: &pipelines.upsampling_final_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&bloom_texture.view(0)),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&pipelines.sampler),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: bloom_uniforms,
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: BindingResource::TextureView(view_target.source),
                        },
                    ],
                });

        {
            let view = &bloom_texture.view(0);
            let mut downsampling_first_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &RenderPassDescriptor {
                        label: Some("bloom_downsampling_first_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                    },
                ));
            downsampling_first_pass.set_render_pipeline(downsampling_first_pipeline);
            downsampling_first_pass.set_bind_group(
                0,
                &downsampling_first_bind_group,
                &[uniform_index.0],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                downsampling_first_pass.set_camera_viewport(viewport);
            }
            downsampling_first_pass.draw(0..3, 0..1);
        }

        for mip in 1..bloom_texture.mip_count {
            let view = &bloom_texture.view(mip);
            let mut downsampling_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &RenderPassDescriptor {
                        label: Some("bloom_downsampling_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                    },
                ));
            downsampling_pass.set_render_pipeline(downsampling_pipeline);
            downsampling_pass.set_bind_group(
                0,
                &bind_groups.downsampling_bind_groups[mip as usize - 1],
                &[uniform_index.0],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                downsampling_pass.set_camera_viewport(viewport);
            }
            downsampling_pass.draw(0..3, 0..1);
        }

        for mip in (1..bloom_texture.mip_count).rev() {
            let view = &bloom_texture.view(mip - 1);
            let mut upsampling_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &RenderPassDescriptor {
                        label: Some("bloom_upsampling_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Load,
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    },
                ));
            upsampling_pass.set_render_pipeline(upsampling_pipeline);
            upsampling_pass.set_bind_group(
                0,
                &bind_groups.upsampling_bind_groups[(bloom_texture.mip_count - mip - 1) as usize],
                &[uniform_index.0],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                upsampling_pass.set_camera_viewport(viewport);
            }
            upsampling_pass.draw(0..3, 0..1);
        }

        {
            let mut upsampling_final_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &RenderPassDescriptor {
                        label: Some("bloom_upsampling_final_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view: view_target.destination,
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                    },
                ));
            upsampling_final_pass.set_render_pipeline(upsampling_final_pipeline);
            upsampling_final_pass.set_bind_group(
                0,
                &upsampling_final_bind_group,
                &[uniform_index.0],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                upsampling_final_pass.set_camera_viewport(viewport);
            }
            upsampling_final_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}

#[derive(Resource)]
struct BloomPipelines {
    downsampling_first_pipeline: CachedRenderPipelineId,
    downsampling_pipeline: CachedRenderPipelineId,
    upsampling_pipeline: CachedRenderPipelineId,
    upsampling_final_pipeline: CachedRenderPipelineId,

    main_bind_group_layout: BindGroupLayout,
    upsampling_final_bind_group_layout: BindGroupLayout,

    sampler: Sampler,
}

impl FromWorld for BloomPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        let main_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bloom_main_bind_group_layout"),
                entries: &[
                    // Input texture (higher resolution for downsample, lower resolution for upsample)
                    BindGroupLayoutEntry {
                        binding: 0,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                    // Sampler
                    BindGroupLayoutEntry {
                        binding: 1,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                    // Bloom settings
                    BindGroupLayoutEntry {
                        binding: 2,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(BloomSettings::min_size()),
                        },
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                ],
            });

        let upsampling_final_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bloom_upsampling_final_bind_group_layout"),
                entries: &[
                    // Main pass texture
                    BindGroupLayoutEntry {
                        binding: 0,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                    // Sampler
                    BindGroupLayoutEntry {
                        binding: 1,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                    // Bloom settings
                    BindGroupLayoutEntry {
                        binding: 2,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(BloomSettings::min_size()),
                        },
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                    // Second to last upsample result texture (BloomTexture mip 0)
                    BindGroupLayoutEntry {
                        binding: 3,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                ],
            });

        let mut pipeline_cache = world.resource_mut::<PipelineCache>();

        let downsampling_first_pipeline =
            pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("bloom_downsampling_first_pipeline".into()),
                layout: Some(vec![main_bind_group_layout.clone()]),
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                    shader_defs: vec![],
                    entry_point: "downsample_first".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::Rg11b10Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
            });

        let downsampling_pipeline =
            pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("bloom_downsampling_pipeline".into()),
                layout: Some(vec![main_bind_group_layout.clone()]),
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                    shader_defs: vec![],
                    entry_point: "downsample".into(),
                    targets: vec![Some(ColorTargetState {
                        format: TextureFormat::Rg11b10Float,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
            });

        let upsampling_pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("bloom_upsampling_pipeline".into()),
            layout: Some(vec![main_bind_group_layout.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "upsample".into(),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rg11b10Float,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
        });

        let upsampling_final_pipeline =
            pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("bloom_upsampling_final_pipeline".into()),
                layout: Some(vec![upsampling_final_bind_group_layout.clone()]),
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                    shader_defs: vec![],
                    entry_point: "upsample_final".into(),
                    targets: vec![Some(ColorTargetState {
                        format: ViewTarget::TEXTURE_FORMAT_HDR,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    })],
                }),
                primitive: PrimitiveState::default(),
                depth_stencil: None,
                multisample: MultisampleState::default(),
            });

        BloomPipelines {
            downsampling_first_pipeline,
            downsampling_pipeline,
            upsampling_pipeline,
            upsampling_final_pipeline,

            sampler,

            main_bind_group_layout,
            upsampling_final_bind_group_layout,
        }
    }
}

fn extract_bloom_settings(
    mut commands: Commands,
    cameras: Extract<Query<(Entity, &Camera, &BloomSettings), With<Camera>>>,
) {
    for (entity, camera, bloom_settings) in &cameras {
        if camera.is_active && camera.hdr {
            commands.get_or_spawn(entity).insert(bloom_settings.clone());
        }
    }
}

#[derive(Component)]
struct BloomTexture {
    // First mip is half the screen resolution, successive mips are half the previous
    texture: CachedTexture,
    mip_count: u32,
}

impl BloomTexture {
    fn view(&self, base_mip_level: u32) -> TextureView {
        self.texture.texture.create_view(&TextureViewDescriptor {
            base_mip_level,
            mip_level_count: Some(unsafe { NonZeroU32::new_unchecked(1) }),
            ..Default::default()
        })
    }
}

fn prepare_bloom_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera), With<BloomSettings>>,
) {
    let mut textures = HashMap::default();
    for (entity, camera) in &views {
        if let Some(UVec2 {
            x: width,
            y: height,
        }) = camera.physical_viewport_size
        {
            let min_view = width.min(height) as f32;
            // How many times we can halve the resolution, minus 3 to avoid tiny mips
            let mip_count = (min_view.log2().round() as i32 - 3).max(1) as u32;

            let texture_descriptor = TextureDescriptor {
                label: Some("bloom_texture"),
                size: Extent3d {
                    width: (width / 2).max(1),
                    height: (height / 2).max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: mip_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rg11b10Float,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            };

            let texture = textures
                .entry(camera.target.clone())
                .or_insert_with(|| texture_cache.get(&render_device, texture_descriptor))
                .clone();

            commands
                .entity(entity)
                .insert(BloomTexture { texture, mip_count });
        }
    }
}

#[derive(Resource, Default)]
struct BloomUniforms {
    buffer: DynamicUniformBuffer<BloomSettings>,
}

#[derive(Component)]
struct BloomUniformIndex(u32);

fn prepare_bloom_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut bloom_uniforms: ResMut<BloomUniforms>,
    bloom_query: Query<(Entity, &BloomSettings)>,
) {
    bloom_uniforms.buffer.clear();

    let entities = bloom_query
        .iter()
        .map(|(entity, settings)| {
            let index = bloom_uniforms.buffer.push(settings.clone());
            (entity, (BloomUniformIndex(index)))
        })
        .collect::<Vec<_>>();
    commands.insert_or_spawn_batch(entities);

    bloom_uniforms
        .buffer
        .write_buffer(&render_device, &render_queue);
}

#[derive(Component)]
struct BloomBindGroups {
    downsampling_bind_groups: Box<[BindGroup]>,
    upsampling_bind_groups: Box<[BindGroup]>,
}

fn queue_bloom_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Res<BloomPipelines>,
    bloom_uniforms: Res<BloomUniforms>,
    views: Query<(Entity, &BloomTexture)>,
) {
    if let Some(bloom_uniforms) = bloom_uniforms.buffer.binding() {
        for (entity, bloom_texture) in &views {
            let bind_group_count = bloom_texture.mip_count as usize - 1;

            let mut downsampling_bind_groups = Vec::with_capacity(bind_group_count);
            for mip in 1..bloom_texture.mip_count {
                let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                    label: Some("bloom_downsampling_bind_group"),
                    layout: &pipelines.main_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&bloom_texture.view(mip - 1)),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&pipelines.sampler),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: bloom_uniforms.clone(),
                        },
                    ],
                });

                downsampling_bind_groups.push(bind_group);
            }

            let mut upsampling_bind_groups = Vec::with_capacity(bind_group_count);
            for mip in (1..bloom_texture.mip_count).rev() {
                let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                    label: Some("bloom_upsampling_bind_group"),
                    layout: &pipelines.main_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&bloom_texture.view(mip)),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&pipelines.sampler),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: bloom_uniforms.clone(),
                        },
                    ],
                });

                upsampling_bind_groups.push(bind_group);
            }

            commands.entity(entity).insert(BloomBindGroups {
                downsampling_bind_groups: downsampling_bind_groups.into_boxed_slice(),
                upsampling_bind_groups: upsampling_bind_groups.into_boxed_slice(),
            });
        }
    }
}
