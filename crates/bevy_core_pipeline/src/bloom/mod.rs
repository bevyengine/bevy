mod downsampling_pipeline;
mod settings;
mod upsampling_pipeline;

use std::ops::Deref;

use bevy_color::{Gray, LinearRgba};
pub use settings::{Bloom, BloomCompositeMode, BloomPrefilter};

use crate::{
    core_2d::graph::{Core2d, Node2d},
    core_3d::graph::{Core3d, Node3d},
};
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, Handle};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_math::{ops, UVec2};
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::{
        ComponentUniforms, DynamicUniformIndex, ExtractComponentPlugin, UniformComponentPlugin,
    },
    frame_graph::{
        BindingResourceRef, ColorAttachmentDrawing, EncoderCommandBuilder, FrameGraph,
        FrameGraphTexture, GraphResourceNodeHandle, PassBuilder, ResourceMeta, TextureInfo,
        TextureViewDrawing, TextureViewInfo,
    },
    render_graph::{NodeRunError, RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::*,
    view::ViewTarget,
    Render, RenderApp, RenderSet,
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

const BLOOM_SHADER_HANDLE: Handle<Shader> = weak_handle!("c9190ddc-573b-4472-8b21-573cab502b73");

const BLOOM_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rg11b10Ufloat;

pub struct BloomPlugin;

impl Plugin for BloomPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, BLOOM_SHADER_HANDLE, "bloom.wgsl", Shader::from_wgsl);

        app.register_type::<Bloom>();
        app.register_type::<BloomPrefilter>();
        app.register_type::<BloomCompositeMode>();
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
                Render,
                (
                    prepare_downsampling_pipeline.in_set(RenderSet::Prepare),
                    prepare_upsampling_pipeline.in_set(RenderSet::Prepare),
                    prepare_bloom_textures.in_set(RenderSet::PrepareResources),
                ),
            )
            // Add bloom to the 3d render graph
            .add_render_graph_node::<ViewNodeRunner<BloomNode>>(Core3d, Node3d::Bloom)
            .add_render_graph_edges(
                Core3d,
                (Node3d::EndMainPass, Node3d::Bloom, Node3d::Tonemapping),
            )
            // Add bloom to the 2d render graph
            .add_render_graph_node::<ViewNodeRunner<BloomNode>>(Core2d, Node2d::Bloom)
            .add_render_graph_edges(
                Core2d,
                (Node2d::EndMainPass, Node2d::Bloom, Node2d::Tonemapping),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<BloomDownsamplingPipeline>()
            .init_resource::<BloomUpsamplingPipeline>();
    }
}

