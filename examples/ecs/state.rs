//! This example illustrates how to use [`States`] for high-level app control flow.
//! States are a powerful but intuitive tool for controlling which logic runs when.
//! You can have multiple independent states, and the [`OnEnter`] and [`OnExit`] schedules
//! can be used to great effect to ensure that you handle setup and teardown appropriately.
//!
//! In this case, we're transitioning from a `Menu` state to an `InGame` state, which can be
//! paused or not paused. When in game, we can move the bevy logo around with the arrow keys,
//! and invert the movement by holding the shift key.

// This lint usually gives bad advice in the context of Bevy -- hiding complex queries behind
// type aliases tends to obfuscate code while offering no improvement in code cleanliness.
#![allow(clippy::type_complexity)]

use bevy::ecs::schedule::{Entering, Exiting};
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Update, toggle_pause)
        .add_systems(Startup, setup)
        // You need to register a state type for it to be usable in the app.
        // This sets up all the necessary systems, schedules & resources in the background.
        .add_state::<AppState>()
        // This system runs when we enter `AppState::Menu`, during the `StateTransition` schedule.
        // All systems from the exit schedule of the state we're leaving are run first,
        // and then all systems from the enter schedule of the state we're entering are run second.
        .add_systems(OnEnter(AppState::Menu), setup_menu)
        // The `OnEnter` struct can accept any valid state, including nested enums
        .add_systems(OnEnter(AppState::InGame(GameState::Paused)), setup_paused)
        // This will run every time we wnter the "AppState::InGame(GameState::Paused)", meaning it will
        // also run whenever we pause the game. This is something we need to take into account within the function
        // We are setting up the game here because we move directly from "AppState::Menu" to "AppState::InGame(GameState::Paused)".
        // If we were to change that, we would have to change this as well.
        .add_systems(OnEnter(AppState::InGame(GameState::Paused)), setup_game)
        .add_systems(
            OnEnter(AppState::InGame(GameState::Running)),
            setup_in_game_ui,
        )
        // We can also uise `OnExit` to run the system whenever we leave a state.
        // Note that, just like `OnEnter` (and `in_state` below), `OnExit` relies on the state's
        // `Eq` implementation to determine whether it should run or not, so if we want to run
        // a system in multiple situations, we need to add it to each schedule individually.
        // The Nested State & Sturct State examples show a different approach.
        .add_systems(OnExit(AppState::Menu), cleanup_ui)
        .add_systems(OnExit(AppState::InGame(GameState::Running)), cleanup_ui)
        .add_systems(OnExit(AppState::InGame(GameState::Paused)), cleanup_ui)
        // In addition to `OnEnter` and `OnExit`, you can run systems any other schedule as well.
        // To do so, you will want to add the `in_state()` run condition, which will check
        // if we're in the correct state every time the schedule runs. In this case - that's every frame.
        .add_systems(Update, menu.run_if(in_state(AppState::Menu)))
        .add_systems(
            Update,
            change_color.run_if(in_state(AppState::InGame(GameState::Running))),
        )
        .add_systems(
            Update,
            change_color.run_if(in_state(AppState::InGame(GameState::Paused))),
        )
        .add_systems(
            Update,
            invert_movement.run_if(in_state(AppState::InGame(GameState::Running))),
        )
        // We can also have more than one state type set up in an app.
        // In this case, we are adding a Struct as our state type, instead of an enum.
        .add_state::<MovementState>()
        // We can also use `in_state` conditions referring to multiple states on a single system!
        .add_systems(
            Update,
            movement.run_if(
                in_state(AppState::InGame(GameState::Running))
                    .and_then(in_state(MovementState { inverted: false })),
            ),
        )
        .add_systems(
            Update,
            inverted_movement.run_if(
                in_state(AppState::InGame(GameState::Running))
                    .and_then(in_state(MovementState { inverted: true })),
            ),
        )
        .run();
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Menu,
    InGame(GameState),
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
enum GameState {
    #[default]
    Running,
    Paused,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, States, Default)]
