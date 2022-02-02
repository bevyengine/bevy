use bevy::{
    core_pipeline::{draw_3d_graph, node, AlphaMask3d, Opaque3d, Transparent3d},
    prelude::*,
    render::{
        camera::{ActiveCameras, ExtractedCameraNames},
        render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotValue},
        render_phase::RenderPhase,
        renderer::RenderContext,
        RenderApp, RenderStage,
    },
    window::{CreateWindow, WindowId},
};

/// This example creates a second window and draws a mesh from two different cameras, one in each window
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_startup_system(create_new_window);

    let render_app = app.sub_app_mut(RenderApp);
    render_app.add_system_to_stage(RenderStage::Extract, extract_secondary_camera_phases);
    let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
    graph.add_node(SECONDARY_PASS_DRIVER, SecondaryCameraDriver);
    graph
        .add_node_edge(node::MAIN_PASS_DEPENDENCIES, SECONDARY_PASS_DRIVER)
        .unwrap();
    app.run();
}

fn extract_secondary_camera_phases(mut commands: Commands, active_cameras: Res<ActiveCameras>) {
    if let Some(secondary) = active_cameras.get(SECONDARY_CAMERA_NAME) {
        if let Some(entity) = secondary.entity {
            commands.get_or_spawn(entity).insert_bundle((
                RenderPhase::<Opaque3d>::default(),
                RenderPhase::<AlphaMask3d>::default(),
                RenderPhase::<Transparent3d>::default(),
            ));
        }
    }
}

const SECONDARY_CAMERA_NAME: &str = "Secondary";
const SECONDARY_PASS_DRIVER: &str = "secondary_pass_driver";

fn create_new_window(
    mut create_window_events: EventWriter<CreateWindow>,

    mut commands: Commands,
    mut active_cameras: ResMut<ActiveCameras>,
) {
    let window_id = WindowId::new();

    // sends out a "CreateWindow" event, which will be received by the windowing backend
    create_window_events.send(CreateWindow {
        id: window_id,
        descriptor: WindowDescriptor {
            width: 800.,
            height: 600.,
            vsync: false,
            title: "Second window".to_string(),
            ..Default::default()
        },
    });
    // second window camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        camera: Camera {
            window: window_id,
            name: Some(SECONDARY_CAMERA_NAME.into()),
            ..Default::default()
        },
        transform: Transform::from_xyz(6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });

    active_cameras.add(SECONDARY_CAMERA_NAME);
}

struct SecondaryCameraDriver;
impl Node for SecondaryCameraDriver {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let extracted_cameras = world.get_resource::<ExtractedCameraNames>().unwrap();
        if let Some(camera_3d) = extracted_cameras.entities.get(SECONDARY_CAMERA_NAME) {
            graph.run_sub_graph(
                crate::draw_3d_graph::NAME,
                vec![SlotValue::Entity(*camera_3d)],
            )?;
        }
        Ok(())
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // add entities to the world
    commands.spawn_scene(asset_server.load("models/monkey/Monkey.gltf#Scene0"));
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, 4.0),
        ..Default::default()
    });
    // main camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}
