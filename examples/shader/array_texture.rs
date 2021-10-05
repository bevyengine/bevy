use bevy::{
    prelude::*,
    reflect::TypeUuid,
    render::{
        mesh::shape,
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::{ShaderStage, ShaderStages},
    },
};

/// This example illustrates how to create a texture for use with a texture2DArray shader uniform
/// variable.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_asset::<MyArrayTexture>()
        .add_startup_system(setup)
        .add_system(create_array_texture)
        .run();
}

#[derive(Component, RenderResources, Default, TypeUuid)]
#[uuid = "93fb26fc-6c05-489b-9029-601edf703b6b"]
struct MyArrayTexture {
    pub texture: Handle<Texture>,
}

const VERTEX_SHADER: &str = r#"
#version 450

layout(location = 0) in vec3 Vertex_Position;
layout(location = 0) out vec4 v_Position;

layout(set = 0, binding = 0) uniform CameraViewProj {
    mat4 ViewProj;
};
layout(set = 1, binding = 0) uniform Transform {
    mat4 Model;
};

void main() {
    v_Position = ViewProj * Model * vec4(Vertex_Position, 1.0);
    gl_Position = v_Position;
}
"#;

const FRAGMENT_SHADER: &str = r#"
#version 450

layout(location = 0) in vec4 v_Position;
layout(location = 0) out vec4 o_Target;

layout(set = 2, binding = 0) uniform texture2DArray MyArrayTexture_texture;
layout(set = 2, binding = 1) uniform sampler MyArrayTexture_texture_sampler;

void main() {
    // Screen-space coordinates determine which layer of the array texture we sample.
    vec2 ss = v_Position.xy / v_Position.w;
    float layer = 0.0;
    if (ss.x > 0.0 && ss.y > 0.0) {
        layer = 0.0;
    } else if (ss.x < 0.0 && ss.y > 0.0) {
        layer = 1.0;
    } else if (ss.x > 0.0 && ss.y < 0.0) {
        layer = 2.0;
    } else {
        layer = 3.0;
    }

    // Convert to texture coordinates.
    vec2 uv = (ss + vec2(1.0)) / 2.0;

    o_Target = texture(sampler2DArray(MyArrayTexture_texture, MyArrayTexture_texture_sampler), vec3(uv, layer));
}
"#;

struct LoadingTexture(Option<Handle<Texture>>);

struct MyPipeline(Handle<PipelineDescriptor>);

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut shaders: ResMut<Assets<Shader>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    // Start loading the texture.
    commands.insert_resource(LoadingTexture(Some(
        asset_server.load("textures/array_texture.png"),
    )));

    // Create a new shader pipeline.
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: shaders.add(Shader::from_glsl(ShaderStage::Vertex, VERTEX_SHADER)),
        fragment: Some(shaders.add(Shader::from_glsl(ShaderStage::Fragment, FRAGMENT_SHADER))),
    }));
    commands.insert_resource(MyPipeline(pipeline_handle));

    // Add an AssetRenderResourcesNode to our Render Graph. This will bind MyArrayTexture resources
    // to our shader.
    render_graph.add_system_node(
        "my_array_texture",
        AssetRenderResourcesNode::<MyArrayTexture>::new(true),
    );
    // Add a Render Graph edge connecting our new "my_array_texture" node to the main pass node.
    // This ensures "my_array_texture" runs before the main pass.
    render_graph
        .add_node_edge("my_array_texture", base::node::MAIN_PASS)
        .unwrap();

    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(2.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

fn create_array_texture(
    mut commands: Commands,
    my_pipeline: Res<MyPipeline>,
    mut loading_texture: ResMut<LoadingTexture>,
    mut textures: ResMut<Assets<Texture>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut array_textures: ResMut<Assets<MyArrayTexture>>,
) {
    let (handle, texture) = match loading_texture.0.as_ref() {
        Some(handle) => {
            if let Some(texture) = textures.get_mut(handle) {
                (loading_texture.0.take().unwrap(), texture)
            } else {
                return;
            }
        }
        None => return,
    };

    // Create a new array texture asset from the loaded texture.
    let array_layers = 4;
    texture.reinterpret_stacked_2d_as_array(array_layers);
    let array_texture = array_textures.add(MyArrayTexture { texture: handle });

    // Spawn a cube that's shaded using the array texture.
    commands
        .spawn_bundle(MeshBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                my_pipeline.0.clone(),
            )]),
            ..Default::default()
        })
        .insert(array_texture);
}
