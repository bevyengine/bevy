//! An implementation of the game "Flappy Bird".

use std::time::Duration;

use bevy::math::{
    bounding::{Aabb2d, BoundingCircle, IntersectsVolume},
    ops::exp,
};
use bevy::prelude::*;
use rand::Rng;

#[derive(Resource, Reflect)]
struct Settings {
    background_color: Color,

    /// Timer spawning a pipe each time it finishes
    pipe_timer_duration: Duration,

    /// Movement speed of the pipes
    pipe_speed: f32,

    /// The size of each pipe rectangle
    pipe_size: Vec2,

    /// How large the gap is between the pipes
    gap_height: f32,

    /// Gravity applied to the bird
    gravity: f32,

    /// Size of the bird sprite
    bird_size: f32,

    /// Acceleration the bird is set to on a flap
    flap_power: f32,

    /// Horizontal position of the bird
    bird_position: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            background_color: Color::srgb(0.9, 0.9, 0.9),
            pipe_timer_duration: Duration::from_millis(2000),
            pipe_speed: 200.,
            pipe_size: Vec2::new(100., 500.),
            gap_height: 300.,
            gravity: 700.,
            bird_size: 100.,
            flap_power: 400.,
            bird_position: -500.,
        }
    }
}

#[derive(Component)]
struct Bird;

#[derive(Component)]
struct Pipe;

#[derive(Component)]
struct PipeMarker;

/// Marker component for the text displaying the score
#[derive(Component)]
struct ScoreText;

/// This resource tracks the game's score
#[derive(Resource, Deref, DerefMut)]
struct Score(usize);

/// 2-dimensional velocity
#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

/// Timer that determines when new pipes are spawned
#[derive(Resource, Deref, DerefMut)]
struct PipeTimer(Timer);

/// The size of the window at the start of the game
///
/// Handling resizing while the game is playing is quite hard, so we ignore that
#[derive(Resource, Deref, DerefMut)]
struct WindowSize(Vec2);

/// Event emitted when the bird touches the edges or a pipe
#[derive(Event, Default)]
struct CollisionEvent;

/// Event emitted when a new pipe should be spawned
#[derive(Event, Default)]
struct SpawnPipeEvent;

/// Sound that should be played when a pipe is passed
#[derive(Resource, Deref)]
struct ScoreSound(Handle<AudioSource>);

fn main() {
    let settings = Settings::default();
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (set_window_size, setup))
        .add_systems(
            FixedUpdate,
            (
                reset,
                add_pipes,
                spawn_pipe,
                flap,
                apply_gravity,
                apply_velocity,
                check_collisions,
                increase_score,
                remove_pipes,
            ),
        )
        .insert_resource(Score(0))
        .insert_resource(ClearColor(settings.background_color))
        .insert_resource(PipeTimer(Timer::new(
            settings.pipe_timer_duration,
            TimerMode::Repeating,
        )))
        .insert_resource(WindowSize(Vec2::ZERO))
        .insert_resource(settings)
        .add_event::<CollisionEvent>()
        .add_event::<SpawnPipeEvent>()
        .run();
}

/// Set up the camera and score UI
fn setup(
    mut commands: Commands,
    mut collision_events: EventWriter<CollisionEvent>,
    asset_server: Res<AssetServer>,
) {
    commands.spawn(Camera2d);

    // TODO: Replace with a custom sound, or rename file
    let score_sound = asset_server.load("sounds/breakout_collision.ogg");
    commands.insert_resource(ScoreSound(score_sound));

    // Spawn the score UI.
    commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Start,
            justify_content: JustifyContent::Center,
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        children![(
            ScoreText,
            Text::new("0"),
            TextFont {
                font_size: 66.0,
                ..default()
            },
            TextColor(Color::srgb(0.3, 0.3, 0.9)),
        )],
    ));

    // Create a collision event to trigger a reset
    collision_events.write_default();
}

/// Clear everything and put everything to its start state
fn reset(
    mut commands: Commands,
    mut timer: ResMut<PipeTimer>,
    mut score: ResMut<Score>,
    mut collision_events: EventReader<CollisionEvent>,
    mut spawn_pipe_events: EventWriter<SpawnPipeEvent>,
    mut score_text: Single<&mut Text, With<ScoreText>>,
    to_remove: Query<Entity, Or<(With<Bird>, With<Pipe>, With<PipeMarker>)>>,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
) {
    if collision_events.is_empty() {
        return;
    }

    collision_events.clear();

    // Remove any entities left over from the previous game (if any)
    for ent in to_remove {
        commands.entity(ent).despawn();
    }

    // Set the score to 0
    score.0 = 0;
    score_text.0 = 0.to_string();

    // Spawn a new bird
    commands.spawn((
        Bird,
        Sprite {
            image: asset_server.load("branding/icon.png"),
            custom_size: Some(Vec2::splat(settings.bird_size)),
            ..default()
        },
        Transform::from_xyz(settings.bird_position, 0., 0.),
        Velocity(Vec2::new(0., settings.flap_power)),
    ));

    timer.reset();
    spawn_pipe_events.write_default();
}

fn set_window_size(window: Single<&mut Window>, mut window_size: ResMut<WindowSize>) {
    window_size.0 = Vec2::new(window.resolution.width(), window.resolution.height());
}

/// Flap on a spacebar or left mouse button press
fn flap(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    mut bird_velocity: Single<&mut Velocity, With<Bird>>,
    settings: Res<Settings>,
) {
    if keyboard_input.pressed(KeyCode::Space) || mouse_input.pressed(MouseButton::Left) {
        bird_velocity.y = settings.flap_power;
    }
}

