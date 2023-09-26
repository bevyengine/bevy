use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res, ResMut, Resource},
};
use bevy_render::{
    globals::GlobalsUniform,
    render_resource::{
        BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
        BufferBindingType, CachedRenderPipelineId, ColorTargetState, ColorWrites, FragmentState,
        MultisampleState, PipelineCache, PrimitiveState, RenderPipelineDescriptor, Sampler,
        SamplerBindingType, SamplerDescriptor, ShaderDefVal, ShaderStages, ShaderType,
        SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat, TextureSampleType,
        TextureViewDimension,
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
        let bind_group_layout_descriptor = BindGroupLayoutDescriptor {
            label: Some("motion_blur_bind_group_layout"),
            entries: &[
                // The screen texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Motion Vectors
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Depth
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // The sampler that will be used to sample the screen texture
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // The settings uniform that will control the effect
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(MotionBlur::min_size()),
                    },
                    count: None,
                },
                // Global uniforms
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GlobalsUniform::min_size()),
                    },
                    count: None,
                },
            ],
        };

        let bind_group_layout_descriptor_msaa = BindGroupLayoutDescriptor {
            label: Some("motion_blur_bind_group_layout"),
            entries: &[
                // The screen texture
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Motion Vectors
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Float { filterable: false },
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                // Depth
                BindGroupLayoutEntry {
                    binding: 2,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        sample_type: TextureSampleType::Depth,
                        view_dimension: TextureViewDimension::D2,
                        multisampled: true,
                    },
                    count: None,
                },
                // The sampler that will be used to sample the screen texture
                BindGroupLayoutEntry {
                    binding: 3,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
                // The settings uniform that will control the effect
                BindGroupLayoutEntry {
                    binding: 4,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(MotionBlur::min_size()),
                    },
                    count: None,
                },
                // Global uniforms
                BindGroupLayoutEntry {
                    binding: 5,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(GlobalsUniform::min_size()),
                    },
                    count: None,
                },
            ],
        };

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());
        let layout = render_device.create_bind_group_layout(&bind_group_layout_descriptor);
        let layout_msaa =
            render_device.create_bind_group_layout(&bind_group_layout_descriptor_msaa);

        Self {
            sampler,
            layout,
            layout_msaa,
        }
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

        match key.samples {
            1 => (),
            _ => shader_defs.push(ShaderDefVal::from("MULTISAMPLED")),
        }

        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
        {
            shader_defs.push("NO_ARRAY_TEXTURES_SUPPORT".into());
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
                    // blend: Some(BlendState::REPLACE),
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
