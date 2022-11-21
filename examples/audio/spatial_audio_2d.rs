//! This example illustrates how to load and play an audio file, and control where the sounds seems to come from.
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(update_positions)
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    audio: Res<Audio>,
    audio_sinks: Res<Assets<SpatialAudioSink>>,
) {
    let gap = 4.0;

    let music = asset_server.load("sounds/Windless Slopes.ogg");
    let handle = audio_sinks.get_handle(audio.play_spatial_with_settings(
        music,
        PlaybackSettings::LOOP,
        Transform::IDENTITY,
        gap,
        Vec3::ZERO,
    ));
    commands.insert_resource(AudioController(handle));

    // Putting the visual presentation in a parent with a big scale as sound attenuation happens quite fast so distance are kept small
    commands
        .spawn(SpatialBundle {
            transform: Transform::from_scale(Vec3::splat(100.0)),
            ..default()
        })
        .with_children(|parent| {
            // left ear
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::RED,
                    custom_size: Some(Vec2::splat(0.2)),
                    ..default()
                },
                transform: Transform::from_xyz(-gap / 2.0, 0.0, 0.0),
                ..default()
            });

            // right ear
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::GREEN,
                    custom_size: Some(Vec2::splat(0.2)),
                    ..default()
                },
                transform: Transform::from_xyz(gap / 2.0, 0.0, 0.0),
                ..default()
            });

            // sound emitter
            parent.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: Color::BLUE,
                        custom_size: Some(Vec2::new(0.3, 0.3)),
                        ..default()
                    },
                    transform: Transform::from_xyz(0.0, 0.5, 0.0),
                    ..default()
                },
                Emitter,
            ));
        });

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
        emitter_transform.translation.x = (time.elapsed_seconds()).sin() as f32 * 5.0;
        sink.set_emitter_position(emitter_transform.translation);
    }
}
