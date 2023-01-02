pub mod settings;
pub use self::settings::*;

mod downsampling_pipeline;
mod upsampling_pipeline;
use self::{downsampling_pipeline::*, upsampling_pipeline::*};

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

const BLOOM_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rg11b10Float;

impl BloomSettings {
    /// Calculates blend intesnities of blur pyramid levels
    /// during the upsampling+compositing stage.
    ///
    /// The function assumes all pyramid levels are upsampled and
    /// blended into higher frequency ones using this function to
    /// calculate blend levels every time. The final (highest frequency)
    /// pyramid level in not blended into anything therefore this function
    /// is not applied to it. As a result, the *mip* Earameter of 0 indicates
    /// the second-highest frequency pyramid level (in our case that is the
    /// 0th mip of the bloom texture with the original image being the
    /// actual highest frequency level).
    ///
    /// Parameters:
    /// * *mip* - the index of the lower frequency pyramid level (0 - max_mip, where 0 indicates highest frequency mip but not the highest frequency image).
    /// * *max_mip* - the index of the lowest frequency pyramid level.
    ///
    /// This function can be visually previewed for all values of *mip* (normalized) with tweakable
    /// BloomSettings parameters on [Desmos graphing calculator](https://www.desmos.com/calculator/ncc8xbhzzl).
    fn compute_blend_factor(&self, mip: f32, max_mip: f32) -> f32 {
        let x = mip / max_mip;

        let mut lf_boost =
            (1.0 - (1.0 - x).powf(1.0 / (1.0 - self.lf_boost_curvature))) * self.lf_boost;
        let high_pass_lq =
            1.0 - ((x - self.high_pass_frequency) / self.high_pass_frequency).clamp(0.0, 1.0);
        // let high_pass_hq = 0.5 + 0.5 * ((x - self.high_pass_frequency) / self.high_pass_frequency).clamp(0.0, 1.0).mul(std::f32::consts::PI).cos();
        let high_pass = high_pass_lq;

        lf_boost *= match self.composite_mode {
            BloomCompositeMode::EnergyConserving => 1.0 - self.intensity,
            BloomCompositeMode::Additive => 1.0,
        };

        (self.intensity + lf_boost) * high_pass
    }
}

pub struct BloomPlugin;

impl Plugin for BloomPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, BLOOM_SHADER_HANDLE, "bloom.wgsl", Shader::from_wgsl);

        app.register_type::<BloomSettings>();
        app.register_type::<BloomPrefilterSettings>();
        app.register_type::<BloomCompositeMode>();

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<BloomPipelines>()
            .init_resource::<BloomDownsamplingPipeline>()
            .init_resource::<BloomUpsamplingPipeline>()
            .init_resource::<SpecializedRenderPipelines<BloomDownsamplingPipeline>>()
            .init_resource::<SpecializedRenderPipelines<BloomUpsamplingPipeline>>()
            .init_resource::<BloomUniforms>()
            .add_system_to_stage(RenderStage::Extract, extract_bloom_settings)
            .add_system_to_stage(RenderStage::Prepare, prepare_bloom_textures)
            .add_system_to_stage(RenderStage::Prepare, prepare_bloom_uniforms)
            .add_system_to_stage(RenderStage::Prepare, prepare_downsampling_pipeline)
            .add_system_to_stage(RenderStage::Prepare, prepare_upsampling_pipeline)
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

struct BloomNode {
    view_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static BloomTexture,
        &'static BloomBindGroups,
        &'static BloomUniformIndices,
        &'static BloomSettings,
        &'static UpsamplingPipelineIds,
        &'static BloomDownsamplingPipelineIds,
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
        let downsampling_pipeline_res = world.resource::<BloomDownsamplingPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let bloom_uniforms = world.resource::<BloomUniforms>();
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (
            Ok((camera, view_target, bloom_texture, bind_groups, uniform_index, bloom_settings, upsampling_pipeline_ids, downsampling_pipeline_ids)),
            Some(bloom_uniforms),
        ) = (
            self.view_query.get_manual(world, view_entity),
            bloom_uniforms.buffer.binding(),
        ) else {
            return Ok(());
        };

