use super::{settings::BloomUniforms, Bloom, BloomCompositeMode, BLOOM_TEXTURE_FORMAT};
use crate::lens_dirt::{create_lens_dirt_bind_group_layout, LensDirt};

use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_core_pipeline::FullscreenShader;
use bevy_ecs::{
    prelude::{Component, Entity},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_render::{
    render_resource::{
        binding_types::{sampler, storage_buffer_read_only_sized, texture_2d, uniform_buffer},
        *,
    },
    view::ExtractedView,
};
use bevy_shader::Shader;
use bevy_utils::default;
use core::num::NonZero;

#[derive(Component)]
pub struct UpsamplingPipelineIds {
    pub id_main: CachedRenderPipelineId,
    pub id_final: CachedRenderPipelineId,
    pub id_final_dirt: Option<CachedRenderPipelineId>,
}

#[derive(Resource)]
pub struct BloomUpsamplingPipeline {
    pub bind_group_layout: BindGroupLayoutDescriptor,
    pub lens_dirt_bind_group_layout: BindGroupLayoutDescriptor,
    /// The asset handle for the fullscreen vertex shader.
    pub fullscreen_shader: FullscreenShader,
    /// The fragment shader asset handle.
    pub fragment_shader: Handle<Shader>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BloomUpsamplingPipelineKeys {
    composite_mode: BloomCompositeMode,
    target_format: TextureFormat,
    lens_dirt: bool,
}

pub fn init_bloom_upscaling_pipeline(
    mut commands: Commands,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "bloom_upsampling_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<BloomUniforms>(true),
                storage_buffer_read_only_sized(false, NonZero::<u64>::new(4)),
            ),
        ),
    );

    let lens_dirt_bind_group_layout = create_lens_dirt_bind_group_layout();

    commands.insert_resource(BloomUpsamplingPipeline {
        bind_group_layout,
        lens_dirt_bind_group_layout,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "bloom.wgsl"),
    });
}

impl SpecializedRenderPipeline for BloomUpsamplingPipeline {
    type Key = BloomUpsamplingPipelineKeys;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let color_blend = match key.composite_mode {
            BloomCompositeMode::EnergyConserving => BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            BloomCompositeMode::Additive => BlendComponent {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
        };

        let (layout, shader_defs) = if key.lens_dirt {
            (
                vec![
                    self.bind_group_layout.clone(),
                    self.lens_dirt_bind_group_layout.clone(),
                ],
                vec!["LENS_DIRT".into()],
            )
        } else {
            (vec![self.bind_group_layout.clone()], vec![])
        };

        RenderPipelineDescriptor {
            label: Some("bloom_upsampling_pipeline".into()),
            layout,
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                entry_point: Some("upsample".into()),
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
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
    views: Query<(&ExtractedView, Entity, &Bloom, Option<&LensDirt>)>,
) {
    for (view, entity, bloom, maybe_lens_dirt) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomUpsamplingPipelineKeys {
                composite_mode: bloom.composite_mode,
                target_format: BLOOM_TEXTURE_FORMAT,
                lens_dirt: false,
            },
        );

        let pipeline_final_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomUpsamplingPipelineKeys {
                composite_mode: bloom.composite_mode,
                target_format: view.target_format,
                lens_dirt: false,
            },
        );

        let pipeline_final_dirt_id = maybe_lens_dirt.is_some().then(|| {
            pipelines.specialize(
                &pipeline_cache,
                &pipeline,
                BloomUpsamplingPipelineKeys {
                    composite_mode: bloom.composite_mode,
                    target_format: view.target_format,
                    lens_dirt: true,
                },
            )
        });

        commands.entity(entity).insert(UpsamplingPipelineIds {
            id_main: pipeline_id,
            id_final: pipeline_final_id,
            id_final_dirt: pipeline_final_dirt_id,
        });
    }
}
