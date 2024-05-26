use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res, ResMut, Resource},
    world::FromWorld,
};
use bevy_render::{
    globals::GlobalsUniform,
    render_resource::{
        binding_types::{
            sampler, texture_2d, texture_2d_multisampled, texture_depth_2d,
            texture_depth_2d_multisampled, uniform_buffer_sized,
        },
        BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId, ColorTargetState,
        ColorWrites, FragmentState, MultisampleState, PipelineCache, PrimitiveState,
        RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderDefVal,
        ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines,
        TextureFormat, TextureSampleType,
    },
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ExtractedView, Msaa, ViewTarget},
};

use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;

use super::{MotionBlur, MOTION_BLUR_SHADER_HANDLE};

#[derive(Resource)]
pub struct MotionBlurPipeline {
    pub(crate) sampler: Sampler,
    pub(crate) layout: BindGroupLayout,
    pub(crate) layout_msaa: BindGroupLayout,
}

impl MotionBlurPipeline {
    pub(crate) fn new(render_device: &RenderDevice) -> Self {
        let mb_layout = &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // View target (read)
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Motion Vectors
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Depth
                texture_depth_2d(),
                // Linear Sampler
                sampler(SamplerBindingType::Filtering),
                // Motion blur settings uniform input
                uniform_buffer_sized(false, Some(MotionBlur::min_size())),
                // Globals uniform input
                uniform_buffer_sized(false, Some(GlobalsUniform::min_size())),
            ),
        );

        let mb_layout_msaa = &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // View target (read)
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Motion Vectors
                texture_2d_multisampled(TextureSampleType::Float { filterable: false }),
                // Depth
                texture_depth_2d_multisampled(),
                // Linear Sampler
                sampler(SamplerBindingType::Filtering),
                // Motion blur settings uniform input
                uniform_buffer_sized(false, Some(MotionBlur::min_size())),
                // Globals uniform input
                uniform_buffer_sized(false, Some(GlobalsUniform::min_size())),
            ),
        );

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());
        let layout = render_device.create_bind_group_layout("motion_blur_layout", mb_layout);
        let layout_msaa =
            render_device.create_bind_group_layout("motion_blur_layout_msaa", mb_layout_msaa);

        Self {
            sampler,
            layout,
            layout_msaa,
        }
    }
}

impl FromWorld for MotionBlurPipeline {
    fn from_world(render_world: &mut bevy_ecs::world::World) -> Self {
        let render_device = render_world.resource::<RenderDevice>().clone();
        MotionBlurPipeline::new(&render_device)
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct MotionBlurPipelineKey {
    hdr: bool,
    samples: u32,
}

impl SpecializedRenderPipeline for MotionBlurPipeline {
    type Key = MotionBlurPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let layout = match key.samples {
            1 => vec![self.layout.clone()],
            _ => vec![self.layout_msaa.clone()],
        };

        let mut shader_defs = vec![];

        if key.samples > 1 {
            shader_defs.push(ShaderDefVal::from("MULTISAMPLED"));
        }

        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        {
            shader_defs.push("NO_DEPTH_TEXTURE_SUPPORT".into());
            shader_defs.push("SIXTEEN_BYTE_ALIGNMENT".into());
        }

        RenderPipelineDescriptor {
            label: Some("motion_blur_pipeline".into()),
            layout,
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: MOTION_BLUR_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
        }
    }
}

#[derive(Component)]
pub struct MotionBlurPipelineId(pub CachedRenderPipelineId);

pub(crate) fn prepare_motion_blur_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<MotionBlurPipeline>>,
    pipeline: Res<MotionBlurPipeline>,
    msaa: Res<Msaa>,
    views: Query<(Entity, &ExtractedView), With<MotionBlur>>,
) {
    for (entity, view) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            MotionBlurPipelineKey {
                hdr: view.hdr,
                samples: msaa.samples(),
            },
        );

        commands
            .entity(entity)
            .insert(MotionBlurPipelineId(pipeline_id));
    }
}
