mod downsampling_pipeline;
mod settings;
mod upsampling_pipeline;

pub use downsampling_pipeline::BloomDownsamplingMipmapper;
pub use settings::{BloomCompositeMode, BloomPrefilterSettings, BloomSettings};

use crate::{
    core_2d::{self, CORE_2D},
    core_3d::{self, CORE_3D},
    mipmap_generator::{
        self, MipmapBindGroups, MipmapPipeline, MipmapPipelineIds, MipmapPlugin, MipmappedTexture,
    },
};
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Handle};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_math::{uvec2, UVec2};
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponentPlugin, UniformComponentPlugin,
    },
    prelude::Color,
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::*,
    renderer::{RenderContext, RenderDevice},
    texture::TextureCache,
    view::ViewTarget,
    Render, RenderApp, RenderSet,
};
use downsampling_pipeline::{prepare_downsampling_pipeline, BloomUniforms};
use upsampling_pipeline::{
    prepare_upsampling_pipeline, BloomUpsamplingPipeline, UpsamplingPipelineIds,
};

const BLOOM_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(929599476923908);

pub const BLOOM_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rg11b10Float;

// Maximum size of each dimension for the largest mipchain texture used in downscaling/upscaling.
// 512 behaves well with the UV offset of 0.004 used in bloom.wgsl
const MAX_MIP_DIMENSION: u32 = 512;

pub struct BloomPlugin;

impl Plugin for BloomPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, BLOOM_SHADER_HANDLE, "bloom.wgsl", Shader::from_wgsl);

        app.register_type::<BloomSettings>();
        app.register_type::<BloomPrefilterSettings>();
        app.register_type::<BloomCompositeMode>();
        app.add_plugins((
            ExtractComponentPlugin::<BloomSettings>::default(),
            UniformComponentPlugin::<BloomUniforms>::default(),
            MipmapPlugin::<BloomDownsamplingMipmapper>::default(),
        ));

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<BloomUpsamplingPipeline>>()
            .add_systems(
                Render,
                (
                    prepare_downsampling_pipeline.in_set(RenderSet::Prepare),
                    prepare_upsampling_pipeline.in_set(RenderSet::Prepare),
                    prepare_bloom_textures.in_set(RenderSet::PrepareResources),
                    prepare_bloom_bind_groups.in_set(RenderSet::PrepareBindGroups),
                ),
            )
            // Add bloom to the 3d render graph
            .add_render_graph_node::<ViewNodeRunner<BloomNode>>(
                CORE_3D,
                core_3d::graph::node::BLOOM,
            )
            .add_render_graph_edges(
                CORE_3D,
                &[
                    core_3d::graph::node::END_MAIN_PASS,
                    core_3d::graph::node::BLOOM,
                    core_3d::graph::node::TONEMAPPING,
                ],
            )
            // Add bloom to the 2d render graph
            .add_render_graph_node::<ViewNodeRunner<BloomNode>>(
                CORE_2D,
                core_2d::graph::node::BLOOM,
            )
            .add_render_graph_edges(
                CORE_2D,
                &[
                    core_2d::graph::node::MAIN_PASS,
                    core_2d::graph::node::BLOOM,
                    core_2d::graph::node::TONEMAPPING,
                ],
            );
    }

    fn finish(&self, app: &mut App) {
        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app.init_resource::<BloomUpsamplingPipeline>();
    }
}

#[derive(Default)]
struct BloomNode;
impl ViewNode for BloomNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static MipmappedTexture<BloomDownsamplingMipmapper>,
        &'static BloomBindGroups,
        &'static DynamicUniformIndex<BloomUniforms>,
        &'static BloomSettings,
        &'static UpsamplingPipelineIds,
        &'static MipmapPipelineIds<BloomDownsamplingMipmapper>,
    );

    // Atypically for a post-processing effect, we do not need to
    // use a secondary texture normally provided by view_target.post_process_write(),
    // instead we write into our own bloom texture and then directly back onto main.
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            camera,
            view_target,
            bloom_texture,
            bind_groups,
            uniform_index,
            bloom_settings,
            upsampling_pipeline_ids,
            downsampling_pipeline_ids,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let downsampling_pipeline_res =
            world.resource::<MipmapPipeline<BloomDownsamplingMipmapper>>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let uniforms = world.resource::<ComponentUniforms<BloomUniforms>>();

        let (Some(uniforms), Some(upsampling_pipeline), Some(upsampling_final_pipeline)) = (
            uniforms.binding(),
            pipeline_cache.get_render_pipeline(upsampling_pipeline_ids.id_main),
            pipeline_cache.get_render_pipeline(upsampling_pipeline_ids.id_final),
        ) else {
            return Ok(());
        };

        render_context.command_encoder().push_debug_group("bloom");

        // Downsample passes
        if !mipmap_generator::generate_mipmaps(
            render_context,
            pipeline_cache,
            downsampling_pipeline_res,
            downsampling_pipeline_ids,
            &bind_groups.downsampling_bind_groups,
            bloom_texture,
            view_target.main_texture_view(),
            &[BindGroupEntry {
                binding: 2,
                resource: uniforms.clone(),
            }],
            &[uniform_index.index()],
        ) {
            return Ok(());
        }

        // Upsample passes except the final one
        for mip in (1..bloom_texture.mip_count).rev() {
            let view = &bloom_texture.view(mip - 1);
            let mut upsampling_pass =
                render_context.begin_tracked_render_pass(RenderPassDescriptor {
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
                });
            upsampling_pass.set_render_pipeline(upsampling_pipeline);
            upsampling_pass.set_bind_group(
                0,
                &bind_groups.upsampling_bind_groups[(bloom_texture.mip_count - mip - 1) as usize],
                &[uniform_index.index()],
            );
            let blend = compute_blend_factor(
                bloom_settings,
                mip as f32,
                (bloom_texture.mip_count - 1) as f32,
            );
            upsampling_pass.set_blend_constant(Color::rgb_linear(blend, blend, blend));
            upsampling_pass.draw(0..3, 0..1);
        }

        // Final upsample pass
        // This is very similar to the above upsampling passes with the only difference
        // being the pipeline (which itself is barely different) and the color attachment
        {
            let mut upsampling_final_pass =
                render_context.begin_tracked_render_pass(RenderPassDescriptor {
                    label: Some("bloom_upsampling_final_pass"),
                    color_attachments: &[Some(view_target.get_unsampled_color_attachment(
                        Operations {
                            load: LoadOp::Load,
                            store: true,
                        },
                    ))],
                    depth_stencil_attachment: None,
                });
            upsampling_final_pass.set_render_pipeline(upsampling_final_pipeline);
            upsampling_final_pass.set_bind_group(
                0,
                &bind_groups.upsampling_bind_groups[(bloom_texture.mip_count - 1) as usize],
                &[uniform_index.index()],
            );
            if let Some(viewport) = camera.viewport.as_ref() {
                upsampling_final_pass.set_camera_viewport(viewport);
            }
            let blend =
                compute_blend_factor(bloom_settings, 0.0, (bloom_texture.mip_count - 1) as f32);
            upsampling_final_pass.set_blend_constant(Color::rgb_linear(blend, blend, blend));
            upsampling_final_pass.draw(0..3, 0..1);
        }

        render_context.command_encoder().pop_debug_group();

        Ok(())
    }
}

