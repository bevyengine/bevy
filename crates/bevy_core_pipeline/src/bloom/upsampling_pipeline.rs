use crate::FullscreenShader;

use super::{
    downsampling_pipeline::BloomUniforms, Bloom, BloomCompositeMode, BLOOM_TEXTURE_FORMAT,
};
use bevy_asset::{load_embedded_asset, Handle};
use bevy_ecs::{
    error::BevyError,
    prelude::{Component, Entity},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        *,
    },
    renderer::RenderDevice,
    view::ViewTarget,
};
use bevy_utils::default;

#[derive(Component)]
pub struct UpsamplingPipelineIds {
    pub id_main: CachedRenderPipelineId,
    pub id_final: CachedRenderPipelineId,
}

#[derive(Resource)]
pub struct BloomUpsamplingPipeline {
    pub bind_group_layout: BindGroupLayout,
    pub specialized_cache: SpecializedCache<RenderPipeline, BloomUpsamplingSpecializer>,
}

impl FromWorld for BloomUpsamplingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

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

        let fullscreen_shader = world.resource::<FullscreenShader>().clone();
        let fragment_shader = load_embedded_asset!(world, "bloom.wgsl");
        let base_descriptor = RenderPipelineDescriptor {
            label: Some("bloom_upsampling_pipeline".into()),
            layout: vec![bind_group_layout.clone()],
            vertex: fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: fragment_shader.clone(),
                entry_point: Some("upsample".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm, // placeholder
                    blend: Some(BlendState {
                        // placeholder
                        color: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
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
        };

        let specialized_cache =
            SpecializedCache::new(BloomUpsamplingSpecializer, None, base_descriptor);

        BloomUpsamplingPipeline {
            bind_group_layout,
            specialized_cache,
        }
    }
}

pub struct BloomUpsamplingSpecializer;

#[derive(PartialEq, Eq, Hash, Clone, SpecializerKey)]
pub struct BloomUpsamplingKey {
    composite_mode: BloomCompositeMode,
    final_pipeline: bool,
}

impl Specializer<RenderPipeline> for BloomUpsamplingSpecializer {
    type Key = BloomUpsamplingKey;

    fn specialize(
        &self,
        key: Self::Key,
        descriptor: &mut RenderPipelineDescriptor,
    ) -> Result<Canonical<Self::Key>, BevyError> {
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

        let fragment = descriptor.fragment.get_or_insert_default();

        if let Some(Some(color_target)) = fragment.targets.first_mut()
            && let Some(blend_state) = &mut color_target.blend
        {
            blend_state.color = color_blend;
            color_target.format = texture_format;
            Ok(key)
        } else {
            Err("color target state or blend state missing".into())
        }
    }
}

pub fn prepare_upsampling_pipeline(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipeline: ResMut<BloomUpsamplingPipeline>,
    views: Query<(Entity, &Bloom)>,
) -> Result<(), BevyError> {
    for (entity, bloom) in &views {
        let pipeline_id = pipeline.specialized_cache.specialize(
            &pipeline_cache,
            BloomUpsamplingKey {
                composite_mode: bloom.composite_mode,
                final_pipeline: false,
            },
        )?;

        let pipeline_final_id = pipeline.specialized_cache.specialize(
            &pipeline_cache,
            BloomUpsamplingKey {
                composite_mode: bloom.composite_mode,
                final_pipeline: true,
            },
        )?;

        commands.entity(entity).insert(UpsamplingPipelineIds {
            id_main: pipeline_id,
            id_final: pipeline_final_id,
        });
    }
    Ok(())
}
