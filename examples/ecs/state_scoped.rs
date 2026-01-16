//! Shows how to spawn entities that are automatically despawned either when
//! entering or exiting specific game states.
//!
//! This pattern is useful for managing menus, levels, or other state-specific
//! content that should only exist during certain states.
//!
//! If the entity was already despawned then no error will be logged. This means
//! that you don't have to worry about duplicate [`DespawnOnExit`] and
//! [`DespawnOnEnter`] components deep in your hierarchy.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<GameState>()
        .add_systems(Startup, setup_camera)
        .add_systems(OnEnter(GameState::A), on_a_enter)
        .add_systems(OnEnter(GameState::B), on_b_enter)
        .add_systems(OnExit(GameState::A), on_a_exit)
        .add_systems(OnExit(GameState::B), on_b_exit)
        .add_systems(Update, toggle)
        .insert_resource(TickTock(Timer::from_seconds(1.0, TimerMode::Repeating)))
        .run();
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default, States)]
enum GameState {
    #[default]
    A,
    B,
}

#[derive(Resource)]
struct TickTock(Timer);

fn on_a_enter(mut commands: Commands) {
    info!("on_a_enter");
    commands.spawn((
        DespawnOnExit(GameState::A),
        Text::new("Game is in state 'A'"),
        TextFont {
            font_size: 33.0,
            ..default()
        },
        TextColor(Color::srgb(0.5, 0.5, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            left: px(0),
            ..default()
        },
        (children![DespawnOnExit(GameState::A)]),
    ));
}

fn on_a_exit(mut commands: Commands) {
    info!("on_a_exit");
    commands.spawn((
        DespawnOnEnter(GameState::A),
        Text::new("Game state 'A' will be back in 1 second"),
        TextFont {
            font_size: 33.0,
            ..default()
        },
        TextColor(Color::srgb(0.5, 0.5, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: px(0),
            left: px(500),
            ..default()
        },
        // You can apply this even when the parent has a state scoped component.
        // It is unnecessary but in complex hierarchies it saves you from having to
        // mentally track which components are found at the top level.
        (children![DespawnOnEnter(GameState::A)]),
    ));
}

fn on_b_enter(mut commands: Commands) {
    info!("on_b_enter");
    commands.spawn((
        DespawnOnExit(GameState::B),
        Text::new("Game is in state 'B'"),
        TextFont {
            font_size: 33.0,
            ..default()
        },
        TextColor(Color::srgb(0.5, 0.5, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: px(50),
            left: px(0),
            ..default()
        },
        (children![DespawnOnExit(GameState::B)]),
    ));
}

fn on_b_exit(mut commands: Commands) {
    info!("on_b_exit");
    commands.spawn((
        DespawnOnEnter(GameState::B),
        Text::new("Game state 'B' will be back in 1 second"),
        TextFont {
            font_size: 33.0,
            ..default()
        },
        TextColor(Color::srgb(0.5, 0.5, 1.0)),
        Node {
            position_type: PositionType::Absolute,
            top: px(50),
            left: px(500),
            ..default()
        },
        (children![DespawnOnEnter(GameState::B)]),
    ));
}

fn setup_camera(mut commands: Commands) {
    commands.spawn(Camera3d::default());
}

fn toggle(
    time: Res<Time>,
    mut timer: ResMut<TickTock>,
    state: Res<State<GameState>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if !timer.0.tick(time.delta()).is_finished() {
        return;
    }
    *next_state = match state.get() {
        GameState::A => NextState::Pending(GameState::B),
        GameState::B => NextState::Pending(GameState::A),
    }
}
