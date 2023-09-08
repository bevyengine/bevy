use super::{
    downsampling_pipeline::BloomUniforms, BloomCompositeMode, BloomSettings, BLOOM_SHADER_HANDLE,
    BLOOM_TEXTURE_FORMAT,
};
use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use bevy_ecs::{
    prelude::{Component, Entity},
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{render_resource::*, renderer::RenderDevice, view::ViewTarget};

#[derive(Component)]
pub struct UpsamplingPipelineIds {
    pub id_main: CachedRenderPipelineId,
    pub id_final: CachedRenderPipelineId,
}

#[derive(Resource)]
pub struct BloomUpsamplingPipeline {
    pub bind_group_layout: BindGroupLayout,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BloomUpsamplingPipelineKeys {
    composite_mode: BloomCompositeMode,
    final_pipeline: bool,
}

impl FromWorld for BloomUpsamplingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("bloom_upsampling_bind_group_layout"),
                entries: &[
                    // Input texture
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
                    // BloomUniforms
                    BindGroupLayoutEntry {
                        binding: 2,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(BloomUniforms::min_size()),
                        },
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                ],
            });

        BloomUpsamplingPipeline { bind_group_layout }
    }
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
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: BLOOM_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "upsample".into(),
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
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
        }
    }
}

pub fn prepare_upsampling_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BloomUpsamplingPipeline>>,
    pipeline: Res<BloomUpsamplingPipeline>,
    views: Query<(Entity, &BloomSettings)>,
) {
    for (entity, settings) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomUpsamplingPipelineKeys {
                composite_mode: settings.composite_mode,
                final_pipeline: false,
            },
        );

        let pipeline_final_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            BloomUpsamplingPipelineKeys {
                composite_mode: settings.composite_mode,
                final_pipeline: true,
            },
        );

        commands.entity(entity).insert(UpsamplingPipelineIds {
            id_main: pipeline_id,
            id_final: pipeline_final_id,
        });
    }
}
