//! This example illustrates how to load and play an audio file, and control where the sounds seems to come from.
use bevy::{
    audio::{AudioPlugin, SpatialScale},
    prelude::*,
    sprite::MaterialMesh2dBundle,
};

/// Spatial audio uses the distance to attenuate the sound volume. In 2D with the default camera,
/// 1 pixel is 1 unit of distance, so we use a scale so that 100 pixels is 1 unit of distance for
/// audio.
const AUDIO_SCALE: f32 = 1. / 100.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(AudioPlugin {
            spatial_scale: SpatialScale::new_2d(AUDIO_SCALE),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, update_positions)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Space between the two ears
    let gap = 400.0;

    // sound emitter
    commands.spawn((
        MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::new(15.0).into()).into(),
            material: materials.add(ColorMaterial::from(Color::BLUE)),
            transform: Transform::from_translation(Vec3::new(0.0, 50.0, 0.0)),
            ..default()
        },
        Emitter,
        AudioBundle {
            source: asset_server.load("sounds/Windless Slopes.ogg"),
            settings: PlaybackSettings::LOOP.with_spatial(true),
        },
    ));

    let listener = SpatialListener::new(gap);
    commands
        .spawn((SpatialBundle::default(), listener.clone()))
        .with_children(|parent| {
            // left ear
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::RED,
                    custom_size: Some(Vec2::splat(20.0)),
                    ..default()
                },
                transform: Transform::from_xyz(-gap / 2.0, 0.0, 0.0),
                ..default()
            });

            // right ear
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: Color::GREEN,
                    custom_size: Some(Vec2::splat(20.0)),
                    ..default()
                },
                transform: Transform::from_xyz(gap / 2.0, 0.0, 0.0),
                ..default()
            });
        });

    // camera
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component)]
struct Emitter;

fn update_positions(time: Res<Time>, mut emitters: Query<&mut Transform, With<Emitter>>) {
    for mut emitter_transform in emitters.iter_mut() {
        emitter_transform.translation.x = time.elapsed_seconds().sin() * 500.0;
    }
}
