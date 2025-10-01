mod downsampling_pipeline;
mod settings;
mod upsampling_pipeline;

use bevy_image::ToExtents;
pub use settings::{Bloom, BloomCompositeMode, BloomPrefilter};

use crate::bloom::{
    downsampling_pipeline::init_bloom_downsampling_pipeline,
    upsampling_pipeline::init_bloom_upscaling_pipeline,
};
use bevy_app::{App, Plugin};
use bevy_asset::embedded_asset;
use bevy_color::{Gray, LinearRgba};
use bevy_core_pipeline::{
    core_2d::graph::{Core2d, Node2d},
    core_3d::graph::{Core3d, Node3d},
};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_math::{ops, UVec2};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponentPlugin, UniformComponentPlugin,
    },
    render_graph::{NodeRunError, RenderGraphContext, RenderGraphExt, ViewNode, ViewNodeRunner},
    render_resource::*,
    renderer::{RenderContext, RenderDevice},
    texture::{CachedTexture, TextureCache},
    view::ViewTarget,
    Render, RenderApp, RenderStartup, RenderSystems,
};
use downsampling_pipeline::{
    prepare_downsampling_pipeline, BloomDownsamplingPipeline, BloomDownsamplingPipelineIds,
    BloomUniforms,
};
#[cfg(feature = "trace")]
use tracing::info_span;
use upsampling_pipeline::{
    prepare_upsampling_pipeline, BloomUpsamplingPipeline, UpsamplingPipelineIds,
};

const BLOOM_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rg11b10Ufloat;

#[derive(Default)]
pub struct BloomPlugin;

impl Plugin for BloomPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "bloom.wgsl");

        app.add_plugins((
            ExtractComponentPlugin::<Bloom>::default(),
            UniformComponentPlugin::<BloomUniforms>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<BloomDownsamplingPipeline>>()
            .init_resource::<SpecializedRenderPipelines<BloomUpsamplingPipeline>>()
            .add_systems(
                RenderStartup,
                (
                    init_bloom_downsampling_pipeline,
                    init_bloom_upscaling_pipeline,
                ),
            )
            .add_systems(
                Render,
                (
                    prepare_downsampling_pipeline.in_set(RenderSystems::Prepare),
                    prepare_upsampling_pipeline.in_set(RenderSystems::Prepare),
                    prepare_bloom_textures.in_set(RenderSystems::PrepareResources),
                    prepare_bloom_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            // Add bloom to the 3d render graph
            .add_render_graph_node::<ViewNodeRunner<BloomNode>>(Core3d, Node3d::Bloom)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::StartMainPassPostProcessing,
                    Node3d::Bloom,
                    Node3d::Tonemapping,
                ),
            )
            // Add bloom to the 2d render graph
            .add_render_graph_node::<ViewNodeRunner<BloomNode>>(Core2d, Node2d::Bloom)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::StartMainPassPostProcessing,
                    Node2d::Bloom,
                    Node2d::Tonemapping,
                ),
            );
    }
}

