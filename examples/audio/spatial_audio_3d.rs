//! This example illustrates how to load and play an audio file, and control where the sounds seems to come from.
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_positions)
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
        Emitter,
        SpatialAudioBundle {
            source: asset_server.load("sounds/Windless Slopes.ogg"),
            settings: PlaybackSettings::LOOP,
            spatial: SpatialSettings::new(Transform::IDENTITY, gap, Vec3::ZERO),
        },
    ));

    // left ear
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.2 })),
        material: materials.add(Color::RED.into()),
        transform: Transform::from_xyz(-gap / 2.0, 0.0, 0.0),
        ..default()
    });

    // right ear
    commands.spawn(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Cube { size: 0.2 })),
        material: materials.add(Color::GREEN.into()),
        transform: Transform::from_xyz(gap / 2.0, 0.0, 0.0),
        ..default()
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
    // camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 5.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}

#[derive(Component)]
struct Emitter;

fn update_positions(
    time: Res<Time>,
    mut emitters: Query<(&mut Transform, Option<&SpatialAudioSink>), With<Emitter>>,
) {
    for (mut emitter_transform, sink) in emitters.iter_mut() {
        emitter_transform.translation.x = time.elapsed_seconds().sin() * 3.0;
        emitter_transform.translation.z = time.elapsed_seconds().cos() * 3.0;
        if let Some(sink) = &sink {
            sink.set_emitter_position(emitter_transform.translation);
        }
    }
}
