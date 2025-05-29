use crate::FullscreenShader;

use super::{Bloom, BLOOM_TEXTURE_FORMAT};
use bevy_asset::{load_embedded_asset, Handle};
use bevy_ecs::{
    prelude::{Component, Entity},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_math::{Vec2, Vec4};
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        *,
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
    /// Layout with a texture, a sampler, and uniforms
    pub bind_group_layout: BindGroupLayout,
    pub sampler: Sampler,
    /// The asset handle for the fullscreen vertex shader.
    pub fullscreen_shader: FullscreenShader,
    /// The fragment shader asset handle.
    pub fragment_shader: Handle<Shader>,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BloomDownsamplingPipelineKeys {
    prefilter: bool,
    first_downsample: bool,
    uniform_scale: bool,
}

/// The uniform struct extracted from [`Bloom`] attached to a Camera.
/// Will be available for use in the Bloom shader.
#[derive(Component, ShaderType, Clone)]
pub struct BloomUniforms {
    // Precomputed values used when thresholding, see https://catlikecoding.com/unity/tutorials/advanced-rendering/bloom/#3.4
    pub threshold_precomputations: Vec4,
    pub viewport: Vec4,
    pub scale: Vec2,
    pub aspect: f32,
}

impl FromWorld for BloomDownsamplingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Bind group layout
        let bind_group_layout = render_device.create_bind_group_layout(
            "bloom_downsampling_bind_group_layout_with_settings",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    // Input texture binding
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // Sampler binding
                    sampler(SamplerBindingType::Filtering),
                    // Downsampling settings binding
                    uniform_buffer::<BloomUniforms>(true),
                ),
            ),
        );

        // Sampler
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        BloomDownsamplingPipeline {
            bind_group_layout,
            sampler,
            fullscreen_shader: world.resource::<FullscreenShader>().clone(),
            fragment_shader: load_embedded_asset!(world, "bloom.wgsl"),
        }
    }
}

impl SpecializedRenderPipeline for BloomDownsamplingPipeline {
    type Key = BloomDownsamplingPipelineKeys;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let layout = vec![self.bind_group_layout.clone()];

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

        if key.uniform_scale {
            shader_defs.push("UNIFORM_SCALE".into());
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
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
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
            push_constant_ranges: Vec::new(),
            zero_initialize_workgroup_memory: false,
        }
    }
}

pub fn prepare_downsampling_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BloomDownsamplingPipeline>>,
    pipeline: Res<BloomDownsamplingPipeline>,
    views: Query<(Entity, &Bloom)>,
) {
    for (entity, bloom) in &views {
        let prefilter = bloom.prefilter.threshold > 0.0;

        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomDownsamplingPipelineKeys {
                prefilter,
                first_downsample: false,
                uniform_scale: bloom.scale == Vec2::ONE,
            },
        );

        let pipeline_first_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomDownsamplingPipelineKeys {
                prefilter,
                first_downsample: true,
                uniform_scale: bloom.scale == Vec2::ONE,
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
