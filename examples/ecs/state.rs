//! This example illustrates how to use [`States`] for high-level app control flow.
//! States are a powerful but intuitive tool for controlling which logic runs when.
//! You can have multiple independent states, and the [`OnEnter`] and [`OnExit`] schedules
//! can be used to great effect to ensure that you handle setup and teardown appropriately.
//!
//! In this case, we're transitioning from a `Menu` state to an `InGame` state.

use bevy::prelude::*;
use bevy_internal::ecs::schedule::StateMatcher;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_state::<AppState>()
        .add_systems(Startup, setup)
        // This system runs when we enter `AppState::Menu`, during the `StateTransition` schedule.
        // All systems from the exit schedule of the state we're leaving are run first,
        // and then all systems from the enter schedule of the state we're entering are run second.
        .add_systems(OnEnter(AppState::Menu), setup_menu)
        .add_systems(OnEnter(AppState::InGame { paused: true }), setup_paused)
        // We also have the ability to use matchers, which are structs that allow more complex matching functions
        // These can be created with either the `::matching` function, which will run if only one of the systems matches
        // or with the `::matching_exclusive` function, which will run if the entering/exiting system matches
        // regardless of whether the other system matches as well.
        .add_systems(OnEnter::matching(InGame), setup_game)
        .add_systems(OnExit::matching_exclusive(ShowsUI), cleanup_ui)
        // By contrast, update systems are stored in the `Update` schedule. They simply
        // check the value of the `State<T>` resource to see if they should run each frame.
        .add_systems(Update, menu.run_if(in_state(AppState::Menu)))
        .add_systems(
            Update,
            movement.run_if(in_state(AppState::InGame { paused: false })),
        )
        // Alternatively, you can provide the `in_state` function with a matcher
        .add_systems(Update, change_color.run_if(in_state(InGame)))
        .add_systems(Update, toggle_pause)
        .run();
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
struct InGame;

impl StateMatcher<AppState> for InGame {
    fn match_state(&self, state: &AppState) -> bool {
        match state {
            AppState::Menu => false,
            AppState::InGame { .. } => true,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Hash, Clone)]
struct ShowsUI;

impl StateMatcher<AppState> for ShowsUI {
    fn match_state(&self, state: &AppState) -> bool {
        let ui = match state {
            AppState::Menu => true,
            AppState::InGame { paused } => *paused,
        };
        ui
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
        })
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

fn cleanup_ui(mut commands: Commands, roots: Query<Entity, (With<Node>, Without<Parent>)>) {
    for root in &roots {
        commands.entity(root).despawn_recursive();
    }
}
