use crate::FullscreenShader;

use super::{
    downsampling_pipeline::BloomUniforms, Bloom, BloomCompositeMode, BLOOM_TEXTURE_FORMAT,
};
use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_ecs::{
    prelude::{Component, Entity},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        *,
    },
    renderer::RenderDevice,
    view::ViewTarget,
};
use bevy_shader::Shader;
use bevy_utils::default;

#[derive(Component)]
pub struct UpsamplingPipelineIds {
    pub id_main: CachedRenderPipelineId,
    pub id_final: CachedRenderPipelineId,
}

#[derive(Resource)]
pub struct BloomUpsamplingPipeline {
    pub bind_group_layout: BindGroupLayout,
    /// The asset handle for the fullscreen vertex shader.
    pub fullscreen_shader: FullscreenShader,
    /// The fragment shader asset handle.
    pub fragment_shader: Handle<Shader>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BloomUpsamplingPipelineKeys {
    composite_mode: BloomCompositeMode,
    final_pipeline: bool,
}

pub fn init_bloom_upscaling_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let bind_group_layout = render_device.create_bind_group_layout(
        "bloom_upsampling_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // Input texture
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Sampler
                sampler(SamplerBindingType::Filtering),
                // BloomUniforms
                uniform_buffer::<BloomUniforms>(true),
            ),
        ),
    );

    commands.insert_resource(BloomUpsamplingPipeline {
        bind_group_layout,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "bloom.wgsl"),
    });
}

impl SpecializedRenderPipeline for BloomUpsamplingPipeline {
    type Key = BloomUpsamplingPipelineKeys;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let texture_format = if key.final_pipeline {
            ViewTarget::TEXTURE_FORMAT_HDR
        } else {
            BLOOM_TEXTURE_FORMAT
        };

        let color_blend = match key.composite_mode {
            BloomCompositeMode::EnergyConserving => {
                // At the time of developing this we decided to blend our
                // blur pyramid levels using native WGPU render pass blend
                // constants. They are set in the bloom node's run function.
                // This seemed like a good approach at the time which allowed
                // us to perform complex calculations for blend levels on the CPU,
                // however, we missed the fact that this prevented us from using
                // textures to customize bloom appearance on individual parts
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
            }
            BloomCompositeMode::Additive => BlendComponent {
                src_factor: BlendFactor::Constant,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        };

        RenderPipelineDescriptor {
            label: Some("bloom_upsampling_pipeline".into()),
            layout: vec![self.bind_group_layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                entry_point: Some("upsample".into()),
                targets: vec![Some(ColorTargetState {
                    format: texture_format,
                    blend: Some(BlendState {
                        color: color_blend,
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

pub fn prepare_upsampling_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BloomUpsamplingPipeline>>,
    pipeline: Res<BloomUpsamplingPipeline>,
    views: Query<(Entity, &Bloom)>,
) {
    for (entity, bloom) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomUpsamplingPipelineKeys {
                composite_mode: bloom.composite_mode,
                final_pipeline: false,
            },
        );

        let pipeline_final_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomUpsamplingPipelineKeys {
                composite_mode: bloom.composite_mode,
                final_pipeline: true,
            },
        );

        commands.entity(entity).insert(UpsamplingPipelineIds {
            id_main: pipeline_id,
            id_final: pipeline_final_id,
        });
    }
}
