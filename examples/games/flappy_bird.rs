//! An implementation of the game "Flappy Bird".

use std::time::Duration;

use bevy::math::{
    bounding::{Aabb2d, BoundingCircle, IntersectsVolume},
    ops::exp,
};
use bevy::prelude::*;
use rand::Rng;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
enum State {
    #[default]
    MainMenu,
    InGame,
    GameOver,
}

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

#[derive(Component)]
struct StartButton;

#[derive(Component)]
struct OnMainMenu;

#[derive(Component)]
struct OnGameScreen;

#[derive(Component)]
struct OnGameOverScreen;

/// This resource tracks the game's score
#[derive(Resource, Deref, DerefMut)]
struct Score(usize);

/// 2-dimensional velocity
#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

/// Timer that determines when new pipes are spawned
#[derive(Resource, Deref, DerefMut)]
struct PipeTimer(Timer);

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
        .add_systems(Startup, setup)
        .add_systems(OnEnter(State::MainMenu), setup_main_menu)
        .add_systems(OnExit(State::MainMenu), despawn_screen::<OnMainMenu>)
        .add_systems(OnEnter(State::GameOver), setup_game_over)
        .add_systems(OnExit(State::GameOver), despawn_screen::<OnGameOverScreen>)
        .add_systems(
            Update,
            handle_start_button.run_if(in_state(State::MainMenu)),
        )
        .add_systems(
            Update,
            handle_start_button.run_if(in_state(State::GameOver)),
        )
        .add_systems(OnEnter(State::InGame), setup_game)
        .add_systems(OnExit(State::InGame), teardown_game)
        .add_systems(
            FixedUpdate,
            (
                add_pipes,
                spawn_pipe,
                flap,
                apply_gravity,
                apply_velocity,
                check_collisions,
                increase_score,
                remove_pipes,
            )
                .run_if(in_state(State::InGame)),
        )
        .insert_resource(Score(0))
        .insert_resource(ClearColor(settings.background_color))
        .insert_resource(PipeTimer(Timer::new(
            settings.pipe_timer_duration,
            TimerMode::Repeating,
        )))
        .insert_resource(settings)
        .insert_state(State::MainMenu)
        .add_event::<SpawnPipeEvent>()
        .run();
}

fn despawn_screen<T: Component>(menu: Single<Entity, With<T>>, mut commands: Commands) {
    commands.entity(*menu).despawn();
}

/// Set up the camera and score UI
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // TODO: Replace with a custom sound, or rename file
    let score_sound = asset_server.load("sounds/breakout_collision.ogg");
    commands.insert_resource(ScoreSound(score_sound));
}

fn setup_main_menu(mut commands: Commands) {
    commands.spawn((
        OnMainMenu,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            ..default()
        },
        children![
            (
                Text::new("Flipper Birb"),
                TextFont {
                    font_size: 66.0,
                    ..default()
                },
                TextColor(Color::BLACK),
            ),
            (
                Button,
                StartButton,
                Node {
                    width: Val::Px(150.0),
                    height: Val::Px(65.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                },
                BorderColor(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                children![(
                    Text::new("Start"),
                    TextFont {
                        font_size: 33.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            )
        ],
    ));
}

fn handle_start_button(
    query: Query<&Interaction, (Changed<Interaction>, With<StartButton>, With<Button>)>,
    mut next_state: ResMut<NextState<State>>,
) {
    for interaction in query {
        if *interaction == Interaction::Pressed {
            next_state.set(State::InGame);
        }
    }
}

fn setup_game_over(mut commands: Commands, score: Res<Score>) {
    commands.spawn((
        OnGameOverScreen,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            flex_direction: FlexDirection::Column,
            column_gap: Val::Px(10.0),
            ..default()
        },
        children![
            (
                Text::new("Your score was:"),
                TextFont {
                    font_size: 30.0,
                    ..default()
                },
                TextColor(Color::BLACK),
            ),
            (
                Text::new(score.to_string()),
                TextFont {
                    font_size: 120.0,
                    ..default()
                },
                TextColor(Color::BLACK),
            ),
            (
                Button,
                StartButton,
                Node {
                    width: Val::Px(250.0),
                    height: Val::Px(65.0),
                    border: UiRect::all(Val::Px(5.0)),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                },
                BorderColor(Color::BLACK),
                BorderRadius::MAX,
                BackgroundColor(Color::srgb(0.15, 0.15, 0.15)),
                children![(
                    Text::new("Try Again!"),
                    TextFont {
                        font_size: 33.0,
                        ..default()
                    },
                    TextColor(Color::srgb(0.9, 0.9, 0.9)),
                    TextShadow::default(),
                )]
            )
        ],
    ));
}

fn setup_game(
    mut score: ResMut<Score>,
    mut commands: Commands,
    mut spawn_pipe_events: EventWriter<SpawnPipeEvent>,
    mut timer: ResMut<PipeTimer>,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
) {
    // Set the score to 0
    score.0 = 0;

    // Spawn the score UI.
    commands.spawn((
        OnGameScreen,
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

/// Clear everything and put everything to its start state
fn teardown_game(
    mut commands: Commands,
    to_remove: Query<Entity, Or<(With<Bird>, With<Pipe>, With<PipeMarker>)>>,
    game_ui: Single<Entity, With<OnGameScreen>>,
) {
    // Remove any entities left over from the previous game (if any)
    for ent in to_remove {
        commands.entity(ent).despawn();
    }

    let mut ent = commands.entity(*game_ui);
    ent.despawn_related::<Children>();
    ent.despawn();
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
    mut next_state: ResMut<NextState<State>>,
    bird: Single<&Transform, With<Bird>>,
    pipes: Query<&Transform, With<Pipe>>,
    window: Single<&Window>,
    settings: Res<Settings>,
) {
    if bird.translation.y.abs() + settings.bird_size / 2. > window.resolution.height() / 2. {
        next_state.set(State::GameOver);
        return;
    }

    let bird_collider = BoundingCircle::new(bird.translation.truncate(), settings.bird_size / 2.);
    for pipe in pipes {
        let pipe_collider = Aabb2d::new(pipe.translation.truncate(), settings.pipe_size / 2.);
        if bird_collider.intersects(&pipe_collider) {
            next_state.set(State::GameOver);
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
    window: Single<&Window>,
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

    let pipe_location = window.resolution.width() / 2. + size.x / 2.;

    // We first spawn in invisible marker that will increase the score once
    // it passes the bird position and then despawns. This assures that each
    // pipe is counted once.
    commands.spawn((
        PipeMarker,
        Transform::from_xyz(pipe_location, 0.0, 0.0),
        Velocity(Vec2::new(-settings.pipe_speed, 0.)),
    ));

    // Spawn the bottom pipe
    commands.spawn((
        Pipe,
        Mesh2d(shape.clone()),
        MeshMaterial2d(materials.add(color)),
        Transform::from_xyz(pipe_location, pipe_offset + gap_offset, 0.0),
        Velocity(Vec2::new(-settings.pipe_speed, 0.)),
    ));

    // Spawn the top pipe
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
    window: Single<&Window>,
    settings: Res<Settings>,
) {
    for (entity, transform) in &mut query {
        // The entire pipe needs to have left the screen, not just its origin,
        // so we check that the right side of the pipe is off screen.
        let right_side_of_pipe = transform.translation.x + settings.pipe_size.x / 2.;
        if right_side_of_pipe < -window.resolution.width() / 2. {
            commands.entity(entity).despawn();
        }
    }
}