#[derive(Default)]
struct BloomNode;
impl ViewNode for BloomNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static BloomTexture,
        &'static BloomBindGroups,
        &'static DynamicUniformIndex<BloomUniforms>,
        &'static Bloom,
        &'static UpsamplingPipelineIds,
        &'static BloomDownsamplingPipelineIds,
    );

    // Atypically for a post-processing effect, we do not need to
    // use a secondary texture normally provided by view_target.post_process_write(),
    // instead we write into our own bloom texture and then directly back onto main.
    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            camera,
            view_target,
            bloom_texture,
            bind_groups,
            uniform_index,
            bloom_settings,
            upsampling_pipeline_ids,
            downsampling_pipeline_ids,
        ): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        if bloom_settings.intensity == 0.0 {
            return Ok(());
        }

        let downsampling_pipeline_res = world.resource::<BloomDownsamplingPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let uniforms = world.resource::<ComponentUniforms<BloomUniforms>>();

        let (
            Some(uniforms),
            Some(downsampling_first_pipeline),
            Some(downsampling_pipeline),
            Some(upsampling_pipeline),
            Some(upsampling_final_pipeline),
        ) = (
            uniforms.binding(),
            pipeline_cache.get_render_pipeline(downsampling_pipeline_ids.first),
            pipeline_cache.get_render_pipeline(downsampling_pipeline_ids.main),
            pipeline_cache.get_render_pipeline(upsampling_pipeline_ids.id_main),
            pipeline_cache.get_render_pipeline(upsampling_pipeline_ids.id_final),
        )
        else {
            return Ok(());
        };

        let view_texture = view_target.main_texture_view();
        let view_texture_unsampled = view_target.get_unsampled_color_attachment();
        let diagnostics = render_context.diagnostic_recorder();

        render_context.add_command_buffer_generation_task(move |render_device| {
            #[cfg(feature = "trace")]
            let _bloom_span = info_span!("bloom").entered();

            let mut command_encoder =
                render_device.create_command_encoder(&CommandEncoderDescriptor {
                    label: Some("bloom_command_encoder"),
                });
            command_encoder.push_debug_group("bloom");
            let time_span = diagnostics.time_span(&mut command_encoder, "bloom");

            // First downsample pass
            {
                let downsampling_first_bind_group = render_device.create_bind_group(
                    "bloom_downsampling_first_bind_group",
                    &downsampling_pipeline_res.bind_group_layout,
                    &BindGroupEntries::sequential((
                        // Read from main texture directly
                        view_texture,
                        &bind_groups.sampler,
                        uniforms.clone(),
                    )),
                );

                let view = &bloom_texture.view(0);
                let mut downsampling_first_pass =
                    command_encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("bloom_downsampling_first_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                downsampling_first_pass.set_pipeline(downsampling_first_pipeline);
                downsampling_first_pass.set_bind_group(
                    0,
                    &downsampling_first_bind_group,
                    &[uniform_index.index()],
                );
                downsampling_first_pass.draw(0..3, 0..1);
            }

            // Other downsample passes
            for mip in 1..bloom_texture.mip_count {
                let view = &bloom_texture.view(mip);
                let mut downsampling_pass =
                    command_encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("bloom_downsampling_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: Operations::default(),
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                downsampling_pass.set_pipeline(downsampling_pipeline);
                downsampling_pass.set_bind_group(
                    0,
                    &bind_groups.downsampling_bind_groups[mip as usize - 1],
                    &[uniform_index.index()],
                );
                downsampling_pass.draw(0..3, 0..1);
            }

            // Upsample passes except the final one
            for mip in (1..bloom_texture.mip_count).rev() {
                let view = &bloom_texture.view(mip - 1);
                let mut upsampling_pass =
                    command_encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("bloom_upsampling_pass"),
                        color_attachments: &[Some(RenderPassColorAttachment {
                            view,
                            depth_slice: None,
                            resolve_target: None,
                            ops: Operations {
                                load: LoadOp::Load,
                                store: StoreOp::Store,
                            },
                        })],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                upsampling_pass.set_pipeline(upsampling_pipeline);
                upsampling_pass.set_bind_group(
                    0,
                    &bind_groups.upsampling_bind_groups
                        [(bloom_texture.mip_count - mip - 1) as usize],
                    &[uniform_index.index()],
                );
                let blend = compute_blend_factor(
                    bloom_settings,
                    mip as f32,
                    (bloom_texture.mip_count - 1) as f32,
                );
                upsampling_pass.set_blend_constant(LinearRgba::gray(blend).into());
                upsampling_pass.draw(0..3, 0..1);
            }

            // Final upsample pass
            // This is very similar to the above upsampling passes with the only difference
            // being the pipeline (which itself is barely different) and the color attachment
            {
                let mut upsampling_final_pass =
                    command_encoder.begin_render_pass(&RenderPassDescriptor {
                        label: Some("bloom_upsampling_final_pass"),
                        color_attachments: &[Some(view_texture_unsampled)],
                        depth_stencil_attachment: None,
                        timestamp_writes: None,
                        occlusion_query_set: None,
                    });
                upsampling_final_pass.set_pipeline(upsampling_final_pipeline);
                upsampling_final_pass.set_bind_group(
                    0,
                    &bind_groups.upsampling_bind_groups[(bloom_texture.mip_count - 1) as usize],
                    &[uniform_index.index()],
                );
                if let Some(viewport) = camera.viewport.as_ref() {
                    upsampling_final_pass.set_viewport(
                        viewport.physical_position.x as f32,
                        viewport.physical_position.y as f32,
                        viewport.physical_size.x as f32,
                        viewport.physical_size.y as f32,
                        viewport.depth.start,
                        viewport.depth.end,
                    );
                }
                let blend =
                    compute_blend_factor(bloom_settings, 0.0, (bloom_texture.mip_count - 1) as f32);
                upsampling_final_pass.set_blend_constant(LinearRgba::gray(blend).into());
                upsampling_final_pass.draw(0..3, 0..1);
            }

            time_span.end(&mut command_encoder);
            command_encoder.pop_debug_group();
            command_encoder.finish()
        });

        Ok(())
    }
}

