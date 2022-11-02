use std::num::NonZeroU32;

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_core_pipeline::{
    fullscreen_vertex_shader::fullscreen_shader_vertex_state, prelude::Camera3d,
};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::{QueryState, With},
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::UVec2;
use bevy_reflect::TypeUuid;
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
use bevy_utils::HashMap;

pub mod draw_3d_graph {
    pub mod node {
        /// Label for the bloom render node.
        pub const BLOOM: &str = "bloom";
    }
}

const BLOOM_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 929599476923908);

pub struct BloomPlugin;

impl Plugin for BloomPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, BLOOM_SHADER_HANDLE, "bloom.wgsl", Shader::from_wgsl);

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

        let bloom_node = BloomNode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        let draw_3d_graph = graph
            .get_sub_graph_mut(bevy_core_pipeline::core_3d::graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(draw_3d_graph::node::BLOOM, bloom_node);
        draw_3d_graph
            .add_slot_edge(
                draw_3d_graph.input_node().unwrap().id,
                bevy_core_pipeline::core_3d::graph::input::VIEW_ENTITY,
                draw_3d_graph::node::BLOOM,
                BloomNode::IN_VIEW,
            )
            .unwrap();
        // MAIN_PASS -> BLOOM -> TONEMAPPING
        draw_3d_graph
            .add_node_edge(
                bevy_core_pipeline::core_3d::graph::node::MAIN_PASS,
                draw_3d_graph::node::BLOOM,
            )
            .unwrap();
        draw_3d_graph
            .add_node_edge(
                draw_3d_graph::node::BLOOM,
                bevy_core_pipeline::core_3d::graph::node::TONEMAPPING,
            )
            .unwrap();
    }
}

// TODO: Write better documentation.
/// Applies a bloom effect to an HDR-enabled Camera3d.
///
/// See also https://en.wikipedia.org/wiki/Bloom_(shader_effect).
#[derive(Component, Clone)]
pub struct BloomSettings {
    /// Threshold for bloom to apply.
    pub threshold: f32,
    /// Adjusts the threshold curve.
    pub knee: f32,
    /// Scale used when upsampling.
    pub up_sample_scale: f32,
    /// Scale the intensity of the bloom effect. Defaults to 1.0.
    pub intensity: f32,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            threshold: 1.0,
            knee: 0.1,
            up_sample_scale: 1.0,
            intensity: 1.0,
        }
    }
}

struct BloomNode {
    view_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static BloomTextures,
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
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (camera, view_target, textures, bind_groups, uniform_index) =
            match self.view_query.get_manual(world, view_entity) {
                Ok(result) => result,
                _ => return Ok(()),
            };
        let (
            down_sampling_pre_filter_pipeline,
            down_sampling_pipeline,
            up_sampling_pipeline,
            up_sampling_final_pipeline,
        ) = match (
            pipeline_cache.get_render_pipeline(pipelines.down_sampling_pre_filter_pipeline),
            pipeline_cache.get_render_pipeline(pipelines.down_sampling_pipeline),
            pipeline_cache.get_render_pipeline(pipelines.up_sampling_pipeline),
            pipeline_cache.get_render_pipeline(pipelines.up_sampling_final_pipeline),
        ) {
            (Some(p1), Some(p2), Some(p3), Some(p4)) => (p1, p2, p3, p4),
            _ => return Ok(()),
        };

        {
            let view = &BloomTextures::texture_view(&textures.texture_a, 0);
            let mut pre_filter_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &RenderPassDescriptor {
                        label: Some("bloom_pre_filter_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                    },
                ));
            pre_filter_pass.set_render_pipeline(&down_sampling_pre_filter_pipeline);
            pre_filter_pass.set_bind_group(
                0,
                &bind_groups.pre_filter_bind_group,
                &[uniform_index.0],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                pre_filter_pass.set_camera_viewport(viewport);
            }
            pre_filter_pass.draw(0..3, 0..1);
        }

        for mip in 1..textures.mip_count {
            let view = &BloomTextures::texture_view(&textures.texture_a, mip);
            let mut down_sampling_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &RenderPassDescriptor {
                        label: Some("bloom_down_sampling_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                    },
                ));
            down_sampling_pass.set_render_pipeline(&down_sampling_pipeline);
            down_sampling_pass.set_bind_group(
                0,
                &bind_groups.down_sampling_bind_groups[mip as usize - 1],
                &[uniform_index.0],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                down_sampling_pass.set_camera_viewport(viewport);
            }
            down_sampling_pass.draw(0..3, 0..1);
        }

        for mip in (1..textures.mip_count).rev() {
            let view = &BloomTextures::texture_view(&textures.texture_b, mip - 1);
            let mut up_sampling_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &RenderPassDescriptor {
                        label: Some("bloom_up_sampling_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                    },
                ));
            up_sampling_pass.set_render_pipeline(&up_sampling_pipeline);
            up_sampling_pass.set_bind_group(
                0,
                &bind_groups.up_sampling_bind_groups[mip as usize - 1],
                &[uniform_index.0],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                up_sampling_pass.set_camera_viewport(viewport);
            }
            up_sampling_pass.draw(0..3, 0..1);
        }

