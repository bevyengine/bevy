//! This example illustrates how to use enum-based nested [`States`] for high-level app control flow.
//! States are a powerful but intuitive tool for controlling which logic runs when.
//! You can have multiple independent states, and the `entering!` and `exiting!` macros
//! can be used to great effect to ensure that you handle setup and teardown appropriately with a
//! variety of states.
//!
//! In this case, we're transitioning from a `Menu` state to an `InGame` state, which can be
//! paused or not paused. When in game, we can move the bevy logo around with the arrow keys,
//! and invert the movement by holding the shift key.

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
        // You can achieve the same result using the `Entering` schedule & conditions
        .add_systems(
            Entering,
            setup_paused.run_if(entering(AppState::InGame(GameState::Paused))),
        )
        // However, we can also use the `Entering` schedule with pattern matching
        // to let us ensure the `setup_game` system runs whenever we enter the "InGame" state
        // regardless of whether the game is paused or not!
        // Instead of calling the `entering` function, we are using the `entering!` macro and passing in the
        // state type and the pattern we want to match
        .add_systems(Entering, setup_game.run_if(entering!(AppState, InGame(_))))
        // You can also pass in a closure
        .add_systems(
            Entering,
            setup_in_game_ui.run_if(entering!(AppState, |state: &AppState| state
                == &AppState::InGame(GameState::Running { inverted: true })
                || state == &AppState::InGame(GameState::Running { inverted: false }))),
        )
        // We can also use `Exiting` to run the system whenever we leave a state with the help of pattern matching
        //
        // By default, pattern matching systems only run if the result of the match changes - so when exiting, it'll
        // only run if the next state doesn't match the pattern.
        //
        // However, here we want the system to run in a more complex way:
        // - if we are leaving the "GameState::Running" substate completely (so not when moving between GameState::Running { inverted: true} and GameState::Running { inverted: false })
        // - whenever we leave any other state (AppState::Manu or GameState::Paused)
        //
        // To do so, we provide the `exiting!` macro a sequence of conditions - the first works just like any other pattern match, but the second has the
        // `every` key word at the start, meaning it will run whenever we exit a state that matches the pattern - regardless of the next state.
        //
        // The first pattern is checked first - and if the previous state doesn't match it we skip to the next pattern. However, if it does - we check whether the
        // next state also matches and return false if it does.
        .add_systems(
            Exiting,
            cleanup_ui.run_if(exiting!(AppState, InGame(GameState::Running { .. }), every _)),
        )
        // We can also use all the same options with the "state_matches!" macro
        .add_systems(Update, menu.run_if(state_matches!(AppState::Menu)))
        .add_systems(
            Update,
            change_color.run_if(state_matches!(AppState, InGame(GameState::Running { .. }))),
        )
        .add_systems(
            Update,
            change_color.run_if(state_matches!(AppState::InGame(GameState::Paused))),
        )
        .add_systems(
            Update,
            invert_movement.run_if(state_matches!(AppState, InGame(GameState::Running { .. }))),
        )
        // And of course, you can still use the normal `in_state` value-based option if it works for your needs
        .add_systems(
            Update,
            movement.run_if(in_state(AppState::InGame(GameState::Running {
                inverted: false,
            }))),
        )
        .add_systems(
            Update,
            inverted_movement.run_if(in_state(AppState::InGame(GameState::Running {
                inverted: true,
            }))),
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
    Paused,
    Running {
        inverted: bool,
    },
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
                GameState::Running { .. } => GameState::Paused,
                GameState::Paused => GameState::default(),
            }),
            _ => state,
        });
    }
}

fn invert_movement(input: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if input.just_pressed(KeyCode::ShiftLeft) {
        next_state.set(AppState::InGame(GameState::Running { inverted: true }));
    }
    if input.just_released(KeyCode::ShiftLeft) {
        next_state.set(AppState::InGame(GameState::Running { inverted: false }));
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
