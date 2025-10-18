//! Uses two windows to visualize a 3D model from different angles.

use bevy::{camera::RenderTarget, prelude::*, window::WindowRef};

fn main() {
    App::new()
        // By default, a primary window gets spawned by `WindowPlugin`, contained in `DefaultPlugins`
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .run();
}

fn setup_scene(mut commands: Commands, asset_server: Res<AssetServer>) {
    // add entities to the world
    commands.spawn(SceneRoot(
        asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/torus/torus.gltf")),
    ));
    // light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    let first_window_camera = commands
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(0.0, 0.0, 6.0).looking_at(Vec3::ZERO, Vec3::Y),
        ))
        .id();

    // Spawn a second window
    let second_window = commands
        .spawn(Window {
            title: "Second window".to_owned(),
            ..default()
        })
        .id();

    let second_window_camera = commands
        .spawn((
            Camera3d::default(),
            Transform::from_xyz(6.0, 0.0, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
            Camera {
                target: RenderTarget::Window(WindowRef::Entity(second_window)),
                ..default()
            },
        ))
        .id();

    let node = Node {
        position_type: PositionType::Absolute,
        top: px(12),
        left: px(12),
        ..default()
    };

    commands
        .spawn((
            node.clone(),
            // Since we are using multiple cameras, we need to specify which camera UI should be rendered to
            UiTargetCamera(first_window_camera),
        ))
        .with_child((Text::new("First window"), TextShadow::default()));

    commands
        .spawn((node, UiTargetCamera(second_window_camera)))
        .with_child((Text::new("Second window"), TextShadow::default()));
}
