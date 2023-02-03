use super::{BloomSettings, BLOOM_SHADER_HANDLE, BLOOM_TEXTURE_FORMAT};
use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy_ecs::{
    prelude::{Component, Entity},
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_math::Vec4;
use bevy_render::{
    render_resource::{
        BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
        BufferBindingType, CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState,
        MultisampleState, PipelineCache, PrimitiveState, RenderPipelineDescriptor,
        SamplerBindingType, Shader, ShaderStages, ShaderType, SpecializedRenderPipeline,
        SpecializedRenderPipelines, TextureSampleType, TextureViewDimension,
    },
    renderer::RenderDevice,
};

#[derive(Component)]
pub struct BloomDownsamplingPipelineIds {
    pub main: CachedRenderPipelineId,
    pub first: CachedRenderPipelineId,
}

#[derive(Resource)]
pub struct BloomDownsamplingPipeline {
    /// Layout with just a texture and a sampler
    pub bind_group_layout: BindGroupLayout,
    /// Layout with a texture, a sampler, and downsampling settings
    pub extended_bind_group_layout: BindGroupLayout,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BloomDownsamplingPipelineKeys {
    prefilter: bool,
    first_downsample: bool,
}

/// The uniform struct extracted from [`BloomSettings`] attached to a [`Camera`].
/// Will be available for use in the Bloom shader in the first downsample pass.
#[derive(Component, ShaderType, Clone)]
pub struct BloomDownsamplingUniforms {
    // Precomputed values used when thresholding, see https://catlikecoding.com/unity/tutorials/advanced-rendering/bloom/#3.4
    pub threshold_precomputations: Vec4,
    pub viewport: Vec4,
}

impl FromWorld for BloomDownsamplingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Input texture
        let texture = BindGroupLayoutEntry {
            binding: 0,
            ty: BindingType::Texture {
                sample_type: TextureSampleType::Float { filterable: true },
                view_dimension: TextureViewDimension::D2,
                multisampled: false,
            },
            visibility: ShaderStages::FRAGMENT,
            count: None,
        };

        // Sampler
        let sampler = BindGroupLayoutEntry {
            binding: 1,
            ty: BindingType::Sampler(SamplerBindingType::Filtering),
            visibility: ShaderStages::FRAGMENT,
            count: None,
        };

        // Downsampling settings
        let settings = BindGroupLayoutEntry {
            binding: 2,
            ty: BindingType::Buffer {
                ty: BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: Some(BloomDownsamplingUniforms::min_size()),
            },
            visibility: ShaderStages::FRAGMENT,
            count: None,
        };

        // Bind group layouts
        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bloom_downsampling_bind_group_layout"),
                entries: &[texture, sampler],
            });
        let extended_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bloom_downsampling_bind_group_layout_with_settings"),
                entries: &[texture, sampler, settings],
            });

        BloomDownsamplingPipeline {
            bind_group_layout,
            extended_bind_group_layout,
        }
    }
}

impl SpecializedRenderPipeline for BloomDownsamplingPipeline {
    type Key = BloomDownsamplingPipelineKeys;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let layout = if key.first_downsample {
            Some(vec![self.extended_bind_group_layout.clone()])
        } else {
            Some(vec![self.bind_group_layout.clone()])
        };

        let entry_point = if key.first_downsample {
            "downsample_first".into()
        } else {
            "downsample".into()
        };

        let mut shader_defs = vec![];

        if key.first_downsample {
            shader_defs.push("FIRST_DOWNSAMPLE".into());
        }

        if key.prefilter {
            shader_defs.push("USE_THRESHOLD".into());
        }

        RenderPipelineDescriptor {
            label: Some(
                if key.first_downsample {
                    "bloom_downsampling_pipeline_first"
                } else {
                    "bloom_downsampling_pipeline"
                }
                .into(),
            ),
            layout,
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point,
                targets: vec![Some(ColorTargetState {
                    format: BLOOM_TEXTURE_FORMAT,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
        }
    }
}

pub fn prepare_downsampling_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BloomDownsamplingPipeline>>,
    pipeline: Res<BloomDownsamplingPipeline>,
    views: Query<(Entity, &BloomSettings)>,
) {
    for (entity, settings) in &views {
        let prefilter = settings.prefilter_settings.threshold > 0.0;

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomDownsamplingPipelineKeys {
                prefilter,
                first_downsample: false,
            },
        );

        let pipeline_first_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomDownsamplingPipelineKeys {
                prefilter,
                first_downsample: true,
            },
        );

        commands
            .entity(entity)
            .insert(BloomDownsamplingPipelineIds {
                first: pipeline_first_id,
                main: pipeline_id,
            });
    }
}
