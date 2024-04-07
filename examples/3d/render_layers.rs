//! Load a scene from a glTF file and render it with different render layers.

use bevy::{
    color::palettes,
    pbr::DirectionalLightShadowMap,
    prelude::*,
    render::camera::Viewport,
    render::view::{CameraLayer, PropagateRenderLayers, RenderLayers},
    window::PrimaryWindow,
};

fn main() {
    App::new()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (toggle_layers_camera1, toggle_layers_camera2))
        .run();
}

#[derive(Component)]
struct Camera1;

#[derive(Component)]
struct Camera2;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    window: Query<&Window, With<PrimaryWindow>>,
) {
    // Camera 1
    let window = window.single();
    let camera1 = commands
        .spawn((
            Camera3dBundle {
                camera: Camera {
                    viewport: Some(Viewport {
                        physical_position: UVec2 {
                            x: window.physical_width() / 4,
                            y: 0,
                        },
                        physical_size: UVec2 {
                            x: window.physical_width() / 2,
                            y: window.physical_height() / 2,
                        },
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                transform: Transform::from_xyz(0., 1.4, 2.0)
                    .looking_at(Vec3::new(0., 0.3, 0.0), Vec3::Y),
                ..default()
            },
            EnvironmentMapLight {
                diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
                specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
                intensity: 1500.0,
            },
            CameraLayer::new(0),
            Camera1,
        ))
        .id();

    // Camera 2
    let camera2 = commands
        .spawn((
            Camera3dBundle {
                camera: Camera {
                    order: 1,
                    viewport: Some(Viewport {
                        physical_position: UVec2 {
                            x: window.physical_width() / 4,
                            y: window.physical_height() / 2,
                        },
                        physical_size: UVec2 {
                            x: window.physical_width() / 2,
                            y: window.physical_height() / 2,
                        },
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                transform: Transform::from_xyz(0., 1.4, 2.0)
                    .looking_at(Vec3::new(0., 0.3, 0.0), Vec3::Y),
                ..default()
            },
            EnvironmentMapLight {
                diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
                specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
                intensity: 1500.0,
            },
            CameraLayer::new(0),
            Camera2,
        ))
        .id();

    // Plane
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(5000.0, 5000.0)),
            material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
            ..default()
        },
        RenderLayers::from_layer(0),
    ));

    // Text (camera 1)
    commands.spawn((
        TextBundle::from_section(
            "Camera 1:\n\
            Press '1..3' to toggle mesh render layers\n\
            Press '4..6' to toggle directional light render layers",
            TextStyle {
                font_size: 20.,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
        TargetCamera(camera1),
    ));

    // Text (camera 2)
    commands.spawn((
        TextBundle::from_section(
            "Camera 2:\n\
            Press 'Q/W/E' to toggle mesh render layers\n\
            Press 'R/T/Y' to toggle directional light render layers",
            TextStyle {
                font_size: 20.,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
        TargetCamera(camera2),
    ));

    // Spawn three copies of the scene, each with a different render layer.
    for i in 0..3 {
        commands.spawn((
            SceneBundle {
                transform: Transform::from_xyz(i as f32 - 1.0, 0.0, 0.0),
                scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
                ..default()
            },
            RenderLayers::from_layer(i + 1),
            PropagateRenderLayers::Auto,
        ));
    }

    // Spawn three directional lights, each with a different render layer.
    let colors = [
        palettes::basic::RED,
        palettes::basic::GREEN,
        palettes::basic::NAVY,
    ];
    for (i, color) in (0..3).zip(colors.iter()) {
        commands.spawn((
            DirectionalLightBundle {
                transform: Transform::from_xyz(4.0, 25.0, 8.0).looking_at(Vec3::ZERO, Vec3::Y),
                directional_light: DirectionalLight {
                    shadows_enabled: true,
                    illuminance: 100_000.0,
                    color: (*color).into(),
                    ..default()
                },
                ..default()
            },
            RenderLayers::from_layer(i + 4),
        ));
    }
}

fn toggle_layers_camera1(
    mut query_camera: Query<&mut CameraLayer, With<Camera1>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut camera_view) = query_camera.get_single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::Digit1) {
        toggle_camera_layer(&mut camera_view, 1);
    }
    if keyboard.just_pressed(KeyCode::Digit2) {
        toggle_camera_layer(&mut camera_view, 2);
    }
    if keyboard.just_pressed(KeyCode::Digit3) {
        toggle_camera_layer(&mut camera_view, 3);
    }
    if keyboard.just_pressed(KeyCode::Digit4) {
        toggle_camera_layer(&mut camera_view, 4);
    }
    if keyboard.just_pressed(KeyCode::Digit5) {
        toggle_camera_layer(&mut camera_view, 5);
    }
    if keyboard.just_pressed(KeyCode::Digit6) {
        toggle_camera_layer(&mut camera_view, 6);
    }
}

fn toggle_layers_camera2(
    mut query_camera: Query<&mut CameraLayer, With<Camera2>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    let Ok(mut camera_view) = query_camera.get_single_mut() else {
        return;
    };

    if keyboard.just_pressed(KeyCode::KeyQ) {
        toggle_camera_layer(&mut camera_view, 1);
    }
    if keyboard.just_pressed(KeyCode::KeyW) {
        toggle_camera_layer(&mut camera_view, 2);
    }
    if keyboard.just_pressed(KeyCode::KeyE) {
        toggle_camera_layer(&mut camera_view, 3);
    }
    if keyboard.just_pressed(KeyCode::KeyR) {
        toggle_camera_layer(&mut camera_view, 4);
    }
    if keyboard.just_pressed(KeyCode::KeyT) {
        toggle_camera_layer(&mut camera_view, 5);
    }
    if keyboard.just_pressed(KeyCode::KeyY) {
        toggle_camera_layer(&mut camera_view, 6);
    }
}

fn toggle_camera_layer(camera_view: &mut CameraLayer, layer: usize) {
    if camera_view.equals(layer) {
        camera_view.clear();
    } else {
        camera_view.set(layer);
    }
}
