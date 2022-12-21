use crate::{core_2d, core_3d, fullscreen_vertex_shader::fullscreen_shader_vertex_state};
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::{QueryItem, QueryState, With},
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::UVec2;
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponent, ExtractComponentPlugin,
        UniformComponentPlugin,
    },
    prelude::Camera,
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
    render_phase::TrackedRenderPass,
    render_resource::*,
    renderer::{RenderContext, RenderDevice},
    texture::{CachedTexture, TextureCache},
    view::ViewTarget,
    RenderApp, RenderStage,
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;
use bevy_utils::HashMap;
use std::num::NonZeroU32;

const BLOOM_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 929599476923908);

pub struct BloomPlugin;

impl Plugin for BloomPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, BLOOM_SHADER_HANDLE, "bloom.wgsl", Shader::from_wgsl);

        app.register_type::<BloomSettings>();
        app.add_plugin(ExtractComponentPlugin::<BloomSettings>::default());
        app.add_plugin(UniformComponentPlugin::<BloomUniform>::default());

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<BloomPipelines>()
            .add_system_to_stage(RenderStage::Prepare, prepare_bloom_textures)
            .add_system_to_stage(RenderStage::Queue, queue_bloom_bind_groups);

        {
            let bloom_node = BloomNode::new(&mut render_app.world);
            let mut graph = render_app.world.resource_mut::<RenderGraph>();
            let draw_3d_graph = graph
                .get_sub_graph_mut(crate::core_3d::graph::NAME)
                .unwrap();
            draw_3d_graph.add_node(core_3d::graph::node::BLOOM, bloom_node);
            draw_3d_graph.add_slot_edge(
                draw_3d_graph.input_node().id,
                crate::core_3d::graph::input::VIEW_ENTITY,
                core_3d::graph::node::BLOOM,
                BloomNode::IN_VIEW,
            );
            // MAIN_PASS -> BLOOM -> TONEMAPPING
            draw_3d_graph.add_node_edge(
                crate::core_3d::graph::node::MAIN_PASS,
                core_3d::graph::node::BLOOM,
            );
            draw_3d_graph.add_node_edge(
                core_3d::graph::node::BLOOM,
                crate::core_3d::graph::node::TONEMAPPING,
            );
        }

        {
            let bloom_node = BloomNode::new(&mut render_app.world);
            let mut graph = render_app.world.resource_mut::<RenderGraph>();
            let draw_2d_graph = graph
                .get_sub_graph_mut(crate::core_2d::graph::NAME)
                .unwrap();
            draw_2d_graph.add_node(core_2d::graph::node::BLOOM, bloom_node);
            draw_2d_graph.add_slot_edge(
                draw_2d_graph.input_node().id,
                crate::core_2d::graph::input::VIEW_ENTITY,
                core_2d::graph::node::BLOOM,
                BloomNode::IN_VIEW,
            );
            // MAIN_PASS -> BLOOM -> TONEMAPPING
            draw_2d_graph.add_node_edge(
                crate::core_2d::graph::node::MAIN_PASS,
                core_2d::graph::node::BLOOM,
            );
            draw_2d_graph.add_node_edge(
                core_2d::graph::node::BLOOM,
                crate::core_2d::graph::node::TONEMAPPING,
            );
        }
    }
}

/// Applies a bloom effect to a HDR-enabled 2d or 3d camera.
///
/// Bloom causes bright objects to "glow", emitting a halo of light around them.
///
/// Often used in conjunction with `bevy_pbr::StandardMaterial::emissive`.
///
/// Note: This light is not "real" in the way directional or point lights are.
///
/// Bloom will not cast shadows or bend around other objects - it is purely a post-processing
/// effect overlaid on top of the already-rendered scene.
///
/// See also <https://en.wikipedia.org/wiki/Bloom_(shader_effect)>.
#[derive(Component, Reflect, Clone)]
pub struct BloomSettings {
    /// Baseline of the threshold curve (default: 1.0).
    ///
    /// RGB values under the threshold curve will not have bloom applied.
    pub threshold: f32,

    /// Knee of the threshold curve (default: 0.1).
    pub knee: f32,