        // Downsampling pipelines
        let (
            Some(downsampling_first_pipeline),
            Some(downsampling_pipeline),
        ) = (
            pipeline_cache.get_render_pipeline(downsampling_pipeline_ids.id_first),
            pipeline_cache.get_render_pipeline(downsampling_pipeline_ids.id_main),
        ) else {
            return Ok(());
        };

        // Upsampling pipleines
        let (
            Some(upsampling_pipeline),
            Some(upsampling_final_pipeline),
        ) = (
            pipeline_cache.get_render_pipeline(upsampling_pipeline_ids.id_main),
            pipeline_cache.get_render_pipeline(upsampling_pipeline_ids.id_final),
        ) else {
            return Ok(());
        };

        {
            let needs_settings = bloom_settings.prefilter_settings.threshold > 0.0;
            let downsampling_first_bind_group = {
                let texture = BindGroupEntry {
                    binding: 0,
                    // We read from main texture directly
                    resource: BindingResource::TextureView(view_target.main_texture()),
                };

                let sampler = BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Sampler(&pipelines.sampler),
                };

                let settings = BindGroupEntry {
                    binding: 2,
                    resource: bloom_uniforms.clone(),
                };

                if needs_settings {
                    render_context
                        .render_device
                        .create_bind_group(&BindGroupDescriptor {
                            label: Some("bloom_downsampling_first_bind_group"),
                            layout: &downsampling_pipeline_res.extended_bind_group_layout,
                            entries: &[texture, sampler, settings],
                        })
                } else {
                    render_context
                        .render_device
                        .create_bind_group(&BindGroupDescriptor {
                            label: Some("bloom_downsampling_first_bind_group"),
                            layout: &downsampling_pipeline_res.bind_group_layout,
                            entries: &[texture, sampler],
                        })
                }
            };

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
            if needs_settings {
                downsampling_first_pass.set_bind_group(
                    0,
                    &downsampling_first_bind_group,
                    &[uniform_index.downsampling],
                );
            } else {
                downsampling_first_pass.set_bind_group(0, &downsampling_first_bind_group, &[]);
            };
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
                &[],
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
                &[],
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
                &[],
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

        BloomPipelines { sampler }
    }
}

fn extract_bloom_settings(
    mut commands: Commands,
    cameras: Extract<Query<(Entity, &Camera, &BloomSettings), With<Camera>>>,
) {
    for (entity, camera, bloom_settings) in &cameras {
        if camera.is_active {
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
                format: BLOOM_TEXTURE_FORMAT,
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
    buffer: DynamicUniformBuffer<BloomDownsamplingUniform>,
}

#[derive(Component)]
struct BloomUniformIndices {
    downsampling: u32,
}

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
            let uniform = BloomDownsamplingUniform {
                threshold_precomputations: Vec4::new(
                    settings.prefilter_settings.threshold,
                    settings.prefilter_settings.threshold - knee,
                    2.0 * knee,
                    0.25 / (knee + 0.00001),
                ),
            };

            (
                entity,
                (BloomUniformIndices {
                    downsampling: bloom_uniforms.buffer.push(uniform),
                }),
            )
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
    downsampling_pipeline: Res<BloomDownsamplingPipeline>,
    upsampling_pipeline: Res<BloomUpsamplingPipeline>,
    // bloom_uniforms: Res<BloomUniforms>,
    views: Query<(Entity, &BloomTexture)>,
) {
    for (entity, bloom_texture) in &views {
        let bind_group_count = bloom_texture.mip_count as usize - 1;

        let mut downsampling_bind_groups = Vec::with_capacity(bind_group_count);
        for mip in 1..bloom_texture.mip_count {
            let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("bloom_downsampling_bind_group"),
                layout: &downsampling_pipeline.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&bloom_texture.view(mip - 1)),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&pipelines.sampler),
                    },
                ],
            });

            downsampling_bind_groups.push(bind_group);
        }

        let mut upsampling_bind_groups = Vec::with_capacity(bind_group_count);
        for mip in (0..bloom_texture.mip_count).rev() {
            let bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("bloom_upsampling_bind_group"),
                layout: &upsampling_pipeline.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&bloom_texture.view(mip)),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(&pipelines.sampler),
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
