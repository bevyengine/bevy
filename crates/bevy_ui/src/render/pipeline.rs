use bevy_ecs::prelude::*;
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        *,
    },
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ViewTarget, ViewUniform},
};

#[derive(Resource)]
pub struct UiPipeline {
    pub view_layout: BindGroupLayout,
    pub image_layout: BindGroupLayout,
}

impl FromWorld for UiPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let view_layout = render_device.create_bind_group_layout(
            "ui_view_layout",
            &BindGroupLayoutEntries::single(
                ShaderStages::VERTEX_FRAGMENT,
                uniform_buffer::<ViewUniform>(true),
            ),
        );

        let image_layout = render_device.create_bind_group_layout(
            "ui_image_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    sampler(SamplerBindingType::Filtering),
                ),
            ),
        );

        UiPipeline {
            view_layout,
            image_layout,
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum UiPipelineSpecialization {
    Node,
    Text,
    LinearGradient,
    RadialGradient,
    DashedBorder,
    Shadow,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct UiPipelineKey {
    pub hdr: bool,
    pub clip: bool,
    pub specialization: UiPipelineSpecialization,
}

impl SpecializedRenderPipeline for UiPipeline {
    type Key = UiPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();
        let mut formats = vec![];

        match key.specialization {
            UiPipelineSpecialization::Node => {
                shader_defs.push("SPECIAL".into());
                shader_defs.push("NODE".into());
                formats.extend([
                    // @location(0) i_location: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(1) i_size: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(2) i_flags: u32,
                    VertexFormat::Uint32,
                    // @location(3) i_border: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(4) i_radius: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(5) i_color: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(6) i_uv: vec4<f32>,
                    VertexFormat::Float32x4,
                ]);
            }
            UiPipelineSpecialization::Text => {
                shader_defs.push("SPECIAL".into());
                shader_defs.push("TEXT".into());
                formats.extend([
                    // @location(0) i_location: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(1) i_size: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(2) i_uv_min: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(3) i_uv_size: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(4) i_color: vec4<f32>,
                    VertexFormat::Float32x4,
                ]);
            }
            UiPipelineSpecialization::LinearGradient => {
                shader_defs.push("SPECIAL".into());
                shader_defs.push("LINEAR_GRADIENT".into());
                formats.extend([
                    // @location(0) i_location: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(1) i_size: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(2) i_flags: u32,
                    VertexFormat::Uint32,
                    // @location(3) i_border: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(4) i_radius: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(5) focal_point: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(6) angle: f32,
                    VertexFormat::Float32,
                    // @location(7) start_color: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(8) start_len: f32,
                    VertexFormat::Float32,
                    // @location(9) end_len: f32,
                    VertexFormat::Float32,
                    // @location(10) end_color: vec4<f32>,
                    VertexFormat::Float32x4,
                ]);
            }
            UiPipelineSpecialization::RadialGradient => {
                shader_defs.push("SPECIAL".into());
                shader_defs.push("RADIAL_GRADIENT".into());
                formats.extend([
                    // @location(0) i_location: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(1) i_size: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(4) i_flags: u32,
                    VertexFormat::Uint32,
                    // @location(2) i_border: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(3) i_radius: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(5) center: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(6) ratio: f32,
                    VertexFormat::Float32,
                    // @location(7) start_color: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(8) start_len: f32,
                    VertexFormat::Float32,
                    // @location(9) end_len: f32,
                    VertexFormat::Float32,
                    // @location(10) end_color: vec4<f32>,
                    VertexFormat::Float32x4,
                ]);
            }
            UiPipelineSpecialization::DashedBorder => {
                shader_defs.push("SPECIAL".into());
                shader_defs.push("DASHED_BORDER".into());
                formats.extend([
                    // @location(0) i_location: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(1) i_size: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(2) i_line_thickness: f32,
                    VertexFormat::Float32,
                    // @location(3) i_color: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(4) i_radius: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(5) i_dash_length: f32,
                    VertexFormat::Float32,
                    // @location(6) i_break_length: f32,
                    VertexFormat::Float32,
                ]);
            }
            UiPipelineSpecialization::Shadow => {
                shader_defs.push("SPECIAL".into());
                shader_defs.push("SHADOW".into());
                formats.extend([
                    // @location(0) i_location: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(1) i_size: vec2<f32>,
                    VertexFormat::Float32x2,
                    // @location(2) i_radius: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(3) i_color: vec4<f32>,
                    VertexFormat::Float32x4,
                    // @location(4) i_blur_radius: f32,
                    VertexFormat::Float32,
                ]);
            }
        }

        if key.clip {
            shader_defs.push("CLIP".into());
            // @location(?) i_flags: u32,
            formats.push(VertexFormat::Float32x4);
        }

        let instance_rate_vertex_buffer_layout =
            VertexBufferLayout::from_vertex_formats(VertexStepMode::Instance, formats);
        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: super::UI_SHADER_HANDLE,
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![instance_rate_vertex_buffer_layout],
            },
            fragment: Some(FragmentState {
                shader: super::UI_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: Some(BlendState::ALPHA_BLENDING),
                    write_mask: ColorWrites::ALL,
                })],
            }),
            layout: vec![self.view_layout.clone(), self.image_layout.clone()],
            push_constant_ranges: Vec::new(),
            primitive: PrimitiveState {
                front_face: FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: PolygonMode::Fill,
                conservative: false,
                topology: PrimitiveTopology::TriangleList,
                strip_index_format: None,
            },
            depth_stencil: None,
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            label: Some("ui_pipeline".into()),
        }
    }
}
