use bevy::{
    prelude::*,
    render::{
        render_graph::{PipelineDescriptor, resource_name},
        Shader, ShaderStage, Vertex,
    },
};

use bevy_derive::Uniforms;

// #[derive(Uniforms)]
struct MyMaterial {
    pub color: Vec4
}

fn main() {
    AppBuilder::new()
        .add_defaults()
        .setup_world(setup)
        .setup_render_graph(|builder, pipeline_storage, shader_storage| {
            builder
                // .add_resource_provider(UniformResourceProvider::<MyMaterial>::new())
                .add_pipeline_to_pass(
                    resource_name::pass::MAIN,
                    pipeline_storage,
                    PipelineDescriptor::build(
                        shader_storage, Shader::from_glsl(
                            ShaderStage::Vertex,r#"
                                #version 450
                                layout(location = 0) in vec4 a_Pos;
                                layout(location = 0) out vec4 v_Position;
                                layout(set = 0, binding = 0) uniform Camera {
                                    mat4 ViewProj;
                                };
                                layout(set = 1, binding = 0) uniform Object {
                                    mat4 Model;
                                };
                                void main() {
                                    v_Position = Model * vec4(a_Pos);
                                    gl_Position = ViewProj * v_Position;
                                }
                            "#),
                    )
                    .with_fragment_shader(
                        Shader::from_glsl(
                            ShaderStage::Fragment, r#"
                                #version 450
                                layout(location = 0) in vec4 v_Position;
                                layout(location = 0) out vec4 o_Target;
                                layout(set = 1, binding = 1) uniform MyMaterial_color {
                                    vec4 color;
                                };
                                void main() {
                                    o_Target = color;
                                }
                        "#)
                    )
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
        })
        .run();
}

fn setup(world: &mut World) {
    let cube_handle = {
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        mesh_storage.add(Mesh::load(MeshType::Cube))
    };

    world
        .build()
        // red cube
        .add_archetype(MeshEntity {
            mesh: cube_handle,
            translation: Translation::new(0.0, 0.0, 1.0),
            ..MeshEntity::default()
        })
        // camera
        .add_archetype(CameraEntity {
            camera: Camera::new(CameraType::Projection {
                fov: std::f32::consts::PI / 4.0,
                near: 1.0,
                far: 1000.0,
                aspect_ratio: 1.0,
            }),
            active_camera: ActiveCamera,
            local_to_world: LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
        })
        .build();
}
