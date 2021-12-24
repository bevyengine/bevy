use std::{num::NonZeroU32, sync::Mutex};

use bevy_app::Plugin;
use bevy_core_pipeline::MainPass3dNode;
use bevy_ecs::prelude::*;
use bevy_render::{
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
    render_resource::std140::{AsStd140, Std140},
    render_resource::*,
    renderer::{RenderContext, RenderDevice, RenderQueue},
    view::{ExtractedView, ViewTarget},
    RenderApp, RenderStage,
};

pub struct BloomPlugin;

impl Plugin for BloomPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        app.init_resource::<BloomSettings>();

        let render_app = app.sub_app_mut(RenderApp);
        render_app
            .init_resource::<BloomShaders>()
            .add_system_to_stage(RenderStage::Extract, extract_bloom_settings);

        let bloom_node = BloomNode::new(&mut render_app.world);

        let mut render_graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

        let draw_3d_graph = render_graph
            .get_sub_graph_mut(bevy_core_pipeline::draw_3d_graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(BloomNode::NODE_NAME, bloom_node);

        draw_3d_graph
            .add_node_edge(
                bevy_core_pipeline::draw_3d_graph::node::MAIN_PASS,
                BloomNode::NODE_NAME,
            )
            .unwrap();
        draw_3d_graph
            .add_node_edge(
                BloomNode::NODE_NAME,
                bevy_core_pipeline::draw_3d_graph::node::TONEMAPPING,
            )
            .unwrap();

        draw_3d_graph
            .add_slot_edge(
                draw_3d_graph.input_node().unwrap().id,
                bevy_core_pipeline::draw_3d_graph::input::VIEW_ENTITY,
                BloomNode::NODE_NAME,
                BloomNode::IN_VIEW,
            )
            .unwrap();

        draw_3d_graph
            .add_slot_edge(
                bevy_core_pipeline::draw_3d_graph::node::MAIN_PASS,
                MainPass3dNode::OUT_TEXTURE,
                BloomNode::NODE_NAME,
                BloomNode::IN_HDR,
            )
            .unwrap();
    }
}

fn extract_bloom_settings(mut commands: Commands, bloom_settings: Res<BloomSettings>) {
    commands.insert_resource(bloom_settings.into_inner().clone());
}

/// Resources used by [`BloomNode`].
pub struct BloomShaders {
    pub down_sampling_pipeline: RenderPipeline,
    pub down_sampling_pre_filter_pipeline: RenderPipeline,
    pub up_sampling_pipeline: RenderPipeline,
    pub up_sampling_final_pipeline: RenderPipeline,
    pub shader_module: ShaderModule,
    pub down_layout: BindGroupLayout,
    pub up_layout: BindGroupLayout,
    pub sampler: Sampler,
}

impl FromWorld for BloomShaders {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.get_resource::<RenderDevice>().unwrap();
        let shader = ShaderModuleDescriptor {
            label: Some("bloom shader"),
            source: ShaderSource::Wgsl(include_str!("bloom.wgsl").into()),
        };
        let shader_module = render_device.create_shader_module(&shader);

