//! This example illustrates how to register custom transition behavior.
//!
//! In this case we are trying to add `OnReenter` and `OnReexit`
//! which will work much like `OnEnter` and `OnExit`, but
//! additionally trigger if the state changed into itself.
//!
//! While identity transitions exist internally, the default schedules
//! intentionally ignore them, as it is not a common use-case.

use std::marker::PhantomData;

use bevy::{ecs::schedule::ScheduleLabel, prelude::*, state::state::StateTransitionSteps};

use custom_transitions::*;
use ui::*;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Menu,
    InGame,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, SubStates)]
#[source(AppState = AppState::InGame)]
enum IsPaused {
    #[default]
    Running,
    Paused,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<AppState>()
        .add_sub_state::<IsPaused>()
        .add_systems(Startup, setup)
        .add_systems(OnEnter(AppState::Menu), setup_menu)
        .add_systems(Update, menu.run_if(in_state(AppState::Menu)))
        .add_systems(OnExit(AppState::Menu), cleanup_menu)
        // We will restart the game progress every time we re-enter into it.
        .add_plugins(IdentityTransitionsPlugin::<AppState>::default())
        .add_systems(OnReenter(AppState::InGame), setup_game)
        .add_systems(OnReexit(AppState::InGame), teardown_game)
        // Doing it this way allows us to restart the game without any in-between states.
        .add_systems(OnEnter(IsPaused::Paused), setup_paused_screen)
        .add_systems(
            OnExit(IsPaused::Paused),
            clear_state_bound_entities(IsPaused::Paused),
        )
        .add_systems(
            Update,
            (
                (movement, change_color).run_if(in_state(IsPaused::Running)),
                (toggle_pause, restart_game).run_if(in_state(AppState::InGame)),
            ),
        )
        .add_systems(Update, log_transitions::<AppState>)
        .run();
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

fn toggle_pause(
    input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<IsPaused>>,
    mut next_state: ResMut<NextState<IsPaused>>,
) {
    if input.just_pressed(KeyCode::Space) {
        next_state.set(match current_state.get() {
            IsPaused::Running => IsPaused::Paused,
            IsPaused::Paused => IsPaused::Running,
        });
    }
}

fn restart_game(input: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if input.just_pressed(KeyCode::KeyR) {
        // Although we are already in this state
        // setting it again will generate an identity transition.
        // While default schedules ignore those kinds of transitions,
        // out custom schedules will react to them.
        next_state.set(AppState::InGame);
    }
}

#[derive(Component)]
struct StateBound<S: States>(S);

fn clear_state_bound_entities<S: States>(
    state: S,
) -> impl Fn(Commands, Query<(Entity, &StateBound<S>)>) {
    move |mut commands, query| {
        for (entity, bound) in &query {
            if bound.0 == state {
                commands.entity(entity).despawn_recursive();
            }
        }
    }
}

fn log_transitions<S: States>(mut transitions: EventReader<StateTransitionEvent<S>>) {
    for transition in transitions.read() {
        info!(
            "Transition: {:?} => {:?}",
            transition.exited, transition.entered
        );
    }
}

/// This module provides the custom `OnReenter` and `OnReexit` transitions for easy installation.
mod custom_transitions {
    use crate::*;

    /// Custom schedule that will behave like `OnEnter`,
    /// but run even during identity transitions.
    #[derive(ScheduleLabel, Clone, Debug, PartialEq, Eq, Hash)]
    pub struct OnReenter<S: States>(pub S);

    fn run_reenter<S: States>(transition: In<Option<StateTransitionEvent<S>>>, world: &mut World) {
        let Some(transition) = transition.0 else {
            return;
        };
        // If we wanted to ignore identity transitions,
        // we'd compare `exited` and `entered` here,
        // and return if they were the same.
        let Some(entered) = transition.entered else {
            return;
        };

        let _ = world.try_run_schedule(OnReenter(entered));
    }

    /// Custom schedule that will behave like `OnExit`,
    /// but run even during identity transitions.
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

    /// The plugin registers the transitions for one specific state.
    /// If you use this for multiple states consider:
    /// - installing the plugin multiple times,
    /// - add an [`App`] extension method that inserts
    ///   those transitions during state installation.
    #[derive(Default)]
    pub struct IdentityTransitionsPlugin<S: States>(PhantomData<S>);

    impl<S: States> Plugin for IdentityTransitionsPlugin<S> {
        fn build(&self, app: &mut App) {
            app.add_systems(
                StateTransition,
                // The internals can generate at most one transition event per frame
                // so we take the latest one and clear the queue.
                last_transition::<S>
                    // We insert the optional event into our schedule runner.
                    .pipe(run_reenter::<S>)
                    // We use the same step as [`OnEnter`]
                    .in_set(StateTransitionSteps::EnterSchedules),
            )
            .add_systems(
                StateTransition,
                last_transition::<S>
                    .pipe(run_reexit::<S>)
                    .in_set(StateTransitionSteps::ExitSchedules),
            );
        }
    }
}

mod ui {
    use crate::*;

    #[derive(Resource)]
    pub struct MenuData {
        pub button_entity: Entity,
    }

    pub const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
    pub const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
    pub const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

    pub fn setup(mut commands: Commands) {
        commands.spawn(Camera2dBundle::default());
    }

    pub fn setup_menu(mut commands: Commands) {
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

    pub fn setup_game(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn(SpriteBundle {
            texture: asset_server.load("branding/icon.png"),
            ..default()
        });
    }

    pub fn teardown_game(mut commands: Commands, player: Query<Entity, With<Sprite>>) {
        commands.entity(player.single()).despawn();
    }

    pub fn setup_paused_screen(mut commands: Commands) {
        commands
            .spawn((
                StateBound(IsPaused::Paused),
                NodeBundle {
                    style: Style {
                        // center button
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.),
                        ..default()
                    },
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent
                    .spawn(NodeBundle {
                        style: Style {
                            width: Val::Px(400.),
                            height: Val::Px(400.),
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
                            "Paused",
                            TextStyle {
                                font_size: 40.0,
                                color: Color::srgb(0.9, 0.9, 0.9),
                                ..default()
                            },
                        ));
                    });
            });
    }
}
