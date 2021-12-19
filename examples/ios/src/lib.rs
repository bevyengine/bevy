use bevy::{input::touch::TouchPhase, prelude::*, window::WindowMode};

// the `bevy_main` proc_macro generates the required ios boilerplate
#[bevy_main]
fn main() {
    App::new()
        .insert_resource(WindowDescriptor {
            vsync: true,
            resizable: false,
            mode: WindowMode::BorderlessFullscreen,
            ..Default::default()
        })
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup_scene)
        .add_startup_system(setup_music)
        .add_system(touch_camera)
        .run();
}

fn touch_camera(
    windows: ResMut<Windows>,
    mut touches: EventReader<TouchInput>,
    mut camera: Query<&mut Transform, With<Camera>>,
    mut last_position: Local<Option<Vec2>>,
) {
    for touch in touches.iter() {
        if touch.phase == TouchPhase::Started {
            *last_position = None;
        }
        if let Some(last_position) = *last_position {
            let window = windows.get_primary().unwrap();
            let mut transform = camera.single_mut();
            *transform = Transform::from_xyz(
                transform.translation.x
                    + (touch.position.x - last_position.x) / window.width() * 5.0,
                transform.translation.y,
                transform.translation.z
                    + (touch.position.y - last_position.y) / window.height() * 5.0,
            )
            .looking_at(Vec3::ZERO, Vec3::Y);
        }
        *last_position = Some(touch.position);
    }
}

/// set up a simple 3D scene
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.1, 0.2, 0.1).into()),
        ..Default::default()
    });
    // cube
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.5, 0.4, 0.3).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..Default::default()
    });
    // sphere
    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Icosphere {
            subdivisions: 4,
            radius: 0.5,
        })),
        material: materials.add(Color::rgb(0.1, 0.4, 0.8).into()),
        transform: Transform::from_xyz(1.5, 1.5, 1.5),
        ..Default::default()
    });
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        point_light: PointLight {
            intensity: 5000.0,
            shadows_enabled: true,
            ..Default::default()
        },
        ..Default::default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

fn setup_music(asset_server: Res<AssetServer>, audio: Res<Audio>) {
    let music = asset_server.load("sounds/Windless Slopes.mp3");
    audio.play(music);
}
