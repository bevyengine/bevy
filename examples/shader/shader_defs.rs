use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::shape,
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{asset_shader_defs_system, ShaderDefs, ShaderStage, ShaderStages},
    },
};

/// This example illustrates how to create a custom material asset that uses "shader defs" and a
/// shader that uses that material. In Bevy, "shader defs" are a way to selectively enable parts of
/// a shader based on values set in a component or asset.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_asset::<MyMaterial>()
        .add_startup_system(setup)
        .add_system_to_stage(
            CoreStage::PostUpdate,
            asset_shader_defs_system::<MyMaterial>,
        )
        .run();
}

#[derive(RenderResources, ShaderDefs, Default, TypeUuid)]
#[uuid = "620f651b-adbe-464b-b740-ba0e547282ba"]
struct MyMaterial {
    pub color: Color,
    #[render_resources(ignore)]
    #[shader_def]
    pub always_blue: bool,
}

const VERTEX_SHADER: &str = r#"
#version 450
layout(location = 0) in vec3 Vertex_Position;
layout(set = 0, binding = 0) uniform CameraViewProj {
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
layout(set = 2, binding = 0) uniform MyMaterial_color {
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
    mut commands: Commands,
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

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind MyMaterial resources to
    // our shader
    render_graph.add_system_node(
        "my_material",
        AssetRenderResourcesNode::<MyMaterial>::new(true),
    );

    // Add a Render Graph edge connecting our new "my_material" node to the main pass node. This
    // ensures "my_material" runs before the main pass
    render_graph
        .add_node_edge("my_material", base::node::MAIN_PASS)
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
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 2.0 }));

    // cube
    commands
        .spawn_bundle(MeshBundle {
            mesh: cube_handle.clone(),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                pipeline_handle.clone(),
            )]),
            transform: Transform::from_xyz(-2.0, 0.0, 0.0),
            ..Default::default()
        })
        .insert(green_material);
    // cube
    commands
        .spawn_bundle(MeshBundle {
            mesh: cube_handle,
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                pipeline_handle,
            )]),
            transform: Transform::from_xyz(2.0, 0.0, 0.0),
            ..Default::default()
        })
        .insert(blue_material);
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(3.0, 5.0, -8.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}
