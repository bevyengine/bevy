use bevy_core_pipeline::picking::EntityIndexLayout;
use bevy_ecs::prelude::*;
use bevy_render::{
    picking::PICKING_TEXTURE_FORMAT,
    render_resource::*,
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ViewTarget, ViewUniform},
};

#[derive(Resource)]
pub struct UiPipeline {
    pub view_layout: BindGroupLayout,
    pub image_layout: BindGroupLayout,
    pub entity_index_layout: BindGroupLayout,
}

impl FromWorld for UiPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let entity_index_layout = world.resource::<EntityIndexLayout>();

        let view_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: Some(ViewUniform::min_size()),
                },
                count: None,
            }],
            label: Some("ui_view_layout"),
        });

        let image_layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Texture {
                        multisampled: false,
                        sample_type: TextureSampleType::Float { filterable: true },
                        view_dimension: TextureViewDimension::D2,
                    },
                    count: None,
                },
                BindGroupLayoutEntry {
                    binding: 1,
                    visibility: ShaderStages::FRAGMENT,
                    ty: BindingType::Sampler(SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("ui_image_layout"),
        });

        UiPipeline {
            entity_index_layout: entity_index_layout.layout.clone(),
            image_layout,
            view_layout,
        }
    }
}

bitflags::bitflags! {
    #[repr(transparent)]
    pub struct UiPipelineKey: u32 {
        const NONE              = 0;
        const HDR               = (1 << 0);
        const PICKING           = (1 << 1);
    }
}

impl UiPipelineKey {
    pub fn from_hdr(hdr: bool) -> Self {
        if hdr {
            UiPipelineKey::HDR
        } else {
            UiPipelineKey::NONE
        }
    }
}

impl SpecializedRenderPipeline for UiPipeline {
    type Key = UiPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let vertex_formats = vec![
            // position
            VertexFormat::Float32x3,
            // uv
            VertexFormat::Float32x2,
            // color
            VertexFormat::Float32x4,
        ];

        let mut shader_defs = Vec::new();

        if key.contains(UiPipelineKey::PICKING) {
            shader_defs.push("PICKING".into());
        }

        let vertex_layout =
            VertexBufferLayout::from_vertex_formats(VertexStepMode::Vertex, vertex_formats);

        let blend = Some(BlendState::ALPHA_BLENDING);

        let mut targets = vec![Some(ColorTargetState {
            format: if key.contains(UiPipelineKey::HDR) {
                ViewTarget::TEXTURE_FORMAT_HDR
            } else {
                TextureFormat::bevy_default()
            },
            blend,
            write_mask: ColorWrites::ALL,
        })];

        if key.contains(UiPipelineKey::PICKING) {
            targets.push(Some(ColorTargetState {
                format: PICKING_TEXTURE_FORMAT,
                // TODO: Check this is supported
                blend,
                write_mask: ColorWrites::ALL,
            }));
        }

        RenderPipelineDescriptor {
            vertex: VertexState {
                shader: super::UI_SHADER_HANDLE.typed::<Shader>(),
                entry_point: "vertex".into(),
                shader_defs: shader_defs.clone(),
                buffers: vec![vertex_layout],
            },
            fragment: Some(FragmentState {
                shader: super::UI_SHADER_HANDLE.typed::<Shader>(),
                shader_defs,
                entry_point: "fragment".into(),
                targets,
            }),
            layout: Some(vec![
                self.view_layout.clone(),
                self.entity_index_layout.clone(),
                self.image_layout.clone(),
            ]),
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
