//! This example illustrates how to load and play an audio file, and control where the sounds seems to come from.
use bevy::{prelude::*, sprite::MaterialMesh2dBundle};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_positions)
        .run();
}

/// Spatial audio uses the distance to attenuate the sound volume. In 2D with the default camera, 1 pixel is 1 unit of distance,
/// so we use a scale so that 100 pixels is 1 unit of distance for audio.
const AUDIO_SCALE: f32 = 100.0;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
    audio_sinks: Res<Assets<SpatialAudioSink>>,
) {
    // Space between the two ears
    let gap = 400.0;

    let music = asset_server.load("sounds/Windless Slopes.ogg");
    let handle = audio_sinks.get_handle(audio.play_spatial_with_settings(
        music,
        PlaybackSettings::LOOP,
        Transform::IDENTITY,
        gap / AUDIO_SCALE,
        Vec3::ZERO,
    ));
    commands.insert_resource(AudioController(handle));

    // left ear
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::RED,
            custom_size: Some(Vec2::splat(20.0)),
            ..default()
        },
        transform: Transform::from_xyz(-gap / 2.0, 0.0, 0.0),
        ..default()
    });

    // right ear
    commands.spawn(SpriteBundle {
        sprite: Sprite {
            color: Color::GREEN,
            custom_size: Some(Vec2::splat(20.0)),
            ..default()
        },
        transform: Transform::from_xyz(gap / 2.0, 0.0, 0.0),
        ..default()
    });

    // sound emitter
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(15.0).into()).into(),
            material: materials.add(ColorMaterial::from(Color::BLUE)),
            transform: Transform::from_translation(Vec3::new(0.0, 50.0, 0.0)),
            ..default()
        },
        Emitter,
    ));

    // camera
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component)]
struct Emitter;

#[derive(Resource)]
struct AudioController(Handle<SpatialAudioSink>);

fn update_positions(
    audio_sinks: Res<Assets<SpatialAudioSink>>,
    music_controller: Res<AudioController>,
    time: Res<Time>,
    mut emitter: Query<&mut Transform, With<Emitter>>,
) {
    if let Some(sink) = audio_sinks.get(&music_controller.0) {
        let mut emitter_transform = emitter.single_mut();
        emitter_transform.translation.x = time.elapsed_seconds().sin() * 500.0;
        sink.set_emitter_position(emitter_transform.translation / AUDIO_SCALE);
    }
}
