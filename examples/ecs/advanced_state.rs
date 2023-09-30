//! This example illustrates how to use some of the more advance [`States`] functionality for high-level app control flow.
//!
//! The use case here is identical to the regular [state](./state.rs) example, but we will be
//! utilizing the `StateMatcher` trait more directly.
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_state::<AppState>()
        .add_systems(Startup, setup)
        // When we check whether a state is valid, we use a `StateMatcher`.
        // In the simple example, we use pattern matching to generate one on the fly
        // But we can also use pre-defined ones, like `ShowsUI`
        .add_systems(on_exit_strict!(ShowsUI), cleanup_ui)
        // In addition, all `States` also implement `StateMatcher`, so you can use them here.
        // Although there are some optimizations around `OnEnter(S)` and `OnExit(S)` that make
        // those a better choice in this case.
        .add_systems(on_enter!(AppState::Menu), setup_menu)
        // A state matcher can even be an enum! or really anything with that implements
        // the state matcher trait (see below).
        .add_systems(on_enter!(InGame::Paused), setup_paused)
        .add_systems(on_enter!(InGame::Running), setup_in_game_ui)
        // And unlike a regular state, the values are not mutually exclusive.
        // So this will be valid both when the game is paused and when it's not
        .add_systems(on_enter!(InGame::Any), setup_game)
        // And all the same things apply with the `in_state!` macro
        .add_systems(Update, menu.run_if(in_state!(AppState::Menu)))
        .add_systems(Update, change_color.run_if(in_state!(InGame::Any)))
        .add_systems(Update, toggle_pause)
        // And it still works with conditional states as well
        .add_sub_state::<MovementState, _>(InGame::Running)
        .add_systems(Update, invert_movement.run_if(in_state!(MovementState, _)))
        .add_systems(Update, movement.run_if(in_state(MovementState::Normal)))
        .add_systems(
            Update,
            inverted_movement.run_if(in_state(MovementState::Invert)),
        )
        .run();
}

// The simplest way to define a state matcher is using the `state_matcher!` macro.
// You pass in the visibility (optionally), name, state type and match expression.
state_matcher!(pub ShowsUI, AppState, _);

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
enum InGame {
    Any,
    Running,
    Paused,
}

// State Matchers can also be implemented manually
// This allows for somewhat more complex situations,
// For example here we can determine if we want to match in
// any InGame state, only when the game is paused, or only when it
// is running. You can't have this kind of logic using pure equality,
// and having it defined this ay makes it re-usable and customizable to your needs.
impl StateMatcher<AppState> for InGame {
    fn match_state(&self, state: &AppState) -> bool {
        match state {
            AppState::InGame { paused } => match self {
                InGame::Any => true,
                InGame::Running => !*paused,
                InGame::Paused => *paused,
            },
            _ => false,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Menu,
    InGame {
        paused: bool,
    },
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum MovementState {
    #[default]
    Normal,
    Invert,
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
                next_state.set(AppState::InGame { paused: true });
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
            AppState::InGame { paused } => AppState::InGame { paused: !paused },
            _ => state,
        });
    }
}

fn invert_movement(input: Res<Input<KeyCode>>, mut next_state: ResMut<NextState<MovementState>>) {
    if input.just_pressed(KeyCode::ShiftLeft) {
        next_state.set(MovementState::Invert);
    }
    if input.just_released(KeyCode::ShiftLeft) {
        next_state.set(MovementState::Normal);
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
