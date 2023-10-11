//! This example illustrates how to schedule systems for very specific transitions between states.
//!
//! In this case, we're using rock-paper-scissors logic to illustrate targeting specific transitions.
//!
// This lint usually gives bad advice in the context of Bevy -- hiding complex queries behind
// type aliases tends to obfuscate code while offering no improvement in code cleanliness.
#![allow(clippy::type_complexity)]
use bevy::prelude::*;
use bevy_internal::ecs::schedule::StateMatcher;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_state::<AppState>()
        .add_systems(
            Exiting,
            cleanup_ui.run_if(state_matches!(AppState, every _)),
        )
        // The simplest way to handle transitions is using the `transitioning` function - which works well for
        // state values
        .add_systems(
            Entering,
            setup_rocks_to_paper
                .run_if(AppState::Rock.invert_transition().and_then(AppState::Paper)),
        )
        .add_systems(
            Exiting,
            setup_scissors_to_paper
                .run_if(AppState::Scissors.and_then(AppState::Paper.invert_transition())),
        )
        // We can also use the macro, to enable some pattern matching
        .add_systems(
            Exiting,
            setup_any_to_rock.run_if(
                state_matches!(AppState, Scissors | Paper)
                    .and_then(AppState::Rock.invert_transition()),
            ),
        )
        // You can use all the pattern matching features from the other macros, like the "every" keyword
        .add_systems(
            Exiting,
            setup_scissors.run_if(
                state_matches!(AppState, _)
                    .every()
                    .and_then(AppState::Scissors.invert_transition()),
            ),
        )
        // And it can also work with closures
        .add_systems(
            Exiting,
            setup_sciessors_or_rock_from_paper.run_if(
                AppState::Paper.and_then(
                    (|s: &AppState| s == &AppState::Scissors || s == &AppState::Rock)
                        .invert_transition(),
                ),
            ),
        )
        // In some cases, it might be easy to use "entering!" or "exiting!" instead, while still checking with both states,
        // such as if you have some custom logic that needs to process the "to" and "from" states at once
        .add_systems(
            Exiting,
            paper_always_wins.run_if(state_matches!(
                AppState,
                |from: &AppState, to: &AppState| {
                    if from == &AppState::Paper && to == &AppState::Rock {
                        println!("paper wins");
                        return true;
                    }
                    if from == &AppState::Rock && to == &AppState::Paper {
                        println!("paper always wins");
                        return true;
                    }
                    false
                }
            )),
        )
        // We can also pass closures and other matchers into the transitioning function, for example to test if the previous state even exists
        .add_systems(
            Entering,
            setup_rocks_from_startup.run_if(
                (|s: Option<&AppState>| s.is_none())
                    .invert_transition()
                    .and_then(|s: &AppState| s == &AppState::Rock),
            ),
        )
        .add_systems(Entering, last_choice_was_paper.run_if(AppState::Paper))
        .add_systems(Entering, last_choice_was_rock.run_if(AppState::Rock))
        .add_systems(
            Entering,
            last_choice_was_scissors.run_if(AppState::Scissors),
        )
        .add_systems(Update, menu)
        .run();
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Rock,
    Paper,
    Scissors,
}

const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);
const LABEL: Color = Color::rgba(0.0, 0.0, 0.0, 0.7);

#[derive(Component)]
struct TargetState(AppState);

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn setup_rocks_from_startup(commands: Commands) {
    setup_menu(
        commands,
        "Started at Rock",
        &[
            ("Paper....", AppState::Paper),
            ("SCISSORS!", AppState::Scissors),
        ],
    );
}

fn setup_scissors_to_paper(commands: Commands) {
    setup_menu(
        commands,
        "The paper is cut! Oh no!",
        &[
            ("MOAR SCISSORS!", AppState::Scissors),
            ("Rock", AppState::Rock),
        ],
    );
}

fn setup_rocks_to_paper(commands: Commands) {
    setup_menu(
        commands,
        "Paper wrapped the Rock",
        &[("SCISSORS!", AppState::Scissors), ("Rock?", AppState::Rock)],
    );
}

fn setup_sciessors_or_rock_from_paper(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                // center button
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "The paper is gone now.. The paper is gone.",
                TextStyle {
                    font_size: 60.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                    ..default()
                },
            ));
        });
}

fn paper_always_wins(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                // center button
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexEnd,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "PAPER ALWAYS WINS!",
                TextStyle {
                    font_size: 60.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                    ..default()
                },
            ));
        });
}

fn last_choice_was_paper(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                // center button
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Last choice was paper...",
                TextStyle {
                    font_size: 60.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                    ..default()
                },
            ));
        });
}

fn last_choice_was_scissors(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                // center button
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "You chose scissors - what's next?",
                TextStyle {
                    font_size: 60.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                    ..default()
                },
            ));
        });
}

fn last_choice_was_rock(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                // center button
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::FlexStart,
                align_items: AlignItems::FlexStart,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "ahh yes - we're at rock.",
                TextStyle {
                    font_size: 60.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                    ..default()
                },
            ));
        });
}

fn setup_any_to_rock(commands: Commands) {
    setup_menu(
        commands,
        "Getting Ready to Rock",
        &[
            ("scissors", AppState::Scissors),
            ("Pa-per! Pa-per!", AppState::Paper),
        ],
    );
}

fn setup_scissors(commands: Commands) {
    setup_menu(
        commands,
        "Cutting along with my Scissors",
        &[("ROCK", AppState::Rock), ("Pa.... per?", AppState::Paper)],
    );
}

fn setup_menu(mut commands: Commands, label: &str, options: &[(&str, AppState)]) {
    commands
        .spawn(NodeBundle {
            style: Style {
                // center button
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn((NodeBundle {
                    style: Style {
                        padding: UiRect::all(Val::Px(10.)),
                        // horizontally center child text
                        justify_content: JustifyContent::Center,
                        // vertically center child text
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: LABEL.into(),
                    ..default()
                },))
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        label,
                        TextStyle {
                            font_size: 60.0,
                            color: Color::rgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                });
            for (label, state) in options.iter() {
                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                padding: UiRect::all(Val::Px(10.)),
                                // horizontally center child text
                                justify_content: JustifyContent::Center,
                                // vertically center child text
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: NORMAL_BUTTON.into(),
                            ..default()
                        },
                        TargetState(*state),
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            *label,
                            TextStyle {
                                font_size: 40.0,
                                color: Color::rgb(0.9, 0.9, 0.9),
                                ..default()
                            },
                        ));
                    });
            }
        });
}

fn menu(
    mut next_state: ResMut<NextState<AppState>>,
    mut interaction_query: Query<
        (&Interaction, &TargetState, &mut BackgroundColor),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, target, mut color) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON.into();
                // One way to set the next state is to set the full state value, like so
                next_state.set(target.0);
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

fn cleanup_ui(mut commands: Commands, roots: Query<Entity, (With<Node>, Without<Parent>)>) {
    for root in &roots {
        commands.entity(root).despawn_recursive();
    }
}