        {
            let mut up_sampling_final_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &RenderPassDescriptor {
                        label: Some("bloom_up_sampling_final_pass"),
                        color_attachments: &[Some(view_target.get_color_attachment(Operations {
                            load: LoadOp::Load,
                            store: true,
                        }))],
                        depth_stencil_attachment: None,
                    },
                ));
            up_sampling_final_pass.set_render_pipeline(&up_sampling_final_pipeline);
            up_sampling_final_pass.set_bind_group(
                0,
                &bind_groups.up_sampling_final_bind_group,
                &[uniform_index.0],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                up_sampling_final_pass.set_camera_viewport(viewport);
            }
            up_sampling_final_pass.draw(0..3, 0..1);
        }

        Ok(())
    }
}

#[derive(Resource)]
struct BloomPipelines {
    down_sampling_pre_filter_pipeline: CachedRenderPipelineId,
    down_sampling_pipeline: CachedRenderPipelineId,
    up_sampling_pipeline: CachedRenderPipelineId,
    up_sampling_final_pipeline: CachedRenderPipelineId,
    sampler: Sampler,
    down_sampling_bind_group_layout: BindGroupLayout,
    up_sampling_bind_group_layout: BindGroupLayout,
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

        let down_sampling_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bloom_down_sampling_bind_group_layout"),
                entries: &[
                    // Upsampled input texture (downsampled for final upsample)
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
                            min_binding_size: Some(BloomUniform::min_size()),
                        },
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                ],
            });

        let up_sampling_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bloom_up_sampling_bind_group_layout"),
                entries: &[
                    // Downsampled input texture
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
                            min_binding_size: Some(BloomUniform::min_size()),
                        },
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                    // Upsampled input texture
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

        let down_sampling_pre_filter_pipeline =
            pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("bloom_down_sampling_pre_filter_pipeline".into()),
                layout: Some(vec![down_sampling_bind_group_layout.clone()]),
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                    shader_defs: vec![],
                    entry_point: "down_sample_pre_filter".into(),
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

        let down_sampling_pipeline =
            pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("bloom_down_sampling_pipeline".into()),
                layout: Some(vec![down_sampling_bind_group_layout.clone()]),
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                    shader_defs: vec![],
                    entry_point: "down_sample".into(),
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

        let up_sampling_pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("bloom_up_sampling_pipeline".into()),
            layout: Some(vec![up_sampling_bind_group_layout.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "up_sample".into(),
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

        let up_sampling_final_pipeline =
            pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("bloom_up_sampling_final_pipeline".into()),
                layout: Some(vec![down_sampling_bind_group_layout.clone()]),
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                    shader_defs: vec![],
                    entry_point: "up_sample_final".into(),
                    targets: vec![Some(ColorTargetState {
                        format: ViewTarget::TEXTURE_FORMAT_HDR,
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

        BloomPipelines {
            down_sampling_pre_filter_pipeline,
            down_sampling_pipeline,
            up_sampling_pipeline,
            up_sampling_final_pipeline,
            sampler,
            down_sampling_bind_group_layout,
            up_sampling_bind_group_layout,
        }
    }
}

fn extract_bloom_settings(
    mut commands: Commands,
    cameras_3d: Extract<Query<(Entity, &Camera, &BloomSettings), With<Camera3d>>>,
) {
    for (entity, camera, bloom_settings) in &cameras_3d {
        if camera.is_active && camera.hdr {
            commands.get_or_spawn(entity).insert(bloom_settings.clone());
        }
    }
}

#[derive(Component)]
struct BloomTextures {
    texture_a: CachedTexture,
    texture_b: CachedTexture,
    mip_count: u32,
}

impl BloomTextures {
    fn texture_view(texture: &CachedTexture, base_mip_level: u32) -> TextureView {
        texture.texture.create_view(&TextureViewDescriptor {
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
    let mut texture_as = HashMap::default();
    let mut texture_bs = HashMap::default();
    for (entity, camera) in &views {
        if let Some(UVec2 {
            x: width,
            y: height,
        }) = camera.physical_viewport_size
        {
            let min_element = width.min(height) / 2;
            let mut mip_count = 1;
            while min_element / 2u32.pow(mip_count) > 4 {
                mip_count += 1;
            }

            let mut texture_descriptor = TextureDescriptor {
                label: None,
                size: Extent3d {
                    width: (width / 2).max(1),
                    height: (height / 2).max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: mip_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: ViewTarget::TEXTURE_FORMAT_HDR,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            };

            texture_descriptor.label = Some("bloom_texture_a");
            let texture_a = texture_as
                .entry(camera.target.clone())
                .or_insert_with(|| texture_cache.get(&render_device, texture_descriptor.clone()))
                .clone();

            texture_descriptor.label = Some("bloom_texture_b");
            let texture_b = texture_bs
                .entry(camera.target.clone())
                .or_insert_with(|| texture_cache.get(&render_device, texture_descriptor))
                .clone();

            commands.entity(entity).insert(BloomTextures {
                texture_a,
                texture_b,
                mip_count,
            });
        }
    }
}

#[derive(ShaderType)]
struct BloomUniform {
    threshold: f32,
    knee: f32,
    scale: f32,
    intensity: f32,
}

#[derive(Resource, Default)]
struct BloomUniforms {
    uniforms: DynamicUniformBuffer<BloomUniform>,
}

#[derive(Component)]
struct BloomUniformIndex(u32);

fn prepare_bloom_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut bloom_uniforms: ResMut<BloomUniforms>,
    bloom_query: Query<(Entity, &ExtractedCamera, &BloomSettings)>,
) {
    bloom_uniforms.uniforms.clear();

    let entities = bloom_query
        .iter()
        .filter_map(|(entity, camera, settings)| {
            let size = match camera.physical_viewport_size {
                Some(size) => size,
                None => return None,
            };
            let min_element = size.x.min(size.y) / 2;
            let mut mip_count = 1;
            while min_element / 2u32.pow(mip_count) > 4 {
                mip_count += 1;
            }
            let scale = (min_element / 2u32.pow(mip_count)) as f32 / 8.0;

            let uniform = BloomUniform {
                threshold: settings.threshold,
                knee: settings.knee,
                scale: settings.up_sample_scale * scale,
                intensity: settings.intensity,
            };
            let index = bloom_uniforms.uniforms.push(uniform);
            Some((entity, (BloomUniformIndex(index))))
        })
        .collect::<Vec<_>>();
    commands.insert_or_spawn_batch(entities);

    bloom_uniforms
        .uniforms
        .write_buffer(&render_device, &render_queue);
}

#[derive(Component)]
struct BloomBindGroups {
    pre_filter_bind_group: BindGroup,
    down_sampling_bind_groups: Box<[BindGroup]>,
    up_sampling_bind_groups: Box<[BindGroup]>,
    up_sampling_final_bind_group: BindGroup,
}

fn queue_bloom_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Res<BloomPipelines>,
    uniforms: Res<BloomUniforms>,
    views: Query<(Entity, &ViewTarget, &BloomTextures)>,
) {
    if let Some(uniforms) = uniforms.uniforms.binding() {
        for (entity, view_target, textures) in &views {
            let pre_filter_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("bloom_pre_filter_bind_group"),
                layout: &pipelines.down_sampling_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(view_target.main_texture.texture()),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&pipelines.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: uniforms.clone(),
                    },
                ],
            });

            let mut down_sampling_bind_groups = Vec::new();
            for mip in 1..textures.mip_count {
                let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                    label: Some("bloom_down_sampling_bind_group"),
                    layout: &pipelines.down_sampling_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&BloomTextures::texture_view(
                                &textures.texture_a,
                                mip - 1,
                            )),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&pipelines.sampler),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: uniforms.clone(),
                        },
                    ],
                });

                down_sampling_bind_groups.push(bind_group);
            }

            let mut up_sampling_bind_groups = Vec::new(); // TODO: Make these boxed slices
            for mip in 1..textures.mip_count {
                let up = BloomTextures::texture_view(&textures.texture_a, mip - 1);
                let org = BloomTextures::texture_view(
                    if mip == textures.mip_count - 1 {
                        &textures.texture_a
                    } else {
                        &textures.texture_b
                    },
                    mip,
                );

                let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                    label: Some("bloom_up_sampling_bind_group"),
                    layout: &pipelines.up_sampling_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&org),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&pipelines.sampler),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: uniforms.clone(),
                        },
                        BindGroupEntry {
                            binding: 3,
                            resource: BindingResource::TextureView(&up),
                        },
                    ],
                });

                up_sampling_bind_groups.push(bind_group);
            }

            let up_sampling_final_bind_group =
                render_device.create_bind_group(&BindGroupDescriptor {
                    label: Some("bloom_up_sampling_final_bind_group"),
                    layout: &pipelines.down_sampling_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: BindingResource::TextureView(&BloomTextures::texture_view(
                                &textures.texture_b,
                                0,
                            )),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: BindingResource::Sampler(&pipelines.sampler),
                        },
                        BindGroupEntry {
                            binding: 2,
                            resource: uniforms.clone(),
                        },
                    ],
                });

            commands.entity(entity).insert(BloomBindGroups {
                pre_filter_bind_group,
                down_sampling_bind_groups: down_sampling_bind_groups.into_boxed_slice(),
                up_sampling_bind_groups: up_sampling_bind_groups.into_boxed_slice(),
                up_sampling_final_bind_group,
            });
        }
    }
}
