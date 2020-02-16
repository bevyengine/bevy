use crate::{asset::AssetStorage, render::{
    render_graph_2::{
        pipeline_layout::*, PipelineDescriptor, RenderGraphBuilder, resource_name,
    },
    shader::{Shader, ShaderStage},
    Vertex,
}};
pub trait ForwardPipelineBuilder {
    fn add_forward_pipeline(self, pipeline_descriptor_storage: &mut AssetStorage<PipelineDescriptor>, shader_storage: &mut AssetStorage<Shader>) -> Self;
}

impl ForwardPipelineBuilder for RenderGraphBuilder {
    fn add_forward_pipeline(self, pipeline_descriptor_storage: &mut AssetStorage<PipelineDescriptor>, shader_storage: &mut AssetStorage<Shader>) -> Self {
        self.add_pipeline(
            pipeline_descriptor_storage,
            PipelineDescriptor::build(
                shader_storage,
                Shader::from_glsl(include_str!("forward.vert"), ShaderStage::Vertex),
            )
            .with_fragment_shader(Shader::from_glsl(
                include_str!("forward.frag"),
                ShaderStage::Fragment,
            ))
            .add_bind_group(BindGroup::new(vec![
                Binding {
                    name: "Camera".to_string(),
                    bind_type: BindType::Uniform {
                        dynamic: false,
                        properties: vec![UniformProperty {
                            name: "ViewProj".to_string(),
                            property_type: UniformPropertyType::Mat4,
                        }],
                    },
                },
                Binding {
                    name: "Lights".to_string(),
                    bind_type: BindType::Uniform {
                        dynamic: false,
                        properties: vec![
                            UniformProperty {
                                name: "NumLights".to_string(),
                                property_type: UniformPropertyType::UVec4,
                            },
                            UniformProperty {
                                name: "SceneLights".to_string(),
                                property_type: UniformPropertyType::Array(
                                    Box::new(UniformPropertyType::Struct(vec![
                                        UniformPropertyType::Mat4, // proj
                                        UniformPropertyType::Vec4, // pos
                                        UniformPropertyType::Vec4, // color
                                    ])),
                                    10, // max lights
                                ),
                            },
                        ],
                    },
                },
            ]))
            .add_bind_group(BindGroup::new(vec![
                Binding {
                    name: "Object".to_string(),
                    bind_type: BindType::Uniform {
                        dynamic: true,
                        properties: vec![UniformProperty {
                            name: "Model".to_string(),
                            property_type: UniformPropertyType::Mat4,
                        }],
                    },
                },
                Binding {
                    name: "StandardMaterial_albedo".to_string(),
                    bind_type: BindType::Uniform {
                        dynamic: true,
                        properties: vec![UniformProperty {
                            name: "Albedo".to_string(),
                            property_type: UniformPropertyType::Vec4,
                        }],
                    },
                },
            ]))
            .with_rasterization_state(wgpu::RasterizationStateDescriptor {
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: wgpu::CullMode::Back,
                depth_bias: 0,
                depth_bias_slope_scale: 0.0,
                depth_bias_clamp: 0.0,
            })
            .with_depth_stencil_state(wgpu::DepthStencilStateDescriptor {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                stencil_read_mask: 0,
                stencil_write_mask: 0,
            })
            .add_color_state(wgpu::ColorStateDescriptor {
                format: wgpu::TextureFormat::Bgra8UnormSrgb,
                color_blend: wgpu::BlendDescriptor::REPLACE,
                alpha_blend: wgpu::BlendDescriptor::REPLACE,
                write_mask: wgpu::ColorWrite::ALL,
            })
            .add_vertex_buffer_descriptor(Vertex::get_vertex_buffer_descriptor())
            .add_draw_target(resource_name::draw_target::ASSIGNED_MESHES)
            .build(),
        )
    }
}