#[derive(Default)]
struct BloomNode;
impl ViewNode for BloomNode {
    type ViewQuery = (
        &'static ExtractedCamera,
        &'static ViewTarget,
        &'static BloomTexture,
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
        graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (
            camera,
            view_target,
            bloom_texture,
            uniform_index,
            bloom_settings,
            upsampling_pipeline_ids,
            downsampling_pipeline_ids,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        if bloom_settings.intensity == 0.0 {
            return Ok(());
        }

        let entity = graph.view_entity();
        let downsampling_pipeline_res = world.resource::<BloomDownsamplingPipeline>();
        let upsampling_pipeline_res = world.resource::<BloomUpsamplingPipeline>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let uniforms = world.resource::<ComponentUniforms<BloomUniforms>>();

        let (Some(_), Some(_), Some(_), Some(_), Some(_)) = (
            uniforms.binding(),
            pipeline_cache.get_render_pipeline(downsampling_pipeline_ids.first),
            pipeline_cache.get_render_pipeline(downsampling_pipeline_ids.main),
            pipeline_cache.get_render_pipeline(upsampling_pipeline_ids.id_main),
            pipeline_cache.get_render_pipeline(upsampling_pipeline_ids.id_final),
        ) else {
            return Ok(());
        };

        let view_texture: GraphResourceNodeHandle<FrameGraphTexture> =
            frame_graph.get(view_target.get_main_texture_key())?;

        let mut pass_builder = PassBuilder::new(frame_graph.create_pass_node_bulder("bloom"));

        let color_attachment =
            view_target.get_unsampled_attachment(pass_builder.pass_node_builder())?;

        pass_builder.push_debug_group("bloom");

        // First downsample pass
        {
            let bloom_texture_write =
                pass_builder.write_material(&bloom_texture.get_resource_meta(entity, 0));

            let downsampling_first_bind_group = pass_builder
                .create_bind_group_builder(
                    Some("bloom_downsampling_first_bind_group".into()),
                    downsampling_pipeline_res.bind_group_layout.clone(),
                )
                .push_bind_group_entry(&view_texture)
                .push_bind_group_entry(&downsampling_pipeline_res.sampler_info)
                .push_bind_group_entry(uniforms.deref())
                .build();

            pass_builder
                .create_render_pass_builder()
                .set_pass_name("bloom_downsampling_first_pass")
                .add_color_attachment(ColorAttachmentDrawing {
                    view: TextureViewDrawing {
                        texture: bloom_texture_write,
                        desc: bloom_texture.get_texture_view_info(0),
                    },
                    resolve_target: None,
                    ops: Operations::default(),
                })
                .set_render_pipeline(downsampling_pipeline_ids.first)
                .set_bind_group(0, downsampling_first_bind_group, &[uniform_index.index()])
                .draw(0..3, 0..1);
        }

        // Other downsample passes
        for mip in 1..bloom_texture.mip_count {
            let bind_group_mip = mip - 1;

            let bloom_texture_write =
                pass_builder.write_material(&bloom_texture.get_resource_meta(entity, mip));

            let bind_group_bloom_texture_read = pass_builder
                .read_material(&bloom_texture.get_resource_meta(entity, bind_group_mip));

            let downsampling_bind_group = pass_builder
                .create_bind_group_builder(
                    Some("bloom_downsampling_bind_group".into()),
                    downsampling_pipeline_res.bind_group_layout.clone(),
                )
                .push_bind_resource_ref(BindingResourceRef::TextureView {
                    texture: bind_group_bloom_texture_read.clone(),
                    texture_view_info: bloom_texture.get_texture_view_info(bind_group_mip),
                })
                .push_bind_group_entry(&downsampling_pipeline_res.sampler_info)
                .push_bind_group_entry(uniforms.deref())
                .build();

            pass_builder
                .create_render_pass_builder()
                .set_pass_name("bloom_downsampling_pass")
                .add_color_attachment(ColorAttachmentDrawing {
                    view: TextureViewDrawing {
                        texture: bloom_texture_write,
                        desc: bloom_texture.get_texture_view_info(mip),
                    },
                    resolve_target: None,
                    ops: Operations::default(),
                })
                .set_render_pipeline(downsampling_pipeline_ids.main)
                .set_bind_group(0, downsampling_bind_group, &[uniform_index.index()])
                .draw(0..3, 0..1);
        }

        // Upsample passes except the final one
        for mip in (1..bloom_texture.mip_count).rev() {
            let bind_group_mip = mip;

            let bloom_texture_write =
                pass_builder.write_material(&bloom_texture.get_resource_meta(entity, mip - 1));

            let bind_group_bloom_texture_read = pass_builder
                .read_material(&bloom_texture.get_resource_meta(entity, bind_group_mip));

            let upsampling_bind_group = pass_builder
                .create_bind_group_builder(
                    Some("bloom_upsampling_bind_group".into()),
                    upsampling_pipeline_res.bind_group_layout.clone(),
                )
                .push_bind_resource_ref(BindingResourceRef::TextureView {
                    texture: bind_group_bloom_texture_read.clone(),
                    texture_view_info: bloom_texture.get_texture_view_info(bind_group_mip),
                })
                .push_bind_group_entry(&downsampling_pipeline_res.sampler_info)
                .push_bind_group_entry(uniforms.deref())
                .build();

            let blend = compute_blend_factor(
                bloom_settings,
                mip as f32,
                (bloom_texture.mip_count - 1) as f32,
            );

            pass_builder
                .create_render_pass_builder()
                .set_pass_name("bloom_upsampling_pass")
                .add_color_attachment(ColorAttachmentDrawing {
                    view: TextureViewDrawing {
                        texture: bloom_texture_write,
                        desc: bloom_texture.get_texture_view_info(mip - 1),
                    },
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Load,
                        store: StoreOp::Store,
                    },
                })
                .set_render_pipeline(upsampling_pipeline_ids.id_main)
                .set_bind_group(0, upsampling_bind_group, &[uniform_index.index()])
                .set_blend_constant(LinearRgba::gray(blend).into())
                .draw(0..3, 0..1);
        }

        // Final upsample pass
        // This is very similar to the above upsampling passes with the only difference
        // being the pipeline (which itself is barely different) and the color attachment
        {
            let mip = 0;

            let bloom_texture_read =
                pass_builder.read_material(&bloom_texture.get_resource_meta(entity, mip));

            let upsampling_bind_group = pass_builder
                .create_bind_group_builder(
                    Some("bloom_upsampling_bind_group".into()),
                    upsampling_pipeline_res.bind_group_layout.clone(),
                )
                .push_bind_resource_ref(BindingResourceRef::TextureView {
                    texture: bloom_texture_read.clone(),
                    texture_view_info: bloom_texture.get_texture_view_info(mip),
                })
                .push_bind_group_entry(&downsampling_pipeline_res.sampler_info)
                .push_bind_group_entry(uniforms.deref())
                .build();

            let blend =
                compute_blend_factor(bloom_settings, 0.0, (bloom_texture.mip_count - 1) as f32);

            pass_builder
                .create_render_pass_builder()
                .set_pass_name("bloom_upsampling_final_pass")
                .add_color_attachment(color_attachment)
                .set_render_pipeline(upsampling_pipeline_ids.id_final)
                .set_bind_group(0, upsampling_bind_group, &[uniform_index.index()])
                .set_blend_constant(LinearRgba::gray(blend).into())
                .set_camera_viewport(camera.viewport.clone())
                .draw(0..3, 0..1);
        }

        pass_builder.pop_debug_group();

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
    texture_info: TextureInfo,
    // WebGL does not support binding specific mip levels for sampling, fallback to separate textures instead
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    texture_info: Vec<TextureInfo>,
    mip_count: u32,
}

impl BloomTexture {
    const BLOOM_TEXTURE_KEY: &str = "bloom_texture";

