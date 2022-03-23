use bevy::{
    prelude::*,
    render::camera::RenderTarget,
    window::{CreateWindow, PresentMode, WindowId},
};

/// This example creates a second window and draws a mesh from two different cameras, one in each window
fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_startup_system(create_new_window)
        .run();
}

fn create_new_window(mut create_window_events: EventWriter<CreateWindow>, mut commands: Commands) {
    let window_id = WindowId::new();

    // sends out a "CreateWindow" event, which will be received by the windowing backend
    create_window_events.send(CreateWindow {
        id: window_id,
        descriptor: WindowDescriptor {
            width: 800.,
            height: 600.,
            present_mode: PresentMode::Immediate,
            title: "Second window".to_string(),
            ..default()
        },
    });

    // second window camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        camera: Camera {
            target: RenderTarget::Window(window_id),
            ..default()
        },
        transform: Transform::from_xyz(6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // add entities to the world
    commands.spawn_scene(asset_server.load("models/monkey/Monkey.gltf#Scene0"));
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, 4.0),
        ..default()
    });
    // main camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
