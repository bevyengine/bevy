//! This example illustrates how to use [`States`] for high-level app control flow.
//! States are a powerful but intuitive tool for controlling which logic runs when.
//! You can have multiple independent states, and the [`OnEnter`] and [`OnExit`] schedules
//! can be used to great effect to ensure that you handle setup and teardown appropriately.
//!
//! In this case, we're transitioning from a `Menu` state to an `InGame` state, which can be
//! paused or not paused.

// This lint usually gives bad advice in the context of Bevy -- hiding complex queries behind
// type aliases tends to obfuscate code while offering no improvement in code cleanliness.
#![allow(clippy::type_complexity)]

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
        // The `OnEnter` struct can accept any valid state object, not just enums
        .add_systems(OnEnter(AppState::InGame(GameState::Paused)), setup_paused)
        // In addition to `OnEnter` and `OnExit`, you can run systems any other schedule as well.
        // To do so, you will want to add the `in_state()` run condition, which will check
        // if we're in the correct state every time the schedule runs. In this case - that's every frame.
        .add_systems(Update, menu.run_if(in_state(AppState::Menu)))
        // But that is not all - we can use pattern matching with the `on_enter! macro
        // Which lets us run the system whenever we enter a matching state from one that doesn't maatch
        .add_systems(on_enter!(AppState, InGame { .. }), setup_game)
        .add_systems(
            on_enter!(AppState, InGame(GameState::Running(_))),
            setup_in_game_ui,
        )
        // Both `on_enter!` and `on_exit` also have `_strict` versions, which will match whenever
        // we enter/exit a matching system regardless if the previous/next system matched as well.
        // As a result, this system will run on every state transition, because every state matches
        // the pattern. If it were not strict, this system would never run.
        .add_systems(
            on_exit!(AppState, |to: &AppState, from: Option<&AppState>| {
                match to {
                    AppState::Menu => true,
                    AppState::InGame(g) => match g {
                        GameState::Running(_) => {
                            !matches!(from, Some(AppState::InGame(GameState::Running(_))))
                        }
                        GameState::Paused => true,
                    },
                }
            }),
            cleanup_ui,
        )
        // You can also use pattern matching when in a state, using the `in_state!` macro
        // This works just like the `in_state()` function, but relies on pattern matching rather than
        // strict equality.
        .add_systems(
            Update,
            change_color.run_if(state_matches!(AppState, InGame { .. })),
        )
        .add_systems(
            Update,
            invert_movement.run_if(state_matches!(AppState, InGame(GameState::Running(_)))),
        )
        .add_systems(
            Update,
            movement.run_if(in_state(AppState::InGame(GameState::Running(
                MovementState::Normal,
            )))),
        )
        .add_systems(
            Update,
            inverted_movement.run_if(in_state(AppState::InGame(GameState::Running(
                MovementState::Inverted,
            )))),
        )
        .run();
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Menu,
    InGame(GameState),
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum GameState {
    Running(MovementState),
    Paused,
}

impl Default for GameState {
    fn default() -> Self {
        GameState::Running(MovementState::Normal)
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash)]
enum MovementState {
    #[default]
    Normal,
    Inverted,
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

fn setup_game(mut commands: Commands, asset_server: Res<AssetServer>) {
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
                GameState::Running(_) => GameState::Paused,
                GameState::Paused => GameState::default(),
            }),
            _ => state,
        });
    }
}

fn invert_movement(input: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if input.just_pressed(KeyCode::ShiftLeft) {
        next_state.set(AppState::InGame(GameState::Running(
            MovementState::Inverted,
        )));
    }
    if input.just_released(KeyCode::ShiftLeft) {
        next_state.set(AppState::InGame(GameState::Running(MovementState::Normal)));
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
