//! A 3d Scene with a button and playing sound.

use bevy::{
    color::palettes::basic::*,
    input::{gestures::RotationGesture, touch::TouchPhase},
    prelude::*,
    window::{AppLifecycle, WindowMode},
};

// the `bevy_main` proc_macro generates the required boilerplate for iOS and Android
#[bevy_main]
fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            resizable: false,
            mode: WindowMode::BorderlessFullscreen,
            // on iOS, gestures must be enabled.
            // This doesn't work on Android
            recognize_rotation_gesture: true,
            ..default()
        }),
        ..default()
    }))
    .add_systems(Startup, (setup_scene, setup_music))
    .add_systems(Update, (touch_camera, button_handler, handle_lifetime));

    // MSAA makes some Android devices panic, this is under investigation
    // https://github.com/bevyengine/bevy/issues/8229
    #[cfg(target_os = "android")]
    app.insert_resource(Msaa::Off);

    app.run();
}

fn touch_camera(
    windows: Query<&Window>,
    mut touches: EventReader<TouchInput>,
    mut camera: Query<&mut Transform, With<Camera3d>>,
    mut last_position: Local<Option<Vec2>>,
    mut rotations: EventReader<RotationGesture>,
) {
    let window = windows.single();

    for touch in touches.read() {
        if touch.phase == TouchPhase::Started {
            *last_position = None;
        }
        if let Some(last_position) = *last_position {
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
    // Rotation gestures only work on iOS
    for rotation in rotations.read() {
        let mut transform = camera.single_mut();
        let forward = transform.forward();
        transform.rotate_axis(forward, rotation.0 / 10.0);
    }
}

/// set up a simple 3D scene
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(5.0, 5.0)),
        material: materials.add(Color::srgb(0.1, 0.2, 0.1)),
        ..default()
    });
    // cube
    commands.spawn(PbrBundle {
        mesh: meshes.add(Cuboid::default()),
        material: materials.add(Color::srgb(0.5, 0.4, 0.3)),
        transform: Transform::from_xyz(0.0, 0.5, 0.0),
        ..default()
    });
    // sphere
    commands.spawn(PbrBundle {
        mesh: meshes.add(Sphere::new(0.5).mesh().ico(4).unwrap()),
        material: materials.add(Color::srgb(0.1, 0.4, 0.8)),
        transform: Transform::from_xyz(1.5, 1.5, 1.5),
        ..default()
    });
    // light
    commands.spawn(PointLightBundle {
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        point_light: PointLight {
            intensity: 1_000_000.0,
            // Shadows makes some Android devices segfault, this is under investigation
            // https://github.com/bevyengine/bevy/issues/8214
            #[cfg(not(target_os = "android"))]
            shadows_enabled: true,
            ..default()
        },
        ..default()
    });
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Test ui
    commands
        .spawn((
            ButtonBundle {
                style: Style {
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    position_type: PositionType::Absolute,
                    left: Val::Px(50.0),
                    right: Val::Px(50.0),
                    bottom: Val::Px(50.0),
                    ..default()
                },
                image: UiImage::default().with_color(Color::NONE),
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ))
        .with_children(|b| {
            b.spawn(
                TextBundle::from_section(
                    "Test Button",
                    TextStyle {
                        font_size: 30.0,
                        color: Color::BLACK,
                        ..default()
                    },
                )
                .with_text_justify(JustifyText::Center),
            );
        });
}

fn button_handler(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = BLUE.into();
            }
            Interaction::Hovered => {
                *color = GRAY.into();
            }
            Interaction::None => {
                *color = WHITE.into();
            }
        }
    }
}

fn setup_music(asset_server: Res<AssetServer>, mut commands: Commands) {
    commands.spawn(AudioBundle {
        source: asset_server.load("sounds/Windless Slopes.ogg"),
        settings: PlaybackSettings::LOOP,
    });
}

// Pause audio when app goes into background and resume when it returns.
// This is handled by the OS on iOS, but not on Android.
fn handle_lifetime(
    mut lifecycle_events: EventReader<AppLifecycle>,
    music_controller: Query<&AudioSink>,
) {
    let Ok(music_controller) = music_controller.get_single() else {
        return;
    };

    for event in lifecycle_events.read() {
        match event {
            AppLifecycle::Idle | AppLifecycle::WillSuspend | AppLifecycle::WillResume => {}
            AppLifecycle::Suspended => music_controller.pause(),
            AppLifecycle::Running => music_controller.play(),
        }
    }
}
