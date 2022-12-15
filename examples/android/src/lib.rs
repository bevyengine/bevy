use bevy::{
    log::{Level, LogPlugin},
    prelude::*,
};

#[no_mangle]
#[cfg(target_os = "android")]
fn android_main(android_app: bevy::winit::android::AndroidApp) {
    let mut app = App::new();
    app.insert_resource(bevy::winit::android::AndroidActivityApp { android_app });
    main(&mut app)
}

pub fn main(app: &mut App) {
    #[cfg(target_os = "android")]
    {
        use bevy::render::settings::{WgpuLimits, WgpuSettings, WgpuSettingsPriority};

        // This configures the app to use the most compatible rendering settings.
        // They help with compatibility with as many devices as possible.
        app.insert_resource(WgpuSettings {
            priority: WgpuSettingsPriority::Compatibility,
            limits: WgpuLimits {
                // Was required for my device and emulator
                max_storage_textures_per_shader_stage: 4,
                ..default()
            },
            ..default()
        });
    }

    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                filter: "android_activity=warn,wgpu=warn".to_string(),
                level: Level::INFO,
            })
            .set(AssetPlugin {
                asset_folder: if cfg!(target_os = "android") {
                    "assets".to_string()
                } else {
                    "../../assets".to_string()
                },
                ..default()
            }),
    )
    .add_startup_system(setup)
    .add_system(rotate_camera)
    .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: 5.0 })),
        material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 1.0 })),
        material: materials.add(Color::rgb(0.8, 0.7, 0.6).into()),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            // shadows may not work on all devices
            shadows_enabled: false,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
    // asset
    commands.spawn(ImageBundle {
        style: Style {
            size: Size::new(Val::Px(50.0), Val::Px(50.0)),
            position_type: PositionType::Absolute,
            position: UiRect {
                left: Val::Px(10.0),
                bottom: Val::Px(10.0),
                ..Default::default()
            },
            ..Default::default()
        },
        image: UiImage::new(asset_server.load("branding/icon.png")),
        ..default()
    });
    // Audio
    let music = asset_server.load("sounds/Windless Slopes.ogg");
    audio.play(music);
}

/// Rotate the camera
fn rotate_camera(mut query: Query<&mut Transform, With<Camera>>, time: Res<Time>) {
    for mut transform in &mut query {
        transform.rotate_around(Vec3::ZERO, Quat::from_rotation_y(time.delta_seconds()));
        transform.look_at(Vec3::ZERO, Vec3::Y);
    }
}
