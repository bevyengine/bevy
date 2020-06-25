use bevy::{prelude::*, render::shader};
use bevy_render::{pipeline::{PipelineSpecialization, RenderPipeline, DynamicBinding}, base_render_graph};

fn main() {
    App::build()
        .add_default_plugins()
        .add_asset::<MyMaterial>()
        .add_startup_system(setup.system())
        .add_system_to_stage(
            stage::POST_UPDATE,
            shader::asset_shader_defs_system::<MyMaterial>.system(),
        )
        .run();
}

#[derive(RenderResources, ShaderDefs, Default)]
struct MyMaterial {
    pub color: Color,
    #[render_resources(ignore)]
    #[shader_def]
    pub always_blue: bool,
}

const VERTEX_SHADER: &str = r#"
#version 450
layout(location = 0) in vec3 Vertex_Position;
layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};
layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};
void main() {
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450
layout(location = 0) out vec4 o_Target;
layout(set = 1, binding = 1) uniform MyMaterial_color {
    vec4 color;
};
void main() {
    o_Target = color;

# ifdef MYMATERIAL_ALWAYS_BLUE
    o_Target = vec4(0.0, 0.0, 0.8, 1.0);
# endif
}
"#;

fn setup(
    command_buffer: &mut CommandBuffer,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MyMaterial>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    // Create a new shader pipeline
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    }));

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind MyMaterial resources to our shader
    render_graph.add_system_node(
        "my_material",
        AssetRenderResourcesNode::<MyMaterial>::new(true),
    );

    // Add a Render Graph edge connecting our new "my_material" node to the main pass node
    render_graph
        .add_node_edge("my_material", base_render_graph::node::MAIN_PASS)
        .unwrap();

    // Create a green material
    let green_material = materials.add(MyMaterial {
        color: Color::rgb(0.0, 0.8, 0.0),
        always_blue: false,
    });

    // Create a blue material, which uses our "always_blue" shader def
    let blue_material = materials.add(MyMaterial {
        color: Color::rgb(0.0, 0.0, 0.0),
        always_blue: true,
    });

    // Create a cube mesh which will use our materials
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));

    command_buffer
        .build()
        // cube
        .entity_with(MeshMaterialComponents::<MyMaterial> {
            mesh: cube_handle,
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                pipeline_handle,
                // NOTE: in the future you wont need to manually declare dynamic bindings 
                PipelineSpecialization {
                    dynamic_bindings: vec![
                        // Transform
                        DynamicBinding {
                            bind_group: 1,
                            binding: 0,
                        },
                        // MyMaterial_color
                        DynamicBinding {
                            bind_group: 1,
                            binding: 1,
                        },
                    ],
                    ..Default::default()
                },
            )]),
            material: green_material,
            translation: Translation::new(-2.0, 0.0, 0.0),
            ..Default::default()
        })
        // cube
        .entity_with(MeshMaterialComponents::<MyMaterial> {
            mesh: cube_handle,
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::specialized(
                pipeline_handle,
                // NOTE: in the future you wont need to manually declare dynamic bindings 
                PipelineSpecialization {
                    dynamic_bindings: vec![
                        // Transform
                        DynamicBinding {
                            bind_group: 1,
                            binding: 0,
                        },
                        // MyMaterial_color
                        DynamicBinding {
                            bind_group: 1,
                            binding: 1,
                        },
                    ],
                    ..Default::default()
                },
            )]),
            material: blue_material,
            translation: Translation::new(2.0, 0.0, 0.0),
            ..Default::default()
        })
        // camera
        .entity_with(PerspectiveCameraComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(3.0, 5.0, -8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
