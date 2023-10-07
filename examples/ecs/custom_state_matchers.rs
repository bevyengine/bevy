//! This example builds on the other State related examples, and in particular the Struct State example,
//! to show how you can use custom state matchers to create re-usable matching sections and
//! fully enclosed modules for controlling the state itself in a type safe way.
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
        .add_state::<AppState>()
        // Because we can't really see the internals of `AppState`, we need to rely on matchers exported from
        // the `state` module. Fortunately, we can use matchers with the `entering`/`exiting`/`transitioning` functions!
        // Here we are using the `Menu` unit struct, which uses a derive to match any state with `in_menu: true`
        .add_systems(Entering, setup_menu.run_if(entering(Menu)))
        // But derived matchers aren't limited to Unit structs - they can also be fieldless enums!
        // Here we are checking if we are paused
        .add_systems(Entering, setup_paused.run_if(entering(GameState::Paused)))
        // And here we are using a different variant to see if we are in any game state
        .add_systems(Entering, setup_game.run_if(entering(GameState::Any)))
        // or that we are specifically not paused
        .add_systems(
            Entering,
            setup_in_game_ui.run_if(entering(GameState::Running)),
        )
        // Matchers are also automatically derived from certain functions
        .add_systems(Update, change_color.run_if(state_matches(in_game)))
        .add_systems(Exiting, cleanup_ui.run_if(exiting(UI)))
        .add_systems(Update, menu.run_if(state_matches(Menu)))
        .add_systems(Update, invert_movement.run_if(entering(GameState::Running)))
        // You can also implement your own fully custom state matchers with the help of a few traits
        .add_systems(
            Update,
            movement.run_if(state_matches(Movement::<Standard>::default())),
        )
        .add_systems(
            Update,
            inverted_movement.run_if(state_matches(Movement::<Inverted>::default())),
        )
        .run();
}
use self::state::*;
mod state {
    use bevy::ecs::schedule::SingleStateMatcher;
    use bevy::prelude::{StateMatcher, States};

    // The first portion is identical to the setup in the Struct State example
    // We define the state
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, States)]
    pub struct AppState {
        in_menu: bool,
        in_game: Option<bool>,
        is_paused: bool,
    }

    // Implement our default/initial state
    impl Default for AppState {
        fn default() -> Self {
            Self {
                in_menu: true,
                in_game: Default::default(),
                is_paused: Default::default(),
            }
        }
    }

    // And implement an interface for transfomring one state to another
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

    // Then, we can start implementing our matchers
    // The first matcher is a simple unit struct, built using a derive
    #[derive(StateMatcher)]
    // We need to tell it what type the state is
    #[state_type(AppState)]
    // And then we pass in the same matching syntax used in the macros
    #[matcher(AppState { in_menu: true, ..})]
    pub struct Menu;

    // For our second matcher, we have a fieldless enum
    // This can also be automatically derived
    #[derive(StateMatcher)]
    // We still need to tell it what type the state is
    #[state_type(AppState)]
    pub enum GameState {
        // But now we need to provide the matching syntax for every variant
        #[matcher(AppState { in_game: Some(_), ..})]
        Any,
        #[matcher(AppState { in_game:  Some(_), is_paused: false,..})]
        Running,
        #[matcher(AppState { in_game:  Some(_), is_paused: true,..})]
        Paused,
    }

    #[derive(StateMatcher)]
    #[state_type(AppState)]
    // As noted before, we can use the same syntax used in other macros, including multiple matching segments
    // and the every keyword
    #[matcher(|state: &AppState| state.in_game.is_some() && !state.is_paused, every _)]
    pub struct UI;

    // We can also rely on some of the automatic implementations
    // These include the actual `States` types, as well as a few different `Fn` variants
    // Which can be pre-defined functions or closures.
    //
    // The simples variant is `Fn(&S) -> bool`, but we also support:
    // `Fn(Option<&S>) -> bool`, `Fn(&S, &S) -> bool`, `Fn(&S, Option<&S>) -> bool`, ` Fn(Option<&S>, Option<&S>) -> bool
    // `Fn(&S, &S) -> MatchesStateTransition`, `Fn(&S, Option<&S>) -> MatchesStateTransition` and `Fn(Option<&S>, Option<&S>) -> MatchesStateTransition`
    pub fn in_game(s: &AppState) -> bool {
        s.in_game.is_some()
    }

    // Lastly, you can always implement your own State Matcher
    #[derive(Default)]
    pub struct Inverted;
    #[derive(Default)]
    pub struct Standard;

    pub trait MovementType {
        fn detect(movement: bool) -> bool;
    }

    impl MovementType for Inverted {
        fn detect(movement: bool) -> bool {
            movement
        }
    }

    impl MovementType for Standard {
        fn detect(movement: bool) -> bool {
            !movement
        }
    }

    // All state matchers have to be `Send + Sync + Sized + 'static`, so we need to ensure that on the type here
    #[derive(Default)]
    pub struct Movement<T: MovementType + Default + Send + Sync + 'static>(T);

    // There are then 3 options for implementing the state matcher:
    // - Implementing `SingleStateMatcher<S>` - which requires a function taking in a single state and returning a boolean. This will provide a default `match_state_transition` implementation.
    // - Implementing `TransitionStateMatcher<S>` - which requires a function taking in two optional states (a main and a secondary), and
    //   returning a `MatchesStateTransition` enum. This will provide a default `match_state` implementation.
    // - Implementing `StateMatcher<S>` - this requires manual implementation of both `match_state` and `match_state_transition`.
    // Here, we are using `SingleStateMatcher<S>`, since we don't need any specific, custom logic for `match_state_transition`.
    impl<T: MovementType + Default + Send + Sync + 'static> SingleStateMatcher<AppState>
        for Movement<T>
    {
        fn match_single_state(&self, state: &AppState) -> bool {
            if state.is_paused {
                return false;
            }
            let Some(movement) = &state.in_game else {
                return false;
            };
            T::detect(*movement)
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
