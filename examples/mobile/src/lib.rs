//! A 3d Scene with a button and playing sound.

use bevy::{
    color::palettes::basic::*,
    input::{gestures::RotationGesture, touch::TouchPhase},
    log::{Level, LogPlugin},
    prelude::*,
    window::{AppLifecycle, ScreenEdge, WindowMode},
    winit::WinitSettings,
};

// the `bevy_main` proc_macro generates the required boilerplate for Android
#[bevy_main]
/// The entry point for the application. Is `pub` so that it can be used from
/// `main.rs`.
pub fn main() {
    let mut app = App::new();
    app.add_plugins(
        DefaultPlugins
            .set(LogPlugin {
                // This will show some log events from Bevy to the native logger.
                level: Level::DEBUG,
                filter: "wgpu=error,bevy_render=info,bevy_ecs=trace".to_string(),
                ..Default::default()
            })
            .set(WindowPlugin {
                primary_window: Some(Window {
                    resizable: false,
                    mode: WindowMode::BorderlessFullscreen(MonitorSelection::Primary),
                    // on iOS, gestures must be enabled.
                    // This doesn't work on Android
                    recognize_rotation_gesture: true,
                    // Only has an effect on iOS
                    prefers_home_indicator_hidden: true,
                    // Only has an effect on iOS
                    prefers_status_bar_hidden: true,
                    // Only has an effect on iOS
                    preferred_screen_edges_deferring_system_gestures: ScreenEdge::Bottom,
                    ..default()
                }),
                ..default()
            }),
    )
    // Make the winit loop wait more aggressively when no user input is received
    // This can help reduce cpu usage on mobile devices
    .insert_resource(WinitSettings::mobile())
    .add_systems(Startup, (setup_scene, setup_music))
    .add_systems(
        Update,
        (
            touch_camera,
            button_handler,
            // Only run the lifetime handler when an [`AudioSink`] component exists in the world.
            // This ensures we don't try to manage audio that hasn't been initialized yet.
            handle_lifetime.run_if(any_with_component::<AudioSink>),
        ),
    )
    .run();
}

fn touch_camera(
    window: Query<&Window>,
    mut touches: EventReader<TouchInput>,
    mut camera_transform: Single<&mut Transform, With<Camera3d>>,
    mut last_position: Local<Option<Vec2>>,
    mut rotations: EventReader<RotationGesture>,
) {
    let Ok(window) = window.single() else {
        return;
    };

    for touch in touches.read() {
        if touch.phase == TouchPhase::Started {
            *last_position = None;
        }
        if let Some(last_position) = *last_position {
            **camera_transform = Transform::from_xyz(
                camera_transform.translation.x
                    + (touch.position.x - last_position.x) / window.width() * 5.0,
                camera_transform.translation.y,
                camera_transform.translation.z
                    + (touch.position.y - last_position.y) / window.height() * 5.0,
            )
            .looking_at(Vec3::ZERO, Vec3::Y);
        }
        *last_position = Some(touch.position);
    }
    // Rotation gestures only work on iOS
    for rotation in rotations.read() {
        let forward = camera_transform.forward();
        camera_transform.rotate_axis(forward, rotation.0 / 10.0);
    }
}

/// set up a simple 3D scene
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.1, 0.2, 0.1))),
    ));
    // cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::default())),
        MeshMaterial3d(materials.add(Color::srgb(0.5, 0.4, 0.3))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));
    // sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5).mesh().ico(4).unwrap())),
        MeshMaterial3d(materials.add(Color::srgb(0.1, 0.4, 0.8))),
        Transform::from_xyz(1.5, 1.5, 1.5),
    ));
    // light
    commands.spawn((
        PointLight {
            intensity: 1_000_000.0,
            // Shadows makes some Android devices segfault, this is under investigation
            // https://github.com/bevyengine/bevy/issues/8214
            #[cfg(not(target_os = "android"))]
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        // MSAA makes some Android devices panic, this is under investigation
        // https://github.com/bevyengine/bevy/issues/8229
        #[cfg(target_os = "android")]
        Msaa::Off,
    ));

    // Test ui
    commands
        .spawn((
            Button,
            Node {
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                position_type: PositionType::Absolute,
                left: Val::Px(50.0),
                right: Val::Px(50.0),
                bottom: Val::Px(50.0),
                ..default()
            },
        ))
        .with_child((
            Text::new("Test Button"),
            TextFont {
                font_size: 30.0,
                ..default()
            },
            TextColor::BLACK,
            TextLayout::new_with_justify(Justify::Center),
        ));
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
    commands.spawn((
        AudioPlayer::new(asset_server.load("sounds/Windless Slopes.ogg")),
        PlaybackSettings::LOOP,
    ));
}

// Pause audio when app goes into background and resume when it returns.
// This is handled by the OS on iOS, but not on Android.
fn handle_lifetime(
    mut lifecycle_events: EventReader<AppLifecycle>,
    music_controller: Single<&AudioSink>,
) {
    for event in lifecycle_events.read() {
        match event {
            AppLifecycle::Idle | AppLifecycle::WillSuspend | AppLifecycle::WillResume => {}
            AppLifecycle::Suspended => music_controller.pause(),
            AppLifecycle::Running => music_controller.play(),
        }
    }
}
