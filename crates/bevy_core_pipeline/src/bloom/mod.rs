pub mod settings;
pub use self::settings::*;

use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::{QueryState, With},
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::{UVec2, Vec4};
use bevy_reflect::TypeUuid;
use bevy_render::{
    camera::ExtractedCamera,
    prelude::{Camera, Color},
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

impl BloomSettings {
    fn compute_blend_factor(&self, mip: f32, max_mip: f32) -> f32 {
        let x = mip / max_mip;

        // fn sigmoid(x: f32, curvature: f32) -> f32 {
        //     (x - curvature * x) / (curvature - 2.0 * curvature * x.abs() + 1.0)
        // }

        let c = (2.0 * x.powf(self.bump_angle) - 1.0).powi(2);
        // let s = (1.0 + sigmoid(0.5_f32.powf(1.0 / self.bump_angle) - x, -0.99999)) / 2.0;
        let s = (1.0 + (if x < self.bump_angle { 1.0 } else { 0.0 })) / 2.0; // this is the same as above
        let d = self.far_contribution + (self.near_contribution - self.far_contribution) * s;
        let blend = (1.0 - c * (1.0 - d)) * self.top_intensity;

        return blend;
    }
}

struct BloomNode {
    view_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static BloomTexture,
        &'static BloomBindGroups,
        &'static BloomUniformIndex,
        &'static BloomSettings,
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

    // Atypically for a post-processing effect, we do not
    // use a secondary texture normally provided by view_target.post_process_write(),
    // instead we write into our own bloom texture and then directly back onto main.
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
        let (
            Ok((camera, view_target, bloom_texture, bind_groups, uniform_index, bloom_settings)),
            Some(bloom_uniforms),
        ) = (
            self.view_query.get_manual(world, view_entity),
            bloom_uniforms.buffer.binding(),
        ) else {
            return Ok(());
        };

        let (
            Some(downsampling_first_pipeline),
            Some(downsampling_pipeline),
            Some(upsampling_pipeline),
            Some(upsampling_final_pipeline),
        ) = (
            // Downsampling pipelines
            pipeline_cache.get_render_pipeline(pipelines.downsampling_first_pipeline),
            pipeline_cache.get_render_pipeline(pipelines.downsampling_pipeline),

            // Upsampling pipleines.
            // Get normal(energy conserving) or additive based on bloom_settings.mode
            pipeline_cache.get_render_pipeline(match bloom_settings.mode {
                BloomMode::EnergyConserving => pipelines.upsampling_pipeline,
                BloomMode::Additive => pipelines.additive_upsampling_pipeline,
            }),
            pipeline_cache.get_render_pipeline(match bloom_settings.mode {
                BloomMode::EnergyConserving => pipelines.upsampling_final_pipeline,
                BloomMode::Additive => pipelines.additive_upsampling_final_pipeline,
            }),
        ) else {
            return Ok(());
        };

        let downsampling_first_bind_group =
            render_context
                .render_device
                .create_bind_group(&BindGroupDescriptor {
                    label: Some("bloom_downsampling_first_bind_group"),
                    layout: &pipelines.main_bind_group_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            // We read from main texture directly
                            resource: BindingResource::TextureView(view_target.main_texture()),
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
            let blend = bloom_settings
                .compute_blend_factor((mip) as f32, (bloom_texture.mip_count - 1) as f32);
            upsampling_pass.set_blend_constant(Color::rgb_linear(blend, blend, blend));
            upsampling_pass.draw(0..3, 0..1);
        }

        // This is very similar to the upsampling_pass above with the only difference
        // being the pipeline (which itself is barely different) and the color attachment.
        // Too bad.
        {
            let mut upsampling_final_pass =
                TrackedRenderPass::new(render_context.command_encoder.begin_render_pass(
                    &RenderPassDescriptor {
                        label: Some("bloom_upsampling_final_pass"),
                        color_attachments: &[Some(
                            // We draw directly onto the main texture
                            view_target.get_unsampled_color_attachment(Operations {
                                load: LoadOp::Load,
                                store: true,
                            }),
                        )],
                        depth_stencil_attachment: None,
                    },
                ));
            upsampling_final_pass.set_render_pipeline(upsampling_final_pipeline);
            upsampling_final_pass.set_bind_group(
                0,
                &bind_groups.upsampling_bind_groups[(bloom_texture.mip_count - 1) as usize],
                &[uniform_index.0],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                upsampling_final_pass.set_camera_viewport(viewport);
            }
            let blend =
                bloom_settings.compute_blend_factor(0.0, (bloom_texture.mip_count - 1) as f32);
            upsampling_final_pass.set_blend_constant(Color::rgb_linear(blend, blend, blend));
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
    // A bit of a waste having normal and additive
    // pipelines exist at the same time but I'm not sure
    // how to avoid that. We need access to bloom settings
    // to create them conditionally.
    //
    // TODO: Remove additive pipelines in favor of
    // conditionally defining the normal pipelines
    // with the proper blend components
    additive_upsampling_pipeline: CachedRenderPipelineId,
    additive_upsampling_final_pipeline: CachedRenderPipelineId,

    main_bind_group_layout: BindGroupLayout,

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
                    //
                    // TODO: We don't need this if bloom is configured in an
                    // energy conserving manner (no threshold).
                    // We might be punishing most users by doing this.
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

        let mut pipeline_cache = world.resource_mut::<PipelineCache>();

        let downsampling_first_pipeline =
            pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                label: Some("bloom_downsampling_first_pipeline".into()),
                layout: Some(vec![main_bind_group_layout.clone()]),
                vertex: fullscreen_shader_vertex_state(),
                fragment: Some(FragmentState {
                    shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                    shader_defs: vec!["FIRST_DOWNSAMPLE".into()],
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

        macro_rules! upsampling_pipeline {
            // This macro uses pipeline_cache.queue_render_pipeline()
            // to create a pipeline for upsampling. We use it because
            // most things between all upsampling pipelines are the same.
            (
                $label:expr,
                $texture_format:expr,
                $color_blend:expr
            ) => {
                pipeline_cache.queue_render_pipeline(RenderPipelineDescriptor {
                    label: Some($label.into()),
                    layout: Some(vec![main_bind_group_layout.clone()]),
                    vertex: fullscreen_shader_vertex_state(),
                    fragment: Some(FragmentState {
                        shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                        shader_defs: vec![],
                        entry_point: "upsample".into(),
                        targets: vec![Some(ColorTargetState {
                            format: $texture_format,
                            blend: Some(BlendState {
                                color: $color_blend,
                                alpha: BlendComponent::REPLACE,
                            }),
                            write_mask: ColorWrites::ALL,
                        })],
                    }),
                    primitive: PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: MultisampleState::default(),
                })
            };
        }

        macro_rules! energy_conserving_upsampling_pipeline {
            (
                $label:expr,
                $texture_format:expr
            ) => {
                upsampling_pipeline!(
                    $label,
                    $texture_format,
                    // At the time of developing this we decided to blend our
                    // blur pyramid levels using native WGPU render pass blend
                    // constants. They are set in the node's run function.
                    // This seemed like a good approach at the time which allowed
                    // us to perform complex calculations for blend levels on the CPU,
                    // however, we missed the fact that this prevented us from using
                    // textures to customize bloom apperance on individual parts
                    // of the screen and create effects such as lens dirt or
                    // screen blur behind certain UI elements.
                    //
                    // TODO: Use alpha instead of blend constants and move
                    // compute_blend_factor to the shader. The shader
                    // will likely need to know current mip number or
                    // mip "angle" (original texture is 0deg, max mip is 90deg)
                    // so make sure you give it that as a uniform.
                    // That does have to be provided per each pass unlike other
                    // uniforms that are set once.
                    BlendComponent {
                        src_factor: BlendFactor::Constant,
                        dst_factor: BlendFactor::OneMinusConstant,
                        operation: BlendOperation::Add,
                    }
                )
            };
        }

        macro_rules! additive_upsampling_pipeline {
            (
                $label:expr,
                $texture_format:expr
            ) => {
                upsampling_pipeline!(
                    $label,
                    $texture_format,
                    BlendComponent {
                        src_factor: BlendFactor::Constant,
                        dst_factor: BlendFactor::One,
                        operation: BlendOperation::Add,
                    }
                )
            };
        }

        let upsampling_pipeline = energy_conserving_upsampling_pipeline!(
            "bloom_upsampling_pipeline",
            TextureFormat::Rg11b10Float
        );
        let upsampling_final_pipeline = energy_conserving_upsampling_pipeline!(
            "bloom_upsampling_final_pipeline",
            ViewTarget::TEXTURE_FORMAT_HDR
        );
        let additive_upsampling_pipeline =
            additive_upsampling_pipeline!("bloom_upsampling_pipeline", TextureFormat::Rg11b10Float);
        let additive_upsampling_final_pipeline = additive_upsampling_pipeline!(
            "bloom_upsampling_final_pipeline",
            ViewTarget::TEXTURE_FORMAT_HDR
        );

        BloomPipelines {
            downsampling_first_pipeline,
            downsampling_pipeline,
            upsampling_pipeline,
            upsampling_final_pipeline,
            additive_upsampling_pipeline,
            additive_upsampling_final_pipeline,

            sampler,

            main_bind_group_layout,
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
            // How many times we can halve the resolution
            let mip_count = (min_view.log2().floor() as u32).max(1);

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

#[derive(ShaderType)]
struct BloomUniform {
    // Precomputed values used when thresholding, see https://catlikecoding.com/unity/tutorials/advanced-rendering/bloom/#3.4
    threshold_precomputations: Vec4,
}

#[derive(Resource, Default)]
struct BloomUniforms {
    buffer: DynamicUniformBuffer<BloomUniform>,
}

#[derive(Component)]
struct BloomUniformIndex(u32);

// TODO: This entire system is not needed if bloom is configured in an
// energy conserving manner. Because we only change the preflter settings here.
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
            let knee = settings.prefilter_settings.threshold
                * settings
                    .prefilter_settings
                    .threshold_softness
                    .clamp(0.0, 1.0);
            let uniform = BloomUniform {
                threshold_precomputations: Vec4::new(
                    settings.prefilter_settings.threshold,
                    settings.prefilter_settings.threshold - knee,
                    2.0 * knee,
                    0.25 / (knee + 0.00001),
                ),
            };

            let index = bloom_uniforms.buffer.push(uniform);
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
                        // TODO: This is unused. Uniforms only matter in prefiltering (first downsample)
                        BindGroupEntry {
                            binding: 2,
                            resource: bloom_uniforms.clone(),
                        },
                    ],
                });

                downsampling_bind_groups.push(bind_group);
            }

            let mut upsampling_bind_groups = Vec::with_capacity(bind_group_count);
            for mip in (0..bloom_texture.mip_count).rev() {
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
                        // TODO: This is unused. Uniforms only matter in prefiltering (first downsample)
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