/// Apply gravity to the bird and set its rotation
fn apply_gravity(
    mut bird: Single<(&mut Transform, &mut Velocity), With<Bird>>,
    time: Res<Time>,
    settings: Res<Settings>,
) {
    /// The logistic function, which is an example of a sigmoid function
    fn logistic(x: f32) -> f32 {
        1. / (1. + exp(-x))
    }

    bird.1.y -= settings.gravity * time.delta_secs();

    // We determine the rotation based on the y-component of the velocity.
    // This is tweaked such that a velocity of 100 is pretty much a 90 degree
    // rotation. We take the output of the sigmoid function, which goes from
    // 0 to 1 and stretch it to -1 to 1. Then we multiply with PI/2 to get
    // a rotation in radians.
    let rotation = std::f32::consts::PI / 2. * 2. * (logistic(bird.1.y / 600.) - 0.5);
    bird.0.rotation = Quat::from_rotation_z(rotation);
}

/// Apply velocity to everything with a `Velocity` component
fn apply_velocity(mut query: Query<(&mut Transform, &Velocity)>, time: Res<Time>) {
    for (mut transform, velocity) in &mut query {
        transform.translation.x += velocity.x * time.delta_secs();
        transform.translation.y += velocity.y * time.delta_secs();
    }
}

/// Check for collision with the borders of the window and the pipes
fn check_collisions(
    mut collision_events: EventWriter<CollisionEvent>,
    bird: Single<&Transform, With<Bird>>,
    pipes: Query<&Transform, With<Pipe>>,
    window_size: Res<WindowSize>,
    settings: Res<Settings>,
) {
    if bird.translation.y.abs() > window_size.y / 2. {
        collision_events.write_default();
        return;
    }

    let bird_collider = BoundingCircle::new(bird.translation.truncate(), settings.bird_size / 2.);
    for pipe in pipes {
        let pipe_collider = Aabb2d::new(pipe.translation.truncate(), settings.pipe_size / 2.);
        if bird_collider.intersects(&pipe_collider) {
            collision_events.write_default();
            return;
        }
    }
}

/// Add a pipe each time the timer finishes
fn add_pipes(
    mut timer: ResMut<PipeTimer>,
    mut events: EventWriter<SpawnPipeEvent>,
    time: Res<Time>,
) {
    timer.tick(time.delta());

    if timer.finished() {
        events.write_default();
    }
}

fn spawn_pipe(
    mut events: EventReader<SpawnPipeEvent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    window_size: Res<WindowSize>,
    settings: Res<Settings>,
) {
    if events.is_empty() {
        return;
    }
    events.clear();

    let color = Color::BLACK;
    let size = settings.pipe_size;
    let shape = meshes.add(Rectangle::new(size.x, size.y));

    let mut rng = rand::thread_rng();
    let gap_offset: i64 = rng.gen_range(-200..=200);
    let gap_offset: f32 = gap_offset as f32;

    let pipe_offset = size.y / 2. + settings.gap_height / 2.;

    let pipe_location = window_size.x / 2. + size.x / 2.;

    // We first spawn in invisible marker that will increase the score once
    // it passes the bird position and then despawns. This assures that each
    // pipe is counted once.
    commands.spawn((
        PipeMarker,
        Transform::from_xyz(pipe_location, 0.0, 0.0),
        Velocity(Vec2::new(-settings.pipe_speed, 0.)),
    ));

    // bottom pipe
    commands.spawn((
        Pipe,
        Mesh2d(shape.clone()),
        MeshMaterial2d(materials.add(color)),
        Transform::from_xyz(pipe_location, pipe_offset + gap_offset, 0.0),
        Velocity(Vec2::new(-settings.pipe_speed, 0.)),
    ));

    // top pipe
    commands.spawn((
        Pipe,
        Mesh2d(shape),
        MeshMaterial2d(materials.add(color)),
        Transform::from_xyz(pipe_location, -pipe_offset + gap_offset, 0.0),
        Velocity(Vec2::new(-settings.pipe_speed, 0.)),
    ));
}

/// Increase the score every time a pipe marker passes the bird
fn increase_score(
    mut commands: Commands,
    mut marker_query: Query<(Entity, &mut Transform), With<PipeMarker>>,
    mut text_query: Query<&mut Text, With<ScoreText>>,
    mut score: ResMut<Score>,
    sound: Res<ScoreSound>,
    settings: Res<Settings>,
) {
    for (entity, transform) in &mut marker_query {
        if transform.translation.x < settings.bird_position {
            commands.entity(entity).despawn();
            score.0 += 1;
            text_query.single_mut().unwrap().0 = score.0.to_string();
            commands.spawn((AudioPlayer(sound.clone()), PlaybackSettings::DESPAWN));
        }
    }
}

/// Remove pipes that have left the screen
fn remove_pipes(
    mut commands: Commands,
    mut query: Query<(Entity, &mut Transform), With<Pipe>>,
    window_size: Res<WindowSize>,
    settings: Res<Settings>,
) {
    for (entity, transform) in &mut query {
        // The entire pipe needs to have left the screen, not just its origin,
        // so we check that the right side of the pipe is off screen.
        let right_side_of_pipe = transform.translation.x + settings.pipe_size.x / 2.;
        if right_side_of_pipe < -window_size.x / 2. {
            commands.entity(entity).despawn();
        }
    }
}
