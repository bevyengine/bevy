//! This example illustrates how to register custom state transition behavior.
//!
//! In this case we are trying to add `OnReenter` and `OnReexit`
//! which will work much like `OnEnter` and `OnExit`,
//! but additionally trigger if the state changed into itself.
//!
//! While identity transitions exist internally in [`StateTransitionEvent`]s,
//! the default schedules intentionally ignore them, as this behavior is not commonly needed or expected.
//!
//! While this example displays identity transitions for a single state,
//! identity transitions are propagated through the entire state graph,
//! meaning any change to parent state will be propagated to [`ComputedStates`] and [`SubStates`].

use std::marker::PhantomData;

use bevy::{dev_tools::states::*, ecs::schedule::ScheduleLabel, prelude::*};

use custom_transitions::*;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Menu,
    InGame,
}

fn main() {
    App::new()
        // We insert the custom transitions plugin for `AppState`.
        .add_plugins((
            DefaultPlugins,
            IdentityTransitionsPlugin::<AppState>::default(),
        ))
        .init_state::<AppState>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(AppState::Menu), setup_menu)
        .add_systems(Update, menu.run_if(in_state(AppState::Menu)))
        .add_systems(OnExit(AppState::Menu), cleanup_menu)
        // We will restart the game progress every time we re-enter into it.
        .add_systems(OnReenter(AppState::InGame), setup_game)
        .add_systems(OnReexit(AppState::InGame), teardown_game)
        // Doing it this way allows us to restart the game without any additional in-between states.
        .add_systems(
            Update,
            ((movement, change_color, trigger_game_restart).run_if(in_state(AppState::InGame)),),
        )
        .add_systems(Update, log_transitions::<AppState>)
        .run();
}

/// This module provides the custom `OnReenter` and `OnReexit` transitions for easy installation.
mod custom_transitions {
    use crate::*;

    /// The plugin registers the transitions for one specific state.
    /// If you use this for multiple states consider:
    /// - installing the plugin multiple times,
    /// - create an [`App`] extension method that inserts
    ///   those transitions during state installation.
    #[derive(Default)]
    pub struct IdentityTransitionsPlugin<S: States>(PhantomData<S>);

    impl<S: States> Plugin for IdentityTransitionsPlugin<S> {
        fn build(&self, app: &mut App) {
            app.add_systems(
                StateTransition,
                // The internals can generate at most one transition event of specific type per frame.
                // We take the latest one and clear the queue.
                last_transition::<S>
                    // We insert the optional event into our schedule runner.
                    .pipe(run_reenter::<S>)
                    // State transitions are handled in three ordered steps, exposed as system sets.
                    // We can add our systems to them, which will run the corresponding schedules when they're evaluated.
                    // These are:
                    // - [`ExitSchedules`] - Ran from leaf-states to root-states,
                    // - [`TransitionSchedules`] - Ran in arbitrary order,
                    // - [`EnterSchedules`] - Ran from root-states to leaf-states.
                    .in_set(EnterSchedules::<S>::default()),
            )
            .add_systems(
                StateTransition,
                last_transition::<S>
                    .pipe(run_reexit::<S>)
                    .in_set(ExitSchedules::<S>::default()),
            );
        }
    }

    /// Custom schedule that will behave like [`OnEnter`], but run on identity transitions.
    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct OnReenter<S: States>(pub S);

    /// Schedule runner which checks conditions and if they're right
    /// runs out custom schedule.
    fn run_reenter<S: States>(transition: In<Option<StateTransitionEvent<S>>>, world: &mut World) {
        // We return early if no transition event happened.
        let Some(transition) = transition.0 else {
            return;
        };

        // If we wanted to ignore identity transitions,
        // we'd compare `exited` and `entered` here,
        // and return if they were the same.

        // We check if we actually entered a state.
        // A [`None`] would indicate that the state was removed from the world.
        // This only happens in the case of [`SubStates`] and [`ComputedStates`].
        let Some(entered) = transition.entered else {
            return;
        };

        // If all conditions are valid, we run our custom schedule.
        let _ = world.try_run_schedule(OnReenter(entered));

        // If you want to overwrite the default `OnEnter` behavior to act like re-enter,
        // you can do so by running the `OnEnter` schedule here. Note that you don't want
        // to run `OnEnter` when the default behavior does so.
        // ```
        // if transition.entered != transition.exited {
        //     return;
        // }
        // let _ = world.try_run_schedule(OnReenter(entered));
        // ```
    }

