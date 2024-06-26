//! This example illustrates how to load and play an audio file, and control where the sounds seems to come from.
use bevy::{
    audio::{AudioPlugin, SpatialScale},
    color::palettes::css::*,
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
            default_spatial_scale: SpatialScale::new_2d(AUDIO_SCALE),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, update_emitters)
        .add_systems(Update, update_listener)
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
            mesh: meshes.add(Circle::new(15.0)).into(),
            material: materials.add(Color::from(BLUE)),
            transform: Transform::from_translation(Vec3::new(0.0, 50.0, 0.0)),
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
            // left ear
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: RED.into(),
                    custom_size: Some(Vec2::splat(20.0)),
                    ..default()
                },
                transform: Transform::from_xyz(-gap / 2.0, 0.0, 0.0),
                ..default()
            });

            // right ear
            parent.spawn(SpriteBundle {
                sprite: Sprite {
                    color: LIME.into(),
                    custom_size: Some(Vec2::splat(20.0)),
                    ..default()
                },
                transform: Transform::from_xyz(gap / 2.0, 0.0, 0.0),
                ..default()
            });
        });

    // example instructions
    commands.spawn(
        TextBundle::from_section(
            "Up/Down/Left/Right: Move Listener\nSpace: Toggle Emitter Movement",
            TextStyle::default(),
        )
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );

    // camera
    commands.spawn(Camera2dBundle::default());
}

#[derive(Component, Default)]
struct Emitter {
    stopped: bool,
}

fn update_emitters(
    time: Res<Time>,
    mut emitters: Query<(&mut Transform, &mut Emitter), With<Emitter>>,
    keyboard: Res<ButtonInput<KeyCode>>,
) {
    for (mut emitter_transform, mut emitter) in emitters.iter_mut() {
        if keyboard.just_pressed(KeyCode::Space) {
            emitter.stopped = !emitter.stopped;
        }

        if !emitter.stopped {
            emitter_transform.translation.x = time.elapsed_seconds().sin() * 500.0;
        }
    }
}

fn update_listener(
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut listeners: Query<&mut Transform, With<SpatialListener>>,
) {
    let mut transform = listeners.single_mut();

    let speed = 200.;

    if keyboard.pressed(KeyCode::ArrowRight) {
        transform.translation.x += speed * time.delta_seconds();
    }
    if keyboard.pressed(KeyCode::ArrowLeft) {
        transform.translation.x -= speed * time.delta_seconds();
    }
    if keyboard.pressed(KeyCode::ArrowUp) {
        transform.translation.y += speed * time.delta_seconds();
    }
    if keyboard.pressed(KeyCode::ArrowDown) {
        transform.translation.y -= speed * time.delta_seconds();
    }
}
