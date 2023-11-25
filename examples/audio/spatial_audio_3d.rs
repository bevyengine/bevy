//! This example illustrates how to load and play an audio file, and control where the sounds seems to come from.
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_positions)
        .add_systems(Update, update_listener)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Space between the two ears
    let gap = 4.0;

    // sound emitter
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Mesh::from(shape::UVSphere {
                radius: 0.2,
                ..default()
            })),
            material: materials.add(Color::BLUE.into()),
            transform: Transform::from_xyz(0.0, 0.0, 0.0),
            ..default()
        },
        Emitter::default(),
        AudioBundle {
            source: asset_server.load("sounds/Windless Slopes.ogg"),
            settings: PlaybackSettings::LOOP.with_spatial(true),
        },
    ));

    let listener = SpatialListener::new(gap);
    commands
        .spawn((SpatialBundle::default(), listener.clone()))
        .with_children(|parent| {
            // left ear indicator
            parent.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.2 })),
                material: materials.add(Color::RED.into()),
                transform: Transform::from_translation(listener.left_ear_offset),
                ..default()
            });

            // right ear indicator
            parent.spawn(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube { size: 0.2 })),
                material: materials.add(Color::GREEN.into()),
                transform: Transform::from_translation(listener.right_ear_offset),
                ..default()
            });
        });

    // light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // example instructions
    commands.spawn(
        TextBundle::from_section(
            "Up/Down/Left/Right: Move Listener\nSpace: Toggle Emitter Movement",
            TextStyle {
                font_size: 20.0,
                ..default()
            },
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );

    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Component, Default)]
struct Emitter {
    stopped: bool,
}

fn update_positions(
    time: Res<Time>,
    mut emitters: Query<(&mut Transform, &mut Emitter), With<Emitter>>,
    keyboard: Res<Input<KeyCode>>,
) {
    for (mut emitter_transform, mut emitter) in emitters.iter_mut() {
        if keyboard.just_pressed(KeyCode::Space) {
            emitter.stopped = !emitter.stopped;
        }

        if !emitter.stopped {
            emitter_transform.translation.x = time.elapsed_seconds().sin() * 3.0;
            emitter_transform.translation.z = time.elapsed_seconds().cos() * 3.0;
        }
    }
}

fn update_listener(
    keyboard: Res<Input<KeyCode>>,
    time: Res<Time>,
    mut listeners: Query<&mut Transform, With<SpatialListener>>,
) {
    let mut transform = listeners.single_mut();

    let speed = 2.;

    if keyboard.pressed(KeyCode::Right) {
        transform.translation.x += speed * time.delta_seconds();
    }
    if keyboard.pressed(KeyCode::Left) {
        transform.translation.x -= speed * time.delta_seconds();
    }
    if keyboard.pressed(KeyCode::Down) {
        transform.translation.z += speed * time.delta_seconds();
    }
    if keyboard.pressed(KeyCode::Up) {
        transform.translation.z -= speed * time.delta_seconds();
    }
}