    /// Custom schedule that will behave like [`OnExit`], but run on identity transitions.
    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct OnReexit<S: States>(pub S);

    fn run_reexit<S: States>(transition: In<Option<StateTransitionEvent<S>>>, world: &mut World) {
        let Some(transition) = transition.0 else {
            return;
        };
        let Some(exited) = transition.exited else {
            return;
        };

        let _ = world.try_run_schedule(OnReexit(exited));
    }
}

fn menu(
    mut next_state: ResMut<NextState<AppState>>,
    mut interaction_query: Query<
        (&Interaction, &mut UiImage),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut image) in &mut interaction_query {
        let color = &mut image.color;
        match *interaction {
            Interaction::Pressed => {
                *color = PRESSED_BUTTON;
                next_state.set(AppState::InGame);
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON;
            }
            Interaction::None => {
                *color = NORMAL_BUTTON;
            }
        }
    }
}

fn cleanup_menu(mut commands: Commands, menu_data: Res<MenuData>) {
    commands.entity(menu_data.button_entity).despawn_recursive();
}

const SPEED: f32 = 100.0;
fn movement(
    time: Res<Time>,
    input: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Sprite>>,
) {
    for mut transform in &mut query {
        let mut direction = Vec3::ZERO;
        if input.pressed(KeyCode::ArrowLeft) {
            direction.x -= 1.0;
        }
        if input.pressed(KeyCode::ArrowRight) {
            direction.x += 1.0;
        }
        if input.pressed(KeyCode::ArrowUp) {
            direction.y += 1.0;
        }
        if input.pressed(KeyCode::ArrowDown) {
            direction.y -= 1.0;
        }

        if direction != Vec3::ZERO {
            transform.translation += direction.normalize() * SPEED * time.delta_seconds();
        }
    }
}

fn change_color(time: Res<Time>, mut query: Query<&mut Sprite>) {
    for mut sprite in &mut query {
        let new_color = LinearRgba {
            blue: (time.elapsed_seconds() * 0.5).sin() + 2.0,
            ..LinearRgba::from(sprite.color)
        };

        sprite.color = new_color.into();
    }
}

// We can restart the game by pressing "R".
// This will trigger an [`AppState::InGame`] -> [`AppState::InGame`]
// transition, which will run our custom schedules.
fn trigger_game_restart(
    input: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if input.just_pressed(KeyCode::KeyR) {
        // Although we are already in this state setting it again will generate an identity transition.
        // While default schedules ignore those kinds of transitions, our custom schedules will react to them.
        next_state.set(AppState::InGame);
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn setup_game(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(SpriteBundle {
        texture: asset_server.load("branding/icon.png"),
        ..default()
    });
    info!("Setup game");
}

fn teardown_game(mut commands: Commands, player: Query<Entity, With<Sprite>>) {
    commands.entity(player.single()).despawn();
    info!("Teardown game");
}

#[derive(Resource)]
struct MenuData {
    pub button_entity: Entity,
}

const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

fn setup_menu(mut commands: Commands) {
    let button_entity = commands
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
                    image: UiImage::default().with_color(NORMAL_BUTTON),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section(
                        "Play",
                        TextStyle {
                            font_size: 40.0,
                            color: Color::srgb(0.9, 0.9, 0.9),
                            ..default()
                        },
                    ));
                });
        })
        .id();
    commands.insert_resource(MenuData { button_entity });
}