    /// Scale used when upsampling (default: 1.0).
    pub scale: f32,

    /// Intensity of the bloom effect (default: 0.3).
    pub intensity: f32,
}

impl Default for BloomSettings {
    fn default() -> Self {
        Self {
            threshold: 1.0,
            knee: 0.1,
            scale: 1.0,
            intensity: 0.3,
        }
    }
}

impl ExtractComponent for BloomSettings {
    type Query = (&'static Self, &'static Camera);

    type Filter = ();
    type Out = BloomUniform;

    fn extract_component((settings, camera): QueryItem<'_, Self::Query>) -> Option<Self::Out> {
        if !(camera.is_active && camera.hdr) {
            return None;
        }

        camera.physical_viewport_size().map(|size| {
            let min_view = size.x.min(size.y) / 2;
            let mip_count = calculate_mip_count(min_view);
            let scale = (min_view / 2u32.pow(mip_count)) as f32 / 8.0;

            BloomUniform {
                threshold: settings.threshold,
                knee: settings.knee,
                scale: settings.scale * scale,
                intensity: settings.intensity,
            }
        })
    }
}

pub struct BloomNode {
    view_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static BloomTextures,
        &'static BloomBindGroups,
        &'static DynamicUniformIndex<BloomUniform>,
    )>,
}

