use bevy::{
    prelude::*,
    render::{
        mesh::{VertexAttribute, VertexAttributeValues},
        pipeline::{
            DynamicBinding, PipelineDescriptor, PipelineSpecialization, PrimitiveTopology,
            RenderPipeline,
        },
        shader::{ShaderStage, ShaderStages},
    },
};

/// This example illustrates how to create a mesh asset with a custom vertex format and a shader that uses that mesh
fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

const VERTEX_SHADER: &str = r#"
#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec4 Vertex_Color;

layout(location = 0) out vec4 v_Color;

layout(set = 0, binding = 0) uniform Camera {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

void main() {
    v_Color = Vertex_Color;
    gl_Position = ViewProj * vec4((Model * vec4(Vertex_Position, 1.0)).xyz, 1.0);
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450

layout(location = 0) in vec4 v_Color;

layout(location = 0) out vec4 o_Target;

void main() {
    o_Target = v_Color;
}
"#;

fn setup(
    mut commands: Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Create a new shader pipeline
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    }));

    // Make a simple triangle mesh with colored vertices.
    let vertices = [
        ([-3.0, 0.0, -3.0], [1.0, 0.0, 0.0, 1.0]),
        ([3.0, 0.0, -3.0], [0.0, 1.0, 0.0, 1.0]),
        ([0.0, 0.0, 3.0], [0.0, 0.0, 1.0, 1.0]),
    ];
    let mut positions = Vec::new();
    let mut colors = Vec::new();
    for (position, color) in vertices.iter() {
        positions.push(*position);
        colors.push(*color);
    }
    let triangle_mesh = Mesh {
        primitive_topology: PrimitiveTopology::TriangleStrip,
        attributes: vec![
            // Names here must match the vertex shader attributes.
            VertexAttribute {
                name: "Vertex_Position".into(),
                values: VertexAttributeValues::Float3(positions),
            },
            VertexAttribute {
                name: "Vertex_Color".into(),
                values: VertexAttributeValues::Float4(colors),
            },
        ],
        indices: Some(vec![0, 1, 2]),
    };

    // Setup our world
    commands
        // mesh
        .spawn(MeshComponents {
            mesh: meshes.add(triangle_mesh),
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
                    ],
                    ..Default::default()
                },
            )]),
            translation: Translation::new(0.0, 0.0, 0.0),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(3.0, 5.0, -8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