struct MovementState {
    inverted: bool,
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);
const LABEL: Color = Color::rgba(0.0, 0.0, 0.0, 0.7);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn setup_menu(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                // center button
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(ButtonBundle {
                    style: Style {
                        width: Val::Px(150.),
                        height: Val::Px(65.),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: NORMAL_BUTTON.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Play",
                        TextStyle {
                            font_size: 40.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                });
        });
}

fn menu(
    mut next_state: ResMut<NextState<AppState>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                // One way to set the next state is to set the full state value, like so
                next_state.set(AppState::InGame(GameState::Paused));
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
}

fn setup_game(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    has_set_up: Query<Entity, With<Sprite>>,
) {
    // This allows us to check whether we already ran the game setup, which will be the case if we are pausing.
    if !has_set_up.is_empty() {
        return;
    }
    commands.spawn(SpriteBundle {
        texture: asset_server.load("branding/icon.png"),
        ..default()
    });
}

const SPEED: f32 = 100.0;
fn movement(
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Sprite>>,
) {
    for mut transform in &mut query {
        let mut direction = Vec3::ZERO;
        if input.pressed(KeyCode::Left) {
            direction.x -= 1.0;
        }
        if input.pressed(KeyCode::Right) {
            direction.x += 1.0;
        }
        if input.pressed(KeyCode::Up) {
            direction.y += 1.0;
        }
        if input.pressed(KeyCode::Down) {
            direction.y -= 1.0;
        }

        if direction != Vec3::ZERO {
            transform.translation += direction.normalize() * SPEED * time.delta_seconds();
        }
    }
}

fn inverted_movement(
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Sprite>>,
) {
    for mut transform in &mut query {
        let mut direction = Vec3::ZERO;
        if input.pressed(KeyCode::Left) {
            direction.x += 1.0;
        }
        if input.pressed(KeyCode::Right) {
            direction.x -= 1.0;
        }
        if input.pressed(KeyCode::Up) {
            direction.y -= 1.0;
        }
        if input.pressed(KeyCode::Down) {
            direction.y += 1.0;
        }

        if direction != Vec3::ZERO {
            transform.translation += direction.normalize() * SPEED * time.delta_seconds();
        }
    }
}

fn change_color(time: Res<Time>, mut query: Query<&mut Sprite>) {
    for mut sprite in &mut query {
        sprite
            .color
            .set_b((time.elapsed_seconds() * 0.5).sin() + 2.0);
    }
}

fn toggle_pause(input: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if input.just_pressed(KeyCode::Escape) {
        // Alternatively, you provide next_state with a setter function, which will take the current state, and output the new state, allowing for some degree of update-in-place
        next_state.setter(|state| match &state {
            AppState::InGame(state) => AppState::InGame(match state {
                GameState::Running => GameState::Paused,
                GameState::Paused => GameState::default(),
            }),
            _ => state,
        });
    }
}

fn invert_movement(input: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<MovementState>>) {
    if input.just_pressed(KeyCode::ShiftLeft) {
        next_state.set(MovementState { inverted: true });
    }
    if input.just_released(KeyCode::ShiftLeft) {
        next_state.set(MovementState { inverted: false });
    }
}

fn setup_paused(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                // center button
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Auto,
                        height: Val::Auto,
                        padding: UiRect::all(Val::Px(10.)),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: LABEL.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Paused... Press Esc to Resume",
                        TextStyle {
                            font_size: 40.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                });
        });
}

fn setup_in_game_ui(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                // center button
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Start,
                align_items: AlignItems::Start,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        width: Val::Auto,
                        height: Val::Auto,
                        padding: UiRect::all(Val::Px(10.)),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        ..default()
                    },
                    background_color: LABEL.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Press Esc to Pause",
                        TextStyle {
                            font_size: 25.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                    parent.spawn(TextBundle::from_section(
                        "Hold Left Shift to invert movement",
                        TextStyle {
                            font_size: 25.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                });
        });
}

fn cleanup_ui(mut commands: Commands, roots: Query<Entity, (With<Node>, Without<Parent>)>) {
    for root in &roots {
        commands.entity(root).despawn_recursive();
    }
}
