use bevy::{
    core_pipeline::{draw_3d_graph, AlphaMask3d, Opaque3d, Transparent3d},
    prelude::*,
    render::{
        camera::{ActiveCameras, CameraPlugin, ExtractedCameraNames, Viewport},
        render_graph::{self, NodeRunError, RenderGraph, RenderGraphContext, SlotValue},
        render_phase::RenderPhase,
        renderer::RenderContext,
        RenderApp, RenderStage,
    },
};

fn main() {
    App::new()
        // MSAA doesn't work due to https://github.com/bevyengine/bevy/issues/3499
        .insert_resource(Msaa { samples: 1 })
        .add_plugins(DefaultPlugins)
        .add_plugin(SecondaryCamPlugin)
        .add_startup_system(setup)
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut active_cameras: ResMut<ActiveCameras>,
) {
    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..Default::default()
    });
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..Default::default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..Default::default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..Default::default()
    });

    // top camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            name: Some(CameraPlugin::CAMERA_3D.to_string()),
            viewport: Some(Viewport {
                x: 0.0,
                y: 0.0,
                w: 1.0,
                h: 0.5,
                ..Default::default()
            }),
            ..Camera::default()
        },
        ..Default::default()
    });
    // bottom camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            name: Some(SECONDARY_CAMERA_NAME.to_string()),
            viewport: Some(Viewport {
                x: 0.0,
                y: 0.5,
                w: 1.0,
                h: 0.5,
                ..Default::default()
            }),
            ..Camera::default()
        },
        ..Default::default()
    });
    active_cameras.add(SECONDARY_CAMERA_NAME);
}

const SECONDARY_CAMERA_NAME: &str = "Secondary";
const SECONDARY_PASS_DRIVER: &str = "secondary_pass_driver";

struct SecondaryCamPlugin;

impl Plugin for SecondaryCamPlugin {
    fn build(&self, app: &mut App) {
        let render_app = app.sub_app_mut(RenderApp);
        render_app.add_system_to_stage(RenderStage::Extract, extract_secondary_camera_phases);
        let mut graph = render_app.world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_node(SECONDARY_PASS_DRIVER, SecondaryCameraDriver);
        graph
            .add_node_edge(
                bevy::core_pipeline::node::MAIN_PASS_DEPENDENCIES,
                SECONDARY_PASS_DRIVER,
            )
            .unwrap();
        graph
            .add_node_edge(SECONDARY_PASS_DRIVER, bevy::ui::node::UI_PASS_DRIVER)
            .unwrap();
    }
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

struct SecondaryCameraDriver;
impl render_graph::Node for SecondaryCameraDriver {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let extracted_cameras = world.get_resource::<ExtractedCameraNames>().unwrap();
        if let Some(camera_3d) = extracted_cameras.entities.get(SECONDARY_CAMERA_NAME) {
            graph.run_sub_graph(draw_3d_graph::NAME, vec![SlotValue::Entity(*camera_3d)])?;
        }
        Ok(())
    }
}