impl BloomNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
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
            downsampling_prefilter_pipeline,
            downsampling_pipeline,
            upsampling_pipeline,
            upsampling_final_pipeline,
        ) = match (
            pipeline_cache.get_render_pipeline(pipelines.downsampling_prefilter_pipeline),
            pipeline_cache.get_render_pipeline(pipelines.downsampling_pipeline),
            pipeline_cache.get_render_pipeline(pipelines.upsampling_pipeline),
            pipeline_cache.get_render_pipeline(pipelines.upsampling_final_pipeline),
        ) {
            (Some(p1), Some(p2), Some(p3), Some(p4)) => (p1, p2, p3, p4),
            _ => return Ok(()),
        };

        {
            let view = &BloomTextures::texture_view(&textures.texture_a, 0);
            let mut prefilter_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &RenderPassDescriptor {
                        label: Some("bloom_prefilter_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                    },
                ));
            prefilter_pass.set_render_pipeline(downsampling_prefilter_pipeline);
            prefilter_pass.set_bind_group(
                0,
                &bind_groups.prefilter_bind_group,
                &[uniform_index.index()],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                prefilter_pass.set_camera_viewport(viewport);
            }
            prefilter_pass.draw(0..3, 0..1);
        }

        for mip in 1..textures.mip_count {
            let view = &BloomTextures::texture_view(&textures.texture_a, mip);
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
                &[uniform_index.index()],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                downsampling_pass.set_camera_viewport(viewport);
            }
            downsampling_pass.draw(0..3, 0..1);
        }

        for mip in (1..textures.mip_count).rev() {
            let view = &BloomTextures::texture_view(&textures.texture_b, mip - 1);
            let mut upsampling_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &RenderPassDescriptor {
                        label: Some("bloom_upsampling_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view,
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                    },
                ));
            upsampling_pass.set_render_pipeline(upsampling_pipeline);
            upsampling_pass.set_bind_group(
                0,
                &bind_groups.upsampling_bind_groups[mip as usize - 1],
                &[uniform_index.index()],
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
                        color_attachments: &[Some(view_target.get_unsampled_color_attachment(
                            Operations {
                                load: LoadOp::Load,
                                store: true,
                            },
                        ))],
                        depth_stencil_attachment: None,
                    },
                ));
            upsampling_final_pass.set_render_pipeline(upsampling_final_pipeline);
            upsampling_final_pass.set_bind_group(
                0,
                &bind_groups.upsampling_final_bind_group,
                &[uniform_index.index()],
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
    downsampling_prefilter_pipeline: CachedRenderPipelineId,
    downsampling_pipeline: CachedRenderPipelineId,
    upsampling_pipeline: CachedRenderPipelineId,
    upsampling_final_pipeline: CachedRenderPipelineId,
    sampler: Sampler,
    downsampling_bind_group_layout: BindGroupLayout,
    upsampling_bind_group_layout: BindGroupLayout,
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

        let downsampling_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bloom_downsampling_bind_group_layout"),
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

        let upsampling_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bloom_upsampling_bind_group_layout"),
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

        let downsampling_prefilter_pipeline =
            pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("bloom_downsampling_prefilter_pipeline".into()),
                layout: Some(vec![downsampling_bind_group_layout.clone()]),
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                    shader_defs: vec![],
                    entry_point: "downsample_prefilter".into(),
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

        let downsampling_pipeline =
            pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("bloom_downsampling_pipeline".into()),
                layout: Some(vec![downsampling_bind_group_layout.clone()]),
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                    shader_defs: vec![],
                    entry_point: "downsample".into(),
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

        let upsampling_pipeline = pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
            label: Some("bloom_upsampling_pipeline".into()),
            layout: Some(vec![upsampling_bind_group_layout.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "upsample".into(),
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

        let upsampling_final_pipeline =
            pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("bloom_upsampling_final_pipeline".into()),
                layout: Some(vec![downsampling_bind_group_layout.clone()]),
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                    shader_defs: vec![],
                    entry_point: "upsample_final".into(),
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
            downsampling_prefilter_pipeline,
            downsampling_pipeline,
            upsampling_pipeline,
            upsampling_final_pipeline,
            sampler,
            downsampling_bind_group_layout,
            upsampling_bind_group_layout,
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
    views: Query<(Entity, &ExtractedCamera), With<BloomUniform>>,
) {
    let mut texture_as = HashMap::default();
    let mut texture_bs = HashMap::default();
    for (entity, camera) in &views {
        if let Some(UVec2 {
            x: width,
            y: height,
        }) = camera.physical_viewport_size
        {
            let min_view = width.min(height) / 2;
            let mip_count = calculate_mip_count(min_view);

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

/// The uniform struct extracted from [`BloomSettings`] attached to a [`Camera`].
/// Will be available for use in the Bloom shader.
#[doc(hidden)]
#[derive(Component, ShaderType, Clone)]
pub struct BloomUniform {
    threshold: f32,
    knee: f32,
    scale: f32,
    intensity: f32,
}

#[derive(Component)]
struct BloomBindGroups {
    prefilter_bind_group: BindGroup,
    downsampling_bind_groups: Box<[BindGroup]>,
    upsampling_bind_groups: Box<[BindGroup]>,
    upsampling_final_bind_group: BindGroup,
}

fn queue_bloom_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipelines: Res<BloomPipelines>,
    uniforms: Res<ComponentUniforms<BloomUniform>>,
    views: Query<(Entity, &ViewTarget, &BloomTextures)>,
) {
    if let Some(uniforms) = uniforms.binding() {
        for (entity, view_target, textures) in &views {
            let prefilter_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("bloom_prefilter_bind_group"),
                layout: &pipelines.downsampling_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(view_target.main_texture()),
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

            let bind_group_count = textures.mip_count as usize - 1;

            let mut downsampling_bind_groups = Vec::with_capacity(bind_group_count);
            for mip in 1..textures.mip_count {
                let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                    label: Some("bloom_downsampling_bind_group"),
                    layout: &pipelines.downsampling_bind_group_layout,
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

                downsampling_bind_groups.push(bind_group);
            }

            let mut upsampling_bind_groups = Vec::with_capacity(bind_group_count);
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
                    label: Some("bloom_upsampling_bind_group"),
                    layout: &pipelines.upsampling_bind_group_layout,
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

                upsampling_bind_groups.push(bind_group);
            }

            let upsampling_final_bind_group =
                render_device.create_bind_group(&BindGroupDescriptor {
                    label: Some("bloom_upsampling_final_bind_group"),
                    layout: &pipelines.downsampling_bind_group_layout,
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
                prefilter_bind_group,
                downsampling_bind_groups: downsampling_bind_groups.into_boxed_slice(),
                upsampling_bind_groups: upsampling_bind_groups.into_boxed_slice(),
                upsampling_final_bind_group,
            });
        }
    }
}

fn calculate_mip_count(min_view: u32) -> u32 {
    ((min_view as f32).log2().round() as i32 - 3).max(1) as u32
}
