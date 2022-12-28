use bevy_ecs::{
    prelude::{Component, Entity},
    query::With,
    system::{Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_render::{
    render_resource::{
        BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
        BlendComponent, BlendFactor, BlendOperation, BlendState, BufferBindingType,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState, MultisampleState,
        PipelineCache, PrimitiveState, RenderPipelineDescriptor, SamplerBindingType, Shader,
        ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines,
        TextureFormat, TextureSampleType, TextureViewDimension,
    },
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ExtractedView, ViewTarget},
};

use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;

use super::{
    BloomCompositeMode, BloomSettings, BloomUniform, BLOOM_SHADER_HANDLE, BLOOM_TEXTURE_FORMAT,
};

#[derive(Component)]
pub struct UpsamplingPipelineIds {
    pub id_main: CachedRenderPipelineId,
    pub id_final: CachedRenderPipelineId,
}

#[derive(Resource)]
pub struct BloomUpsamplingPipeline {
    bind_group_layout: BindGroupLayout,
}

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct BloomUpsamplingPipelineKeys {
    hdr: bool,
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
                    // Bloom settings
                    //
                    // TODO: Bloom settings are irrelevant right now but we are
                    // creating a slot for them anyway because bled factor calculation
                    // based on bloom settings should be moved to the shader.
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

        BloomUpsamplingPipeline {
            // sampler,
            bind_group_layout,
        }
    }
}

impl SpecializedRenderPipeline for BloomUpsamplingPipeline {
    type Key = BloomUpsamplingPipelineKeys;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let texture_format = if key.final_pipeline {
            if key.hdr {
                ViewTarget::TEXTURE_FORMAT_HDR
            } else {
                TextureFormat::bevy_default()
            }
        } else {
            BLOOM_TEXTURE_FORMAT
        };

        RenderPipelineDescriptor {
            label: Some("bloom_upsampling_pipeline".into()),
            layout: Some(vec![self.bind_group_layout.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: BLOOM_SHADER_HANDLE.typed::<Shader>(),
                shader_defs: vec![],
                entry_point: "upsample".into(),
                targets: vec![Some(ColorTargetState {
                    format: texture_format,
                    blend: Some(BlendState {
                        color: match key.composite_mode {
                            BloomCompositeMode::EnergyConserving => {
                                // At the time of developing this we decided to blend our
                                // blur pyramid levels using native WGPU render pass blend
                                // constants. They are set in the bloom node's run function.
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
                            }
                            BloomCompositeMode::Additive => BlendComponent {
                                src_factor: BlendFactor::Constant,
                                dst_factor: BlendFactor::One,
                                operation: BlendOperation::Add,
                            },
                        },
                        alpha: BlendComponent::REPLACE,
                    }),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
        }
    }
}

pub fn prepare_upsampling_pipeline(
    mut commands: Commands,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<BloomUpsamplingPipeline>>,
    pipeline: Res<BloomUpsamplingPipeline>,
    views: Query<(Entity, &ExtractedView, &BloomSettings), With<BloomSettings>>,
    // cameras: Query<(Entity, &Camera, &BloomSettings), With<Camera>>,
) {
    for (entity, view, settings) in &views {
        let pipeline_id = pipelines.specialize(
            &mut pipeline_cache,
            &pipeline,
            BloomUpsamplingPipelineKeys {
                hdr: view.hdr,
                composite_mode: settings.composite_mode,
                final_pipeline: false,
            },
        );

        let pipeline_final_id = pipelines.specialize(
            &mut pipeline_cache,
            &pipeline,
            BloomUpsamplingPipelineKeys {
                hdr: view.hdr,
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