    #[cfg(any(
        not(feature = "webgl"),
        not(target_arch = "wasm32"),
        feature = "webgpu"
    ))]
    pub fn get_texture_view_info(&self, base_mip_level: u32) -> TextureViewInfo {
        TextureViewInfo {
            base_mip_level,
            mip_level_count: Some(1u32),
            ..Default::default()
        }
    }
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    pub fn get_texture_view_info(&self, _base_mip_level: u32) -> TextureViewInfo {
        TextureViewInfo {
            base_mip_level: 0,
            mip_level_count: Some(1u32),
            ..Default::default()
        }
    }

    #[cfg(any(
        not(feature = "webgl"),
        not(target_arch = "wasm32"),
        feature = "webgpu"
    ))]
    pub fn get_resource_meta(
        &self,
        entity: Entity,
        _base_mip_level: u32,
    ) -> ResourceMeta<FrameGraphTexture> {
        let key = format!("{}_{}", Self::BLOOM_TEXTURE_KEY, entity);
        ResourceMeta {
            key,
            desc: self.texture_info.clone(),
        }
    }
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    pub fn get_resource_meta(&self, base_mip_level: u32) -> ResourceMeta<FrameGraphTexture> {
        let key = format!("{}_{}", Self::BLOOM_TEXTURE_KEY, base_mip_level);
        ResourceMeta {
            key,
            desc: self.texture_info[base_mip_level as usize].clone(),
        }
    }
}

fn prepare_bloom_textures(
    mut commands: Commands,
    views: Query<(Entity, &ExtractedCamera, &Bloom)>,
) {
    for (entity, camera, bloom) in &views {
        if let Some(UVec2 {
            x: width,
            y: height,
        }) = camera.physical_viewport_size
        {
            // How many times we can halve the resolution minus one so we don't go unnecessarily low
            let mip_count = bloom.max_mip_dimension.ilog2().max(2) - 1;
            let mip_height_ratio = if height != 0 {
                bloom.max_mip_dimension as f32 / height as f32
            } else {
                0.
            };

            let texture_info = TextureInfo {
                label: Some("bloom_texture".into()),
                size: Extent3d {
                    width: ((width as f32 * mip_height_ratio).round() as u32).max(1),
                    height: ((height as f32 * mip_height_ratio).round() as u32).max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: mip_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: BLOOM_TEXTURE_FORMAT,
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                view_formats: vec![],
            };

            #[cfg(any(
                not(feature = "webgl"),
                not(target_arch = "wasm32"),
                feature = "webgpu"
            ))]
            let tmp_texture_info = texture_info;
            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            let tmp_texture_info: Vec<CachedTexture> = (0..mip_count)
                .map(|mip| {
                    let temp_texture_info = TextureInfo {
                        size: Extent3d {
                            width: (texture_descriptor.size.width >> mip).max(1),
                            height: (texture_descriptor.size.height >> mip).max(1),
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        ..texture_info.clone()
                    };

                    ResourceMeta {
                        key_meta: BloomTexture::BLOOM_TEXTURE_KEY.to_string(),
                        desc: texture_info,
                    };
                })
                .collect();

            commands.entity(entity).insert(BloomTexture {
                texture_info: tmp_texture_info,
                mip_count,
            });
        }
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
