//! Load a scene from a glTF file and render it with different render groups.

use bevy::{
    color::palettes,
    pbr::DirectionalLightShadowMap,
    prelude::*,
    render::view::{CameraView, PropagateRenderGroups, RenderGroups},
};

fn main() {
    App::new()
        .insert_resource(DirectionalLightShadowMap { size: 4096 })
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, toggle_layers)
        .run();
}

#[derive(Component)]
struct MovedScene;

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3dBundle {
            transform: Transform::from_xyz(0., 1.4, 2.0)
                .looking_at(Vec3::new(0., 0.3, 0.0), Vec3::Y),
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 1500.0,
        },
        CameraView::from_layers(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]),
    ));

    // Plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(5000.0, 5000.0)),
        material: materials.add(Color::srgb(0.3, 0.5, 0.3)),
        ..default()
    });

    // Text
    commands.spawn((
        TextBundle::from_section(
            "Press '1..3' to toggle mesh render layers\n\
            Press '4..6' to toggle directional light render layers\n\
            Press '1 and 7' to toggle the spot light render layers",
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
        RenderGroups::from_layer(0),
    ));

    // Spawn three copies of the scene, each with a different render group.
    for i in 0..3 {
        commands.spawn((
            SceneBundle {
                transform: Transform::from_xyz(i as f32 - 1.0, 0.0, 0.0),
                scene: asset_server.load("models/FlightHelmet/FlightHelmet.gltf#Scene0"),
                ..default()
            },
            RenderGroups::from_layer(i + 1),
            PropagateRenderGroups::Auto,
        ));
    }

    // Spawn three directional lights, each with a different render group.
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
            RenderGroups::from_layer(i + 4),
        ));
    }

    // Spawn a spot light that is in the same layer as mesh 1.
    // - Notice that the mesh does not cast a shadow when the camera can see the light but not the mesh.
    commands.spawn((
        SpotLightBundle {
            transform: Transform::from_xyz(- 3.0, 2.0, 2.0).looking_at(Vec3::ZERO, Vec3::Y),
            spot_light: SpotLight {
                shadows_enabled: true,
                intensity: 10_000_000.0,
                color: palettes::basic::LIME.into(),
                ..default()
            },
            ..default()
        },
        RenderGroups::from_layers(&[1, 7]),
    ));
}

fn toggle_layers(mut query_camera: Query<&mut CameraView>, keyboard: Res<ButtonInput<KeyCode>>) {
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
    if keyboard.just_pressed(KeyCode::Digit7) {
        toggle_camera_layer(&mut camera_view, 7);
    }
}

fn toggle_camera_layer(camera_view: &mut CameraView, layer: usize) {
    if camera_view.contains_layer(layer) {
        camera_view.remove(layer);
    } else {
        camera_view.add(layer);
    }
}