        let down_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("bloom_down_sampling_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                },
            ],
        });

        let up_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("bloom_up_sampling_layout"),
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 2,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 3,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    visibility: ShaderStages::VERTEX_FRAGMENT,
                    count: None,
                },
            ],
        });

        let down_pipeline_layout =
            render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
                label: Some("bloom_down_sampling_layout"),
                bind_group_layouts: &[&down_layout],
                push_constant_ranges: &[],
            });

        let up_pipeline_layout = render_device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("bloom_up_sampling_layout"),
            bind_group_layouts: &[&up_layout],
            push_constant_ranges: &[],
        });

        let down_sampling_pre_filter_pipeline =
            render_device.create_render_pipeline(&RawRenderPipelineDescriptor {
                label: Some("bloom_down_sampling_pre_filter_pipeline"),
                layout: Some(&down_pipeline_layout),
                vertex: RawVertexState {
                    module: &shader_module,
                    entry_point: "vertex",
                    buffers: &[],
                },
                fragment: Some(RawFragmentState {
                    module: &shader_module,
                    entry_point: "down_sample_pre_filter",
                    targets: &[ColorTargetState {
                        format: ViewTarget::TEXTURE_FORMAT_HDR,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }],
                }),
                primitive: PrimitiveState {
                    cull_mode: Some(Face::Back),
                    ..Default::default()
                },
                multisample: Default::default(),
                depth_stencil: None,
                multiview: None,
            });

        let down_sampling_pipeline =
            render_device.create_render_pipeline(&RawRenderPipelineDescriptor {
                label: Some("bloom_down_sampling_pipeline"),
                layout: Some(&down_pipeline_layout),
                vertex: RawVertexState {
                    module: &shader_module,
                    entry_point: "vertex",
                    buffers: &[],
                },
                fragment: Some(RawFragmentState {
                    module: &shader_module,
                    entry_point: "down_sample",
                    targets: &[ColorTargetState {
                        format: ViewTarget::TEXTURE_FORMAT_HDR,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }],
                }),
                primitive: PrimitiveState {
                    cull_mode: Some(Face::Back),
                    ..Default::default()
                },
                multisample: Default::default(),
                depth_stencil: None,
                multiview: None,
            });

        let up_sampling_pipeline =
            render_device.create_render_pipeline(&RawRenderPipelineDescriptor {
                label: Some("bloom_up_sampling_pipeline"),
                layout: Some(&up_pipeline_layout),
                vertex: RawVertexState {
                    module: &shader_module,
                    entry_point: "vertex",
                    buffers: &[],
                },
                fragment: Some(RawFragmentState {
                    module: &shader_module,
                    entry_point: "up_sample",
                    targets: &[ColorTargetState {
                        format: ViewTarget::TEXTURE_FORMAT_HDR,
                        blend: None,
                        write_mask: ColorWrites::ALL,
                    }],
                }),
                primitive: PrimitiveState {
                    cull_mode: Some(Face::Back),
                    ..Default::default()
                },
                multisample: Default::default(),
                depth_stencil: None,
                multiview: None,
            });

        let up_sampling_final_pipeline =
            render_device.create_render_pipeline(&RawRenderPipelineDescriptor {
                label: Some("bloom_up_sampling_final_pipeline"),
                layout: Some(&down_pipeline_layout),
                vertex: RawVertexState {
                    module: &shader_module,
                    entry_point: "vertex",
                    buffers: &[],
                },
                fragment: Some(RawFragmentState {
                    module: &shader_module,
                    entry_point: "up_sample_final",
                    targets: &[ColorTargetState {
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
                    }],
                }),
                primitive: PrimitiveState {
                    cull_mode: Some(Face::Back),
                    ..Default::default()
                },
                multisample: Default::default(),
                depth_stencil: None,
                multiview: None,
            });

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        BloomShaders {
            down_sampling_pre_filter_pipeline,
            down_sampling_pipeline,
            up_sampling_pipeline,
            up_sampling_final_pipeline,
            shader_module,
            down_layout,
            up_layout,
            sampler,
        }
    }
}

struct MipChain {
    mips: u32,
    scale: f32,
    width: u32,
    height: u32,
    target_id: TextureViewId,
    tex_a: Texture,
    tex_b: Texture,
    pre_filter_bind_group: BindGroup,
    down_sampling_bind_groups: Vec<BindGroup>,
    up_sampling_bind_groups: Vec<BindGroup>,
    up_sampling_final_bind_group: BindGroup,
}

