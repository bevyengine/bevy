use bevy::{
    prelude::*,
    render::{
        pipeline::{PipelineDescriptor, RenderPipeline},
        shader::{ShaderStage, ShaderStages},
    },
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(star)
        .run();
}

fn star(
    mut commands: Commands,
    // We will add a new Mesh for the star being created
    mut meshes: ResMut<Assets<Mesh>>,
    // A pipeline will be added with custom shaders
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    // Access to add new shaders
    mut shaders: ResMut<Assets<Shader>>,
) {
    // We first create a pipeline, which is the sequence of steps that are
    // needed to get to pixels on the screen starting from a description of the
    // geometries in the scene. Pipelines have fixed steps, which sometimes can
    // be turned off (for instance, depth and stencil tests) and programmable
    // steps, the vertex and fragment shaders, that we can customize writing
    // shader programs.

    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        // Vertex shaders are run once for every vertex in the mesh.
        // Each vertex can have attributes associated to it (e.g. position,
        // color, texture mapping). The output of a shader is per-vertex.
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        // Fragment shaders are run for each pixel belonging to a triangle on
        // the screen. Their output is per-pixel.
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    }));

    // Let's define the mesh for the object we want to draw: a nice star.
    // We will specify here what kind of topology is used to define the mesh,
    // that is, how triangles are built from the vertices. We will use a
    // triangle list, meaning that each vertex of the triangle has to be
    // specified.
    let mut star = Mesh::new(bevy::render::pipeline::PrimitiveTopology::TriangleList);

    // Vertices need to have a position attribute. We will use the following
    // vertices (I hope you can spot the star in the schema).
    //
    //        1
    //
    //     10   2
    // 9      0      3
    //     8     4
    //        6
    //   7        5
    //
    // These vertices are specificed in 3D space.
    let mut v_pos = vec![[0.0, 0.0, 0.0]];
    for i in 0..10 {
        // Angle of each vertex is 1/10 of TAU, plus PI/2 for positioning vertex 0
        let a = std::f32::consts::FRAC_PI_2 - i as f32 * std::f32::consts::TAU / 10.0;
        // Radius of internal vertices (2, 4, 6, 8, 10) is 100, it's 200 for external
        let r = (1 - i % 2) as f32 * 100.0 + 100.0;
        // Add the vertex coordinates
        v_pos.push([r * a.cos(), r * a.sin(), 0.0]);
    }
    // Set the position attribute
    star.set_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);
    // And a RGB color attribute as well
    let mut v_color = vec![[0.0, 0.0, 0.0]];
    v_color.extend_from_slice(&[[1.0, 1.0, 0.0]; 10]);
    star.set_attribute("Vertex_Color", v_color);

    // Now, we specify the indices of the vertex that are going to compose the
    // triangles in our star. Vertices in triangles have to be specified in CCW
    // winding (that will be the front face, colored). Since we are using
    // triangle list, we will specify each triangle as 3 vertices
    //   First triangle: 0, 2, 1
    //   Second triangle: 0, 3, 2
    //   Third triangle: 0, 4, 3
    //   etc
    //   Last triangle: 0, 1, 10
    let mut indices = vec![0, 1, 10];
    for i in 2..=10 {
        indices.extend_from_slice(&[0, i, i - 1]);
    }
    star.set_indices(Some(bevy::render::mesh::Indices::U32(indices)));

    // We can now spawn the entities for the star and the camera
    commands.spawn_bundle(MeshBundle {
        mesh: meshes.add(star),
        render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
            pipeline_handle,
        )]),
        ..Default::default()
    });
    commands
        // And use an orthographic projection
        .spawn_bundle(OrthographicCameraBundle::new_2d());
}

const VERTEX_SHADER: &str = r"
#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 1) in vec3 Vertex_Color;

layout(location = 1) out vec3 v_Color;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};

layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

void main() {
    v_Color = Vertex_Color;
    gl_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
}
";

const FRAGMENT_SHADER: &str = r"
#version 450

layout(location = 1) in vec3 v_Color;

layout(location = 0) out vec4 o_Target;

void main() {
    o_Target = vec4(v_Color, 1.0);
}
";