#[derive(Component)]
struct BloomTexture {
    // First mip is half the screen resolution, successive mips are half the previous
    #[cfg(any(
        not(feature = "webgl"),
        not(target_arch = "wasm32"),
        feature = "webgpu"
    ))]
    texture: CachedTexture,
    // WebGL does not support binding specific mip levels for sampling, fallback to separate textures instead
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    texture: Vec<CachedTexture>,
    mip_count: u32,
}

impl BloomTexture {
    #[cfg(any(
        not(feature = "webgl"),
        not(target_arch = "wasm32"),
        feature = "webgpu"
    ))]
    fn view(&self, base_mip_level: u32) -> TextureView {
        self.texture.texture.create_view(&TextureViewDescriptor {
            base_mip_level,
            mip_level_count: Some(1u32),
            ..Default::default()
        })
    }
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    fn view(&self, base_mip_level: u32) -> TextureView {
        self.texture[base_mip_level as usize]
            .texture
            .create_view(&TextureViewDescriptor {
                base_mip_level: 0,
                mip_level_count: Some(1u32),
                ..Default::default()
            })
    }
}

fn prepare_bloom_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera, &Bloom)>,
) {
    for (entity, camera, bloom) in &views {
        if let Some(viewport) = camera.physical_viewport_size {
            // How many times we can halve the resolution minus one so we don't go unnecessarily low
            let mip_count = bloom.max_mip_dimension.ilog2().max(2) - 1;
            let mip_height_ratio = if viewport.y != 0 {
                bloom.max_mip_dimension as f32 / viewport.y as f32
            } else {
                0.
            };

            let texture_descriptor = TextureDescriptor {
                label: Some("bloom_texture"),
                size: (viewport.as_vec2() * mip_height_ratio)
                    .round()
                    .as_uvec2()
                    .max(UVec2::ONE)
                    .to_extents(),
                mip_level_count: mip_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: BLOOM_TEXTURE_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            };

            #[cfg(any(
                not(feature = "webgl"),
                not(target_arch = "wasm32"),
                feature = "webgpu"
            ))]
            let texture = texture_cache.get(&render_device, texture_descriptor);
            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            let texture: Vec<CachedTexture> = (0..mip_count)
                .map(|mip| {
                    texture_cache.get(
                        &render_device,
                        TextureDescriptor {
                            size: Extent3d {
                                width: (texture_descriptor.size.width >> mip).max(1),
                                height: (texture_descriptor.size.height >> mip).max(1),
                                depth_or_array_layers: 1,
                            },
                            mip_level_count: 1,
                            ..texture_descriptor.clone()
                        },
                    )
                })
                .collect();

            commands
                .entity(entity)
                .insert(BloomTexture { texture, mip_count });
        }
    }
}

