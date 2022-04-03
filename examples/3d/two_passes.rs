use bevy::{
    core_pipeline::{draw_3d_graph, node, AlphaMask3d, Opaque3d, Transparent3d},
    prelude::*,
    render::{
        camera::{ActiveCamera, Camera, CameraTypePlugin, RenderTarget},
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotValue},
        render_phase::RenderPhase,
        renderer::RenderContext,
        view::RenderLayers,
        RenderApp, RenderStage,
    },
    window::WindowId,
};

// The name of the final node of the first pass.
pub const FIRST_PASS_DRIVER: &str = "first_pass_driver";

// Marks the camera that determines the view rendered in the first pass.
#[derive(Component, Default)]
struct FirstPassCamera;

fn main() {
    let mut app = App::new();
    app.insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_plugin(CameraTypePlugin::<FirstPassCamera>::default())
        .add_startup_system(setup)
        .add_system(cube_rotator_system)
        .add_system(rotator_system)
        .add_system(toggle_msaa);

    let render_app = app.sub_app_mut(RenderApp);
    let driver = FirstPassCameraDriver::new(&mut render_app.world);

    // This will add 3D render phases for the new camera.
    render_app.add_system_to_stage(RenderStage::Extract, extract_first_pass_camera_phases);

    let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();

    // Add a node for the first pass.
    graph.add_node(FIRST_PASS_DRIVER, driver);

    // The first pass's dependencies include those of the main pass.
    graph
        .add_node_edge(node::MAIN_PASS_DEPENDENCIES, FIRST_PASS_DRIVER)
        .unwrap();

    // Insert the first pass node: CLEAR_PASS_DRIVER -> FIRST_PASS_DRIVER -> MAIN_PASS_DRIVER
    graph
        .add_node_edge(node::CLEAR_PASS_DRIVER, FIRST_PASS_DRIVER)
        .unwrap();
    graph
        .add_node_edge(FIRST_PASS_DRIVER, node::MAIN_PASS_DRIVER)
        .unwrap();
    app.run();
}

// Add 3D render phases for FirstPassCamera.
fn extract_first_pass_camera_phases(
    mut commands: Commands,
    active: Res<ActiveCamera<FirstPassCamera>>,
) {
    if let Some(entity) = active.get() {
        commands.get_or_spawn(entity).insert_bundle((
            RenderPhase::<Opaque3d>::default(),
            RenderPhase::<AlphaMask3d>::default(),
            RenderPhase::<Transparent3d>::default(),
        ));
    }
}
// A node for the first pass camera that runs draw_3d_graph with this camera.
struct FirstPassCameraDriver {
    query: QueryState<Entity, With<FirstPassCamera>>,
}

impl FirstPassCameraDriver {
    pub fn new(render_world: &mut World) -> Self {
        Self {
            query: QueryState::new(render_world),
        }
    }
}

impl Node for FirstPassCameraDriver {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        for camera in self.query.iter_manual(world) {
            graph.run_sub_graph(draw_3d_graph::NAME, vec![SlotValue::Entity(camera)])?;
        }
        Ok(())
    }
}

// Marks the first pass cube.
#[derive(Component)]
struct FirstPassCube;

// Marks the main pass cube.
#[derive(Component)]
struct MainPassCube;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 4.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        base_color: Color::GREEN,
        reflectance: 0.02,
        unlit: false,
        ..Default::default()
    });

    let split = 2.0;

    // This specifies the layer used for the first pass, which will be attached to the first pass camera and cube.
    let first_pass_layer = RenderLayers::layer(1);

    // The first pass cube.
    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Transform::from_translation(Vec3::new(-split, 0.0, 1.0)),
            ..Default::default()
        })
        .insert(FirstPassCube)
        .insert(first_pass_layer);

    // Light
    // NOTE: Currently lights are shared between passes - see https://github.com/bevyengine/bevy/issues/3462
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 10.0)),
        ..Default::default()
    });

    // First pass camera
    commands
        .spawn_bundle(PerspectiveCameraBundle::<FirstPassCamera> {
            camera: Camera {
                target: RenderTarget::Window(WindowId::primary()),
                ..Default::default()
            },
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 15.0))
                .looking_at(Vec3::default(), Vec3::Y),
            ..PerspectiveCameraBundle::new()
        })
        .insert(first_pass_layer);

    let cube_size = 4.0;
    let cube_handle = meshes.add(Mesh::from(shape::Box::new(cube_size, cube_size, cube_size)));

    let material_handle = materials.add(StandardMaterial {
        base_color: Color::RED,
        reflectance: 0.02,
        unlit: false,
        ..Default::default()
    });

    // Main pass cube.
    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle,
            material: material_handle,
            transform: Transform {
                translation: Vec3::new(split, 0.0, -4.5),
                rotation: Quat::from_rotation_x(-std::f32::consts::PI / 5.0),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(MainPassCube);

    // The main pass camera.
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_translation(Vec3::new(0.0, 0.0, 15.0))
            .looking_at(Vec3::default(), Vec3::Y),
        ..Default::default()
    });
}

/// Rotates the inner cube (first pass)
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<FirstPassCube>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(1.5 * time.delta_seconds());
        transform.rotation *= Quat::from_rotation_z(1.3 * time.delta_seconds());
    }
}

/// Rotates the outer cube (main pass)
fn cube_rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<MainPassCube>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(1.0 * time.delta_seconds());
        transform.rotation *= Quat::from_rotation_y(0.7 * time.delta_seconds());
    }
}

fn toggle_msaa(input: Res<Input<KeyCode>>, mut msaa: ResMut<Msaa>) {
    if input.just_pressed(KeyCode::M) {
        if msaa.samples == 4 {
            info!("Not using MSAA");
            msaa.samples = 1;
        } else {
            info!("Using 4x MSAA");
            msaa.samples = 4;
        }
    }
}
