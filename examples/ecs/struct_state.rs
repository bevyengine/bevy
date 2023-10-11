//! This example illustrates how to use struct based [`States`] for high-level app control flow.
//! States are a powerful but intuitive tool for controlling which logic runs when.
//! You can have multiple independent states, and the `entering!` and `exiting!` macros
//! can be used to great effect to ensure that you handle setup and teardown appropriately with a
//! variety of states.
//!
//! In this case, we're transitioning from a `Menu` state to an `InGame` state, which can be
//! paused or not paused. When in game, we can move the bevy logo around with the arrow keys,
//! and invert the movement by holding the shift key.
//!
//! The use of Structs allows for private internal fields & more fine-grained, type-safe,
//! control of state transitions.

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
        // Because our state is a complex object, it isn't worth using a value for OnEnter in this case.
        // Instead, we will stick with pattern matching
        .add_systems(
            Entering,
            setup_menu.run_if(state_matches!(AppState, AppState { in_menu: true, .. })),
        )
        .add_systems(
            Entering,
            setup_paused.run_if(state_matches!(
                AppState,
                AppState {
                    is_paused: true,
                    ..
                }
            )),
        )
        // Just like in the nested example, we still have access to closures
        .add_systems(
            Entering,
            setup_game.run_if(state_matches!(AppState, |state: &AppState| {
                println!("Test move: {state:?}");
                state.in_game.is_some()
            })),
        )
        // And we can even pass some closures directly into the `entering()` function, rather than the macro
        .add_systems(
            Entering,
            setup_in_game_ui.run_if(state_matches(|state: &AppState| state.in_game.is_some() && !state.is_paused)),
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
            cleanup_ui
                .run_if(state_matches!(AppState, |state: &AppState| state.in_game.is_some() && !state.is_paused, every _)),
        )
        // We can also use all the same options with the "state_matches!" macro
        .add_systems(
            Update,
            menu.run_if(state_matches!(AppState, AppState { in_menu: true, .. })),
        )
        .add_systems(
            Update,
            change_color.run_if(state_matches!(AppState, |state: &AppState| state
                .in_game
                .is_some())),
        )
        .add_systems(
            Update,
            invert_movement.run_if(state_matches!(
                AppState,
                AppState {
                    in_game: Some(_),
                    is_paused: false,
                    ..
                }
            )),
        )
        // And of course, you can still use the normal `in_state` value-based option if it works for your needs
        .add_systems(
            Update,
            movement.run_if(in_state(AppState {
                in_game: Some(false),
                is_paused: false,
                in_menu: false,
            })),
        )
        .add_systems(
            Update,
            inverted_movement.run_if(in_state(AppState {
                in_game: Some(true),
                is_paused: false,
                in_menu: false,
            })),
        )
        .run();
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, States)]
struct AppState {
    in_menu: bool,
    in_game: Option<bool>,
    is_paused: bool,
}

// We can manually implement default to ensure we start in a valid state
impl Default for AppState {
    fn default() -> Self {
        Self {
            in_menu: true,
            in_game: Default::default(),
            is_paused: Default::default(),
        }
    }
}

// We can then implement only the operations we want to support for our state
impl AppState {
    pub fn toggle_pause(self) -> Self {
        if self.in_game.is_some() {
            Self {
                is_paused: !self.is_paused,
                ..self
            }
        } else {
            self
        }
    }

    pub fn start_game(self) -> Self {
        if self.in_menu {
            Self {
                in_game: Some(false),
                is_paused: true,
                in_menu: false,
            }
        } else {
            self
        }
    }

    pub fn invert_movement(self) -> Self {
        if self.in_game.is_some() && !self.is_paused {
            Self {
                in_game: Some(true),
                is_paused: false,
                in_menu: false,
            }
        } else {
            self
        }
    }

    pub fn reset_movement(self) -> Self {
        if self.in_game.is_some() && !self.is_paused {
            Self {
                in_game: Some(false),
                is_paused: false,
                in_menu: false,
            }
        } else {
            self
        }
    }
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
                // Because we set up operations on AppState, we can rely on them here
                next_state.setter(|s| s.start_game());
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
        // Like above, we can use the supported operations
        next_state.setter(|state| state.toggle_pause());
    }
}

fn invert_movement(input: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if input.just_pressed(KeyCode::ShiftLeft) {
        next_state.setter(|s| s.invert_movement());
    }
    if input.just_released(KeyCode::ShiftLeft) {
        next_state.setter(|s| s.reset_movement());
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