#[derive(Component)]
struct BloomBindGroups {
    downsampling_bind_groups: Box<[BindGroup]>,
    upsampling_bind_groups: Box<[BindGroup]>,
    sampler: Sampler,
}

fn prepare_bloom_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    downsampling_pipeline: Res<BloomDownsamplingPipeline>,
    upsampling_pipeline: Res<BloomUpsamplingPipeline>,
    views: Query<(Entity, &BloomTexture)>,
    uniforms: Res<ComponentUniforms<BloomUniforms>>,
) {
    let sampler = &downsampling_pipeline.sampler;

    for (entity, bloom_texture) in &views {
        let bind_group_count = bloom_texture.mip_count as usize - 1;

        let mut downsampling_bind_groups = Vec::with_capacity(bind_group_count);
        for mip in 1..bloom_texture.mip_count {
            downsampling_bind_groups.push(render_device.create_bind_group(
                "bloom_downsampling_bind_group",
                &downsampling_pipeline.bind_group_layout,
                &BindGroupEntries::sequential((
                    &bloom_texture.view(mip - 1),
                    sampler,
                    uniforms.binding().unwrap(),
                )),
            ));
        }

        let mut upsampling_bind_groups = Vec::with_capacity(bind_group_count);
        for mip in (0..bloom_texture.mip_count).rev() {
            upsampling_bind_groups.push(render_device.create_bind_group(
                "bloom_upsampling_bind_group",
                &upsampling_pipeline.bind_group_layout,
                &BindGroupEntries::sequential((
                    &bloom_texture.view(mip),
                    sampler,
                    uniforms.binding().unwrap(),
                )),
            ));
        }

        commands.entity(entity).insert(BloomBindGroups {
            downsampling_bind_groups: downsampling_bind_groups.into_boxed_slice(),
            upsampling_bind_groups: upsampling_bind_groups.into_boxed_slice(),
            sampler: sampler.clone(),
        });
    }
}

/// Calculates blend intensities of blur pyramid levels
/// during the upsampling + compositing stage.
///
/// The function assumes all pyramid levels are upsampled and
/// blended into higher frequency ones using this function to
/// calculate blend levels every time. The final (highest frequency)
/// pyramid level in not blended into anything therefore this function
/// is not applied to it. As a result, the *mip* parameter of 0 indicates
/// the second-highest frequency pyramid level (in our case that is the
/// 0th mip of the bloom texture with the original image being the
/// actual highest frequency level).
///
/// Parameters:
/// * `mip` - the index of the lower frequency pyramid level (0 - `max_mip`, where 0 indicates highest frequency mip but not the highest frequency image).
/// * `max_mip` - the index of the lowest frequency pyramid level.
///
/// This function can be visually previewed for all values of *mip* (normalized) with tweakable
/// [`Bloom`] parameters on [Desmos graphing calculator](https://www.desmos.com/calculator/ncc8xbhzzl).
fn compute_blend_factor(bloom: &Bloom, mip: f32, max_mip: f32) -> f32 {
    let mut lf_boost =
        (1.0 - ops::powf(
            1.0 - (mip / max_mip),
            1.0 / (1.0 - bloom.low_frequency_boost_curvature),
        )) * bloom.low_frequency_boost;
    let high_pass_lq = 1.0
        - (((mip / max_mip) - bloom.high_pass_frequency) / bloom.high_pass_frequency)
            .clamp(0.0, 1.0);
    lf_boost *= match bloom.composite_mode {
        BloomCompositeMode::EnergyConserving => 1.0 - bloom.intensity,
        BloomCompositeMode::Additive => 1.0,
    };

    (bloom.intensity + lf_boost) * high_pass_lq
}
