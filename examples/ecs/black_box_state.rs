//! This example builds on the other State related examples, and in particular the Struct State example,
//! to show how you can create re-usable matching sections and
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
        // Because we can't really see the internals of `AppState`, we need to rely on matching functions exported from
        // the `state` module. Fortunately, we can use these with the `entering`/`exiting`/`transitioning` functions!
        // Here we are using the `in_menu` function, which uses a derive to match any state with `in_menu: true`
        .add_systems(Entering, setup_menu.run_if(in_menu))
        // You can also use impl's to mimic enums, such as here to see if we're paused
        .add_systems(Entering, setup_paused.run_if(GameState::paused))
        // And here to see if we are in any game state
        .add_systems(Entering, setup_game.run_if(GameState::any))
        // or that we are specifically not paused
        .add_systems(Entering, setup_in_game_ui.run_if(GameState::running))
        // you can also use `Fn` values rather than points, like we do here
        .add_systems(Update, change_color.run_if(in_game()))
        // Or pass in functions that test the full transition rather than just a single state
        .add_systems(Exiting, cleanup_ui.run_if(ui.every()))
        .add_systems(Update, menu.run_if(in_menu))
        .add_systems(Update, invert_movement.run_if(GameState::running))
        // You can also use generics to specialize things
        .add_systems(Update, movement.run_if(in_movement::<Standard>))
        .add_systems(Update, inverted_movement.run_if(in_movement::<Inverted>))
        .run();
}
use bevy_internal::ecs::schedule::StateMatcher;
use state::*;
mod state {
    use bevy::prelude::States;

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

    // Then, we can start implementing our matching functions
    // The simplest ones return a bool from a single state reference
    pub fn in_menu(state: &AppState) -> bool {
        matches!(state, AppState { in_menu: true, .. })
    }

    pub struct GameState;

    impl GameState {
        // We can also use static functions from an `impl` block, like here
        pub fn any(state: &AppState) -> bool {
            matches!(
                state,
                AppState {
                    in_game: Some(_),
                    ..
                }
            )
        }

        // And use functions that take in an optional state reference
        pub fn running(state: Option<&AppState>) -> bool {
            matches!(
                state,
                Some(AppState {
                    in_game: Some(_),
                    is_paused: false,
                    ..
                })
            )
        }

        pub fn paused(state: &AppState) -> bool {
            matches!(
                state,
                AppState {
                    in_game: Some(_),
                    is_paused: true,
                    ..
                }
            )
        }
    }

    // Or we can test a transition directly - taking in either state references or optional state references.
    // The `main_state` is the primary one we care about - if used in the `entering` run condition or in the `to` portion of a transition
    // run-condition, this will be the current state, while `secondary_state` will be the previous state.
    // If we are using the `exiting` run condition or the `from` portion of a transition, `main_state` will be the `previous_state`
    // while `secondary_state` will be the current one.
    pub fn ui(main_state: &AppState, secondary_state: &AppState) -> bool {
        if main_state.in_game.is_some() && !main_state.is_paused {
            return secondary_state.in_game.is_none() || main_state.is_paused;
        }
        true
    }

    // You can also use closures, which means you can pass in stateful objects if needed!
    pub fn in_game() -> impl Fn(&AppState) -> bool {
        |s: &AppState| s.in_game.is_some()
    }

    // Lastly, here we're using generics to impact the function of our run condition.
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

    pub fn in_movement<T: MovementType + Default + Send + Sync + 'static>(
        state: &AppState,
    ) -> bool {
        if state.is_paused {
            return false;
        }
        let Some(movement) = &state.in_game else {
            return false;
        };
        T::detect(*movement)
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
