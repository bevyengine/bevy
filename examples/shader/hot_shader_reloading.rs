use bevy::{
    prelude::*,
    reflect::TypeUuid,
    asset::LoadState,
    render::{
        mesh::shape,
        pipeline::{PipelineDescriptor, RenderPipeline},
        render_graph::{base, AssetRenderResourcesNode, RenderGraph},
        renderer::RenderResources,
        shader::ShaderStages,
    },
};

/// This example illustrates how to load shaders such that they can be
/// edited while the example is still running.
fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_asset::<MyMaterial>()
        .init_resource::<MyShadersHandles>()
        .add_state(AppState::Setup)
        .add_system_set(SystemSet::on_enter(AppState::Setup).with_system(load_shaders.system()))
        .add_system_set(SystemSet::on_update(AppState::Setup).with_system(check_shaders.system()))
        .add_system_set(SystemSet::on_enter(AppState::Finished).with_system(setup.system()))
        .run();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum AppState {
    Setup,
    Finished,
}

#[derive(RenderResources, Default, TypeUuid)]
#[uuid = "3bf9e364-f29d-4d6c-92cf-93298466c620"]
struct MyMaterial {
    pub color: Color,
}

#[derive(Default)]
struct MyShadersHandles {
    vertex: Handle<Shader>,
    fragment: Handle<Shader>
}

fn load_shaders(mut my_shaders: ResMut<MyShadersHandles>, asset_server: Res<AssetServer>) {
    my_shaders.vertex = asset_server.load::<Shader, _>("shaders/hot.vert");
    my_shaders.fragment = asset_server.load::<Shader, _>("shaders/hot.frag");
}

fn check_shaders(mut state: ResMut<State<AppState>>,
                shaders_handles: ResMut<MyShadersHandles>,
                asset_server: Res<AssetServer>) {
    if let LoadState::Loaded = asset_server.get_load_state(shaders_handles.vertex.id) {
        if let LoadState::Loaded = asset_server.get_load_state(shaders_handles.fragment.id) {
            state.set(AppState::Finished).unwrap();
        }
    }
}

fn setup(
    mut commands: Commands,
    my_shaders: Res<MyShadersHandles>,
    asset_server: ResMut<AssetServer>,
    mut pipelines: ResMut<Assets<PipelineDescriptor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<MyMaterial>>,
    mut render_graph: ResMut<RenderGraph>,
) {
    // Watch for changes
    asset_server.watch_for_changes().unwrap();

    // Create a new shader pipeline with shaders loaded from the asset directory
    let pipeline_handle = pipelines.add(PipelineDescriptor::default_config(ShaderStages {
        vertex: my_shaders.vertex.clone(),
        fragment: Some(my_shaders.fragment.clone()),
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

    // Create a new material
    let material = materials.add(MyMaterial {
        color: Color::rgb(0.0, 0.8, 0.0),
    });

    // cube
    commands
        .spawn_bundle(MeshBundle {
            mesh: meshes.add(Mesh::from(shape::Cube { size: 2.0 })),
            render_pipelines: RenderPipelines::from_pipelines(vec![RenderPipeline::new(
                pipeline_handle,
            )]),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..Default::default()
        })
        .insert(material);
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(3.0, 5.0, -8.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}
