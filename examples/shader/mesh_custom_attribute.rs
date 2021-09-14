use bevy::{
    prelude::*,
    render::{
        mesh::{shape, VertexAttributeValues},
        pipeline::{PipelineDescriptor, RenderPipeline},
        shader::{ShaderStage, ShaderStages},
    },
};

/// This example illustrates how to add a custom attribute to a mesh and use it in a custom shader.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

const VERTEX_SHADER: &str = r#"
#version 450
layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Color;
layout(location = 0) out vec3 v_color;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};
layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};
void main() {
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
    v_color = Vertex_Color;
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450
layout(location = 0) out vec4 o_Target;
layout(location = 0) in vec3 v_color;

void main() {
    o_Target = vec4(v_color, 1.0);
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

    // create a generic cube
    let mut cube_with_vertex_colors = Mesh::from(shape::Cube { size: 2.0 });

    // insert our custom color attribute with some nice colors!
    cube_with_vertex_colors.set_attribute(
        // name of the attribute
        "Vertex_Color",
        // the vertex attributes, represented by `VertexAttributeValues`
        // NOTE: the attribute count has to be consistent across all attributes, otherwise bevy
        // will panic.
        VertexAttributeValues::from(vec![
            // top
            [0.79, 0.73, 0.07],
            [0.74, 0.14, 0.29],
            [0.08, 0.55, 0.74],
            [0.20, 0.27, 0.29],
            // bottom
            [0.79, 0.73, 0.07],
            [0.74, 0.14, 0.29],
            [0.08, 0.55, 0.74],
            [0.20, 0.27, 0.29],
            // right
            [0.79, 0.73, 0.07],
            [0.74, 0.14, 0.29],
            [0.08, 0.55, 0.74],
            [0.20, 0.27, 0.29],
            // left
            [0.79, 0.73, 0.07],
            [0.74, 0.14, 0.29],
            [0.08, 0.55, 0.74],
            [0.20, 0.27, 0.29],
            // front
            [0.79, 0.73, 0.07],
            [0.74, 0.14, 0.29],
            [0.08, 0.55, 0.74],
            [0.20, 0.27, 0.29],
            // back
            [0.79, 0.73, 0.07],
            [0.74, 0.14, 0.29],
            [0.08, 0.55, 0.74],
            [0.20, 0.27, 0.29],
        ]),
    );
    // cube
    commands.spawn_bundle(MeshBundle {
        mesh: meshes.add(cube_with_vertex_colors), // use our cube with vertex colors
        render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
            pipeline_handle,
        )]),
        transform: Transform::from_xyz(0.0, 0.0, 0.0),
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(3.0, 5.0, -8.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}