impl MipChain {
    fn new(
        render_device: &RenderDevice,
        bloom_shaders: &BloomShaders,
        uniforms_buffer: &Buffer,
        hdr_target: &TextureView,
        width: u32,
        height: u32,
    ) -> Self {
        let min_element = width.min(height) / 2;
        let mut mips = 1;

        while min_element / 2u32.pow(mips) > 4 {
            mips += 1;
        }

        let size = Extent3d {
            width: (width / 2).max(1),
            height: (height / 2).max(1),
            depth_or_array_layers: 1,
        };

        let tex_a = render_device.create_texture(&TextureDescriptor {
            label: Some("bloom_tex_a"),
            size,
            mip_level_count: mips,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: ViewTarget::TEXTURE_FORMAT_HDR,
            usage: TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING,
        });

        let tex_b = render_device.create_texture(&TextureDescriptor {
            label: Some("bloom_tex_b"),
            size,
            mip_level_count: mips,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: ViewTarget::TEXTURE_FORMAT_HDR,
            usage: TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING,
        });

        let pre_filter_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("bloom_pre_filter_bind_group"),
            layout: &bloom_shaders.down_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(hdr_target),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&bloom_shaders.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: uniforms_buffer.as_entire_binding(),
                },
            ],
        });

        let mut down_sampling_bind_groups = Vec::new();

        for mip in 1..mips {
            let view = tex_a.create_view(&TextureViewDescriptor {
                label: None,
                base_mip_level: mip - 1,
                mip_level_count: Some(unsafe { NonZeroU32::new_unchecked(1) }),
                ..Default::default()
            });

            let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("bloom_down_sampling_bind_group"),
                layout: &bloom_shaders.down_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&bloom_shaders.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: uniforms_buffer.as_entire_binding(),
                    },
                ],
            });

            down_sampling_bind_groups.push(bind_group);
        }

        let mut up_sampling_bind_groups = Vec::new();

        for mip in 1..mips {
            let up = tex_a.create_view(&TextureViewDescriptor {
                label: None,
                base_mip_level: mip - 1,
                mip_level_count: Some(unsafe { NonZeroU32::new_unchecked(1) }),
                ..Default::default()
            });

            let org_tex = if mip == mips - 1 { &tex_a } else { &tex_b };

            let org = org_tex.create_view(&TextureViewDescriptor {
                label: None,
                base_mip_level: mip,
                mip_level_count: Some(unsafe { NonZeroU32::new_unchecked(1) }),
                ..Default::default()
            });

            let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("bloom_up_sampling_bind_group"),
                layout: &bloom_shaders.up_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&org),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&bloom_shaders.sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: uniforms_buffer.as_entire_binding(),
                    },
                    BindGroupEntry {
                        binding: 3,
                        resource: BindingResource::TextureView(&up),
                    },
                ],
            });

            up_sampling_bind_groups.push(bind_group);
        }

        let org = tex_b.create_view(&TextureViewDescriptor {
            label: None,
            base_mip_level: 0,
            ..Default::default()
        });

        let up_sampling_final_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
            label: Some("bloom_up_sampling_final_bind_group"),
            layout: &bloom_shaders.down_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&org),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&bloom_shaders.sampler),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: uniforms_buffer.as_entire_binding(),
                },
            ],
        });

        Self {
            mips,
            scale: (min_element / 2u32.pow(mips)) as f32 / 8.0,
            width,
            height,
            target_id: hdr_target.id(),
            tex_a,
            tex_b,
            pre_filter_bind_group,
            down_sampling_bind_groups,
            up_sampling_bind_groups,
            up_sampling_final_bind_group,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, AsStd140, Default, Debug)]
struct Uniforms {
    threshold: f32,
    knee: f32,
    scale: f32,
}

/// Settings for bloom.
#[derive(Clone, Debug)]
pub struct BloomSettings {
    /// Enables bloom.
    pub enabled: bool,
    /// Threshold for for bloom to apply.
    pub threshold: f32,
    /// Adjusts the threshold curve.
    pub knee: f32,
    /// Scale used when up sampling.
    pub up_sample_scale: f32,
}

impl Default for BloomSettings {
    #[inline]
    fn default() -> Self {
        Self {
            enabled: true,
            threshold: 1.0,
            knee: 0.1,
            up_sample_scale: 1.0,
        }
    }
}

/// Applies bloom effect to the input texture.
///
/// Use [`BloomSettings`] to configure the effect at runtime.
pub struct BloomNode {
    query: QueryState<&'static ExtractedView>,
    uniforms_buffer: Option<Buffer>,
    mip_chain: Mutex<Option<MipChain>>,
}

impl BloomNode {
    pub fn new(render_world: &mut World) -> Self {
        BloomNode {
            query: QueryState::new(render_world),
            uniforms_buffer: None,
            mip_chain: Mutex::new(None),
        }
    }
}

impl BloomNode {
    pub const NODE_NAME: &'static str = "bloom";
    pub const IN_VIEW: &'static str = "view_entity";
    pub const IN_HDR: &'static str = "hdr_target";
}

