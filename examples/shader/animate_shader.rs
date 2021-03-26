use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::shape,
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph::{base, RenderGraph, RenderResourcesNode},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
};

/// This example shows how to animate a shader, by passing the global `time.seconds_since_startup()`
/// via a 'TimeComponent` to the shader
pub fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .add_system(animate_shader.system())
        .run();
}

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "463e4b8a-d555-4fc2-ba9f-4c880063ba92"]
struct TimeComponent {
    value: f32,
}

const VERTEX_SHADER: &str = r#"
#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec2 Vertex_Uv;
layout(location = 0) out vec2 v_Uv;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

void main() {
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
    v_Uv = Vertex_Uv;
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450

layout(location = 0) in vec2 v_Uv;
layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 0) uniform TimeComponent_value {
    float u_time;
};

void main() {
    float speed = 0.7;
    float translation = sin(u_time * speed);
    float percentage = 0.6;
    float threshold = v_Uv.x + translation * percentage;

    vec3 red = vec3(1., 0., 0.);
    vec3 blue = vec3(0., 0., 1.);
    vec3 mixed = mix(red, blue, threshold);

    o_Target = vec4(mixed, 1.0);
}
"#;

fn setup(
    mut commands: Commands,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    // Create a new shader pipeline
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    }));

    // Add a `RenderResourcesNode` to our `RenderGraph`. This will bind `TimeComponent` to our shader
    render_graph.add_system_node(
        "time_component",
        RenderResourcesNode::<TimeComponent>::new(true),
    );

    // Add a `RenderGraph` edge connecting our new "time_component" node to the main pass node. This
    // ensures that "time_component" runs before the main pass
    render_graph
        .add_node_edge("time_component", base::node::MAIN_PASS)
        .unwrap();

    // Spawn a quad and insert the `TimeComponent`
    commands
        .spawn_bundle(MeshBundle {
            mesh: meshes.add(Mesh::from(shape::Quad {
                size: Vec2::new(5.0, 5.0),
                flip: true,
            })),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                pipeline_handle,
            )]),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..Default::default()
        })
        .insert(TimeComponent { value: 0.0 });

    // Spawn a camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 0.0, -8.0).looking_at(Vec3::ZERO, -Vec3::Y),
        ..Default::default()
    });
}

/// In this system we query for the `TimeComponent` and global `Time` resource, and set `time.seconds_since_startup()`
/// as the `value` of the `TimeComponent`. This value will be accessed by the fragment shader and used
/// to animate the shader.
fn animate_shader(time: Res<Time>, mut query: Query<&mut TimeComponent>) {
    for mut time_component in query.iter_mut() {
        time_component.value = time.seconds_since_startup() as f32;
    }
}