fn prepare_bloom_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera), With<BloomSettings>>,
) {
    for (entity, camera) in &views {
        if let Some(UVec2 {
            x: width,
            y: height,
        }) = camera.physical_viewport_size
        {
            let aspect_ratio = width as f32 / height as f32;
            let size = if aspect_ratio >= 1.0 {
                uvec2(
                    MAX_MIP_DIMENSION,
                    (MAX_MIP_DIMENSION as f32 / aspect_ratio).round() as u32,
                )
            } else {
                uvec2(
                    (MAX_MIP_DIMENSION as f32 * aspect_ratio).round() as u32,
                    MAX_MIP_DIMENSION,
                )
            };

            let texture: MipmappedTexture<BloomDownsamplingMipmapper> =
                MipmappedTexture::new(&render_device, &mut texture_cache, size);
            commands.entity(entity).insert(texture);
        }
    }
}

#[derive(Component)]
struct BloomBindGroups {
    downsampling_bind_groups: MipmapBindGroups<BloomDownsamplingMipmapper>,
    upsampling_bind_groups: Box<[BindGroup]>,
}

fn prepare_bloom_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    downsampling_pipeline: Res<MipmapPipeline<BloomDownsamplingMipmapper>>,
    upsampling_pipeline: Res<BloomUpsamplingPipeline>,
    views: Query<(Entity, &MipmappedTexture<BloomDownsamplingMipmapper>)>,
    uniforms: Res<ComponentUniforms<BloomUniforms>>,
) {
    let sampler = &downsampling_pipeline.sampler;

    for (entity, bloom_texture) in &views {
        let bind_group_count = bloom_texture.mip_count as usize - 1;

        let downsampling_bind_groups = MipmapBindGroups::new(
            &render_device,
            &downsampling_pipeline,
            bloom_texture,
            &[BindGroupEntry {
                binding: 2,
                resource: uniforms.binding().unwrap(),
            }],
        );

        let mut upsampling_bind_groups = Vec::with_capacity(bind_group_count);
        for mip in (0..bloom_texture.mip_count).rev() {
            upsampling_bind_groups.push(render_device.create_bind_group(&BindGroupDescriptor {
                label: Some("bloom_upsampling_bind_group"),
                layout: &upsampling_pipeline.bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(&bloom_texture.view(mip)),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Sampler(sampler),
                    },
                    BindGroupEntry {
                        binding: 2,
                        resource: uniforms.binding().unwrap(),
                    },
                ],
            }));
        }

        commands.entity(entity).insert(BloomBindGroups {
            downsampling_bind_groups,
            upsampling_bind_groups: upsampling_bind_groups.into_boxed_slice(),
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
/// * *mip* - the index of the lower frequency pyramid level (0 - max_mip, where 0 indicates highest frequency mip but not the highest frequency image).
/// * *max_mip* - the index of the lowest frequency pyramid level.
///
/// This function can be visually previewed for all values of *mip* (normalized) with tweakable
/// [`BloomSettings`] parameters on [Desmos graphing calculator](https://www.desmos.com/calculator/ncc8xbhzzl).
#[allow(clippy::doc_markdown)]
fn compute_blend_factor(bloom_settings: &BloomSettings, mip: f32, max_mip: f32) -> f32 {
    let mut lf_boost = (1.0
        - (1.0 - (mip / max_mip)).powf(1.0 / (1.0 - bloom_settings.low_frequency_boost_curvature)))
        * bloom_settings.low_frequency_boost;
    let high_pass_lq = 1.0
        - (((mip / max_mip) - bloom_settings.high_pass_frequency)
            / bloom_settings.high_pass_frequency)
            .clamp(0.0, 1.0);
    lf_boost *= match bloom_settings.composite_mode {
        BloomCompositeMode::EnergyConserving => 1.0 - bloom_settings.intensity,
        BloomCompositeMode::Additive => 1.0,
    };

    (bloom_settings.intensity + lf_boost) * high_pass_lq
}