impl Node for BloomNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![
            SlotInfo::new(Self::IN_VIEW, SlotType::Entity),
            SlotInfo::new(Self::IN_HDR, SlotType::TextureView),
        ]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);

        if self.uniforms_buffer.is_none() {
            let render_device = world.get_resource::<RenderDevice>().unwrap();

            let buffer = render_device.create_buffer(&BufferDescriptor {
                label: Some("bloom_uniforms_buffer"),
                size: Uniforms::std140_size_static() as u64,
                mapped_at_creation: false,
                usage: BufferUsages::COPY_DST | BufferUsages::UNIFORM,
            });

            self.uniforms_buffer = Some(buffer);
        }
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;

        let view = match self.query.get_manual(world, view_entity) {
            Ok(view) => view,
            Err(_) => return Ok(()),
        };

        let bloom_shaders = world.get_resource::<BloomShaders>().unwrap();

        let render_queue = world.get_resource::<RenderQueue>().unwrap();
        let settings = world.get_resource::<BloomSettings>().unwrap();

        if !settings.enabled {
            return Ok(());
        }

        let hdr_target = graph.get_input_texture(Self::IN_HDR)?;

        let uniforms_buffer = self.uniforms_buffer.as_ref().unwrap();
        let mut mip_chain = self.mip_chain.lock().unwrap();

        let mip_chain = if let Some(ref mut mip_chain) = *mip_chain {
            // if the window changes but the size of the new window doesn't, using ExtractedWindow.size_changed
            // wouldn't trigger a resize, comparing the size ensures that it does
            if mip_chain.width != view.width
                || mip_chain.height != view.height
                || mip_chain.target_id != hdr_target.id()
            {
                *mip_chain = MipChain::new(
                    &render_context.render_device,
                    bloom_shaders,
                    uniforms_buffer,
                    hdr_target,
                    view.width,
                    view.height,
                );
            }

            mip_chain
        } else {
            *mip_chain = Some(MipChain::new(
                &render_context.render_device,
                bloom_shaders,
                uniforms_buffer,
                hdr_target,
                view.width,
                view.height,
            ));

            mip_chain.as_ref().unwrap()
        };

        let uniforms = Uniforms {
            threshold: settings.threshold,
            knee: settings.knee,
            scale: settings.up_sample_scale * mip_chain.scale,
        };

        render_queue.write_buffer(uniforms_buffer, 0, uniforms.as_std140().as_bytes());

        let view = mip_chain.tex_a.create_view(&TextureViewDescriptor {
            mip_level_count: Some(unsafe { NonZeroU32::new_unchecked(1) }),
            ..Default::default()
        });

        {
            let mut pre_filter_pass =
                render_context
                    .command_encoder
                    .begin_render_pass(&RenderPassDescriptor {
                        label: Some("bloom_pre_filter_pass"),
                        color_attachments: &[RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(RawColor::BLACK),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });

            pre_filter_pass.set_pipeline(&bloom_shaders.down_sampling_pre_filter_pipeline);
            pre_filter_pass.set_bind_group(0, &mip_chain.pre_filter_bind_group, &[]);
            pre_filter_pass.draw(0..3, 0..1);
        }

        for mip in 1..mip_chain.mips {
            let view = mip_chain.tex_a.create_view(&TextureViewDescriptor {
                base_mip_level: mip,
                mip_level_count: Some(unsafe { NonZeroU32::new_unchecked(1) }),
                ..Default::default()
            });

            let mut down_sampling_pass =
                render_context
                    .command_encoder
                    .begin_render_pass(&RenderPassDescriptor {
                        label: Some("bloom_down_sampling_pass"),
                        color_attachments: &[RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(RawColor::BLACK),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });

            down_sampling_pass.set_pipeline(&bloom_shaders.down_sampling_pipeline);
            down_sampling_pass.set_bind_group(
                0,
                &mip_chain.down_sampling_bind_groups[mip as usize - 1],
                &[],
            );
            down_sampling_pass.draw(0..3, 0..1);
        }

        for mip in (1..mip_chain.mips).rev() {
            let view = mip_chain.tex_b.create_view(&TextureViewDescriptor {
                base_mip_level: mip - 1,
                mip_level_count: Some(unsafe { NonZeroU32::new_unchecked(1) }),
                ..Default::default()
            });

            let mut up_sampling_pass =
                render_context
                    .command_encoder
                    .begin_render_pass(&RenderPassDescriptor {
                        label: Some("bloom_up_sampling_pass"),
                        color_attachments: &[RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Clear(RawColor::BLACK),
                                store: true,
                            },
                        }],
                        depth_stencil_attachment: None,
                    });

            up_sampling_pass.set_pipeline(&bloom_shaders.up_sampling_pipeline);
            up_sampling_pass.set_bind_group(
                0,
                &mip_chain.up_sampling_bind_groups[mip as usize - 1],
                &[],
            );
            up_sampling_pass.draw(0..3, 0..1);
        }

        let mut up_sampling_final_pass =
            render_context
                .command_encoder
                .begin_render_pass(&RenderPassDescriptor {
                    label: Some("bloom_up_sampling_final_pass"),
                    color_attachments: &[RenderPassColorAttachment {
                        view: hdr_target,
                        resolve_target: None,
                        ops: Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    }],
                    depth_stencil_attachment: None,
                });

        up_sampling_final_pass.set_pipeline(&bloom_shaders.up_sampling_final_pipeline);
        up_sampling_final_pass.set_bind_group(0, &mip_chain.up_sampling_final_bind_group, &[]);
        up_sampling_final_pass.draw(0..3, 0..1);

        Ok(())
    }
}
