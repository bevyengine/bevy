//! This example illustrates the use of [`ComputedStates`] for more complex state handling patterns.
//!
//! In this case, we'll be implementing the following pattern:
//! - The game will start in a `Menu` state, which we can return to with `Esc`
//! - From there, we can enter the game - where our bevy symbol moves around and changes color
//! - While in game, we can pause and unpause the game using `Space`
//! - We can also toggle "Turbo Mode" with the `T` key - where the movement and color changes are all faster. This
//!   is retained between pauses, but not if we exit to the main menu.
//!
//! In addition, we want to enable a "tutorial" mode, which will involve it's own state that is toggled in the main menu.
//! This will display instructions about movement and turbo mode when in game and unpaused, and instructions on how to unpause when paused.
//!
//! To implement this, we will create 2 root-level states: [`AppState`] and [`TutorialState`].
//! We will then create some computed states that derive from [`AppState`]: [`InGame`] and [`TurboMode`] are marker states implemented
//! as Zero-Sized Structs (ZSTs), while [`IsPaused`] is an enum with 2 distinct states.
//! And lastly, we'll add [`Tutorial`], a computed state deriving from [`TutorialState`], [`InGame`] and [`IsPaused`], with 2 distinct
//! states to display the 2 tutorial texts.

use bevy::{dev_tools::states::*, prelude::*};

use ui::*;

// To begin, we want to define our state objects.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum AppState {
    #[default]
    Menu,
    // Unlike in the `states` example, we're adding more data in this
    // version of our AppState. In this case, we actually have
    // 4 distinct "InGame" states - unpaused and no turbo, paused and no
    // turbo, unpaused and turbo and paused and turbo.
    InGame {
        paused: bool,
        turbo: bool,
    },
}

// The tutorial state object, on the other hand, is a fairly simple enum.
#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum TutorialState {
    #[default]
    Active,
    Inactive,
}

// Because we have 4 distinct values of `AppState` that mean we're "InGame", we're going to define
// a separate "InGame" type and implement `ComputedStates` for it.
// This allows us to only need to check against one type
// when otherwise we'd need to check against multiple.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct InGame;

impl ComputedStates for InGame {
    // Our computed state depends on `AppState`, so we need to specify it as the SourceStates type.
    type SourceStates = AppState;

    // The compute function takes in the `SourceStates`
    fn compute(sources: AppState) -> Option<Self> {
        // You might notice that InGame has no values - instead, in this case, the `State<InGame>` resource only exists
        // if the `compute` function would return `Some` - so only when we are in game.
        match sources {
            // No matter what the value of `paused` or `turbo` is, we're still in the game rather than a menu
            AppState::InGame { .. } => Some(Self),
            _ => None,
        }
    }
}

// Similarly, we want to have the TurboMode state - so we'll define that now.
//
// Having it separate from [`InGame`] and [`AppState`] like this allows us to check each of them separately, rather than
// needing to compare against every version of the AppState that could involve them.
//
// In addition, it allows us to still maintain a strict type representation - you can't Turbo
// if you aren't in game, for example - while still having the
// flexibility to check for the states as if they were completely unrelated.

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
struct TurboMode;

impl ComputedStates for TurboMode {
    type SourceStates = AppState;

    fn compute(sources: AppState) -> Option<Self> {
        match sources {
            AppState::InGame { turbo: true, .. } => Some(Self),
            _ => None,
        }
    }
}

// For the [`IsPaused`] state, we'll actually use an `enum` - because the difference between `Paused` and `NotPaused`
// involve activating different systems.
//
// To clarify the difference, `InGame` and `TurboMode` both activate systems if they exist, and there is
// no variation within them. So we defined them as Zero-Sized Structs.
//
// In contrast, pausing actually involve 3 distinct potential situations:
// - it doesn't exist - this is when being paused is meaningless, like in the menu.
// - it is `NotPaused` - in which elements like the movement system are active.
// - it is `Paused` - in which those game systems are inactive, and a pause screen is shown.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum IsPaused {
    NotPaused,
    Paused,
}

impl ComputedStates for IsPaused {
    type SourceStates = AppState;

    fn compute(sources: AppState) -> Option<Self> {
        // Here we convert from our [`AppState`] to all potential [`IsPaused`] versions.
        match sources {
            AppState::InGame { paused: true, .. } => Some(Self::Paused),
            AppState::InGame { paused: false, .. } => Some(Self::NotPaused),
            // If `AppState` is not `InGame`, pausing is meaningless, and so we set it to `None`.
            _ => None,
        }
    }
}

// Lastly, we have our tutorial, which actually has a more complex derivation.
//
// Like `IsPaused`, the tutorial has a few fully distinct possible states, so we want to represent them
// as an Enum. However - in this case they are all dependant on multiple states: the root [`TutorialState`],
// and both [`InGame`] and [`IsPaused`] - which are in turn derived from [`AppState`].
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
enum Tutorial {
    MovementInstructions,
    PauseInstructions,
}

impl ComputedStates for Tutorial {
    // We can also use tuples of types that implement [`States`] as our [`SourceStates`].
    // That includes other [`ComputedStates`] - though circular dependencies are not supported
    // and will produce a compile error.
    //
    // We could define this as relying on [`TutorialState`] and [`AppState`] instead, but
    // then we would need to duplicate the derivation logic for [`InGame`] and [`IsPaused`].
    // In this example that is not a significant undertaking, but as a rule it is likely more
    // effective to rely on the already derived states to avoid the logic drifting apart.
    //
    // Notice that you can wrap any of the [`States`] here in [`Option`]s. If you do so,
    // the the computation will get called even if the state does not exist.
    type SourceStates = (TutorialState, InGame, Option<IsPaused>);

    // Notice that we aren't using InGame - we're just using it as a source state to
    // prevent the computation from executing if we're not in game. Instead - this
    // ComputedState will just not exist in that situation.
    fn compute(
        (tutorial_state, _in_game, is_paused): (TutorialState, InGame, Option<IsPaused>),
    ) -> Option<Self> {
        // If the tutorial is inactive we don't need to worry about it.
        if !matches!(tutorial_state, TutorialState::Active) {
            return None;
        }

        // If we're paused, we're in the PauseInstructions tutorial
        // Otherwise, we're in the MovementInstructions tutorial
        match is_paused? {
            IsPaused::NotPaused => Some(Tutorial::MovementInstructions),
            IsPaused::Paused => Some(Tutorial::PauseInstructions),
        }
    }
}

fn main() {
    // We start the setup like we did in the states example.
    App::new()
        .add_plugins(DefaultPlugins)
        .init_state::<AppState>()
        .init_state::<TutorialState>()
        // After initializing the normal states, we'll use `.add_computed_state::<CS>()` to initialize our `ComputedStates`
        .add_computed_state::<InGame>()
        .add_computed_state::<IsPaused>()
        .add_computed_state::<TurboMode>()
        .add_computed_state::<Tutorial>()
        // we can then resume adding systems just like we would in any other case,
        // using our states as normal.
        .add_systems(Startup, setup)
        .add_systems(OnEnter(AppState::Menu), setup_menu)
        .add_systems(Update, menu.run_if(in_state(AppState::Menu)))
        .add_systems(OnExit(AppState::Menu), cleanup_menu)
        // We only want to run the [`setup_game`] function when we enter the [`AppState::InGame`] state, regardless
        // of whether the game is paused or not.
        .add_systems(OnEnter(InGame), setup_game)
        // And we only want to run the [`clear_game`] function when we leave the [`AppState::InGame`] state, regardless
        // of whether we're paused.
        .enable_state_scoped_entities::<InGame>()
        // We want the color change, toggle_pause and quit_to_menu systems to ignore the paused condition, so we can use the [`InGame`] derived
        // state here as well.
        .add_systems(
            Update,
            (toggle_pause, change_color, quit_to_menu).run_if(in_state(InGame)),
        )
        // However, we only want to move or toggle turbo mode if we are not in a paused state.
        .add_systems(
            Update,
            (toggle_turbo, movement).run_if(in_state(IsPaused::NotPaused)),
        )
        // We can continue setting things up, following all the same patterns used above and in the `states` example.
        .add_systems(OnEnter(IsPaused::Paused), setup_paused_screen)
        .enable_state_scoped_entities::<IsPaused>()
        .add_systems(OnEnter(TurboMode), setup_turbo_text)
        .enable_state_scoped_entities::<TurboMode>()
        .add_systems(
            OnEnter(Tutorial::MovementInstructions),
            movement_instructions,
        )
        .add_systems(OnEnter(Tutorial::PauseInstructions), pause_instructions)
        .enable_state_scoped_entities::<Tutorial>()
        .add_systems(
            Update,
            (
                log_transitions::<AppState>,
                log_transitions::<TutorialState>,
            ),
        )
        .run();
}

fn menu(
    mut next_state: ResMut<NextState<AppState>>,
    tutorial_state: Res<State<TutorialState>>,
    mut next_tutorial: ResMut<NextState<TutorialState>>,
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, &MenuButton),
        (Changed<Interaction>, With<Button>),
    >,
) {
    for (interaction, mut color, menu_button) in &mut interaction_query {
        match *interaction {
            Interaction::Pressed => {
                *color = if menu_button == &MenuButton::Tutorial
                    && tutorial_state.get() == &TutorialState::Active
                {
                    PRESSED_ACTIVE_BUTTON.into()
                } else {
                    PRESSED_BUTTON.into()
                };

                match menu_button {
                    MenuButton::Play => next_state.set(AppState::InGame {
                        paused: false,
                        turbo: false,
                    }),
                    MenuButton::Tutorial => next_tutorial.set(match tutorial_state.get() {
                        TutorialState::Active => TutorialState::Inactive,
                        TutorialState::Inactive => TutorialState::Active,
                    }),
                };
            }
            Interaction::Hovered => {
                if menu_button == &MenuButton::Tutorial
                    && tutorial_state.get() == &TutorialState::Active
                {
                    *color = HOVERED_ACTIVE_BUTTON.into();
                } else {
                    *color = HOVERED_BUTTON.into();
                }
            }
            Interaction::None => {
                if menu_button == &MenuButton::Tutorial
                    && tutorial_state.get() == &TutorialState::Active
                {
                    *color = ACTIVE_BUTTON.into();
                } else {
                    *color = NORMAL_BUTTON.into();
                }
            }
        }
    }
}

fn toggle_pause(
    input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if input.just_pressed(KeyCode::Space) {
        if let AppState::InGame { paused, turbo } = current_state.get() {
            next_state.set(AppState::InGame {
                paused: !*paused,
                turbo: *turbo,
            });
        }
    }
}

fn toggle_turbo(
    input: Res<ButtonInput<KeyCode>>,
    current_state: Res<State<AppState>>,
    mut next_state: ResMut<NextState<AppState>>,
) {
    if input.just_pressed(KeyCode::KeyT) {
        if let AppState::InGame { paused, turbo } = current_state.get() {
            next_state.set(AppState::InGame {
                paused: *paused,
                turbo: !*turbo,
            });
        }
    }
}

fn quit_to_menu(input: Res<ButtonInput<KeyCode>>, mut next_state: ResMut<NextState<AppState>>) {
    if input.just_pressed(KeyCode::Escape) {
        next_state.set(AppState::Menu);
    }
}

mod ui {
    use crate::*;

    #[derive(Resource)]
    pub struct MenuData {
        pub root_entity: Entity,
    }

    #[derive(Component, PartialEq, Eq)]
    pub enum MenuButton {
        Play,
        Tutorial,
    }

    pub const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
    pub const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
    pub const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

    pub const ACTIVE_BUTTON: Color = Color::srgb(0.15, 0.85, 0.15);
    pub const HOVERED_ACTIVE_BUTTON: Color = Color::srgb(0.25, 0.55, 0.25);
    pub const PRESSED_ACTIVE_BUTTON: Color = Color::srgb(0.35, 0.95, 0.35);

    pub fn setup(mut commands: Commands) {
        commands.spawn(Camera2dBundle::default());
    }

    pub fn setup_menu(mut commands: Commands, tutorial_state: Res<State<TutorialState>>) {
        let button_entity = commands
            .spawn(NodeBundle {
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
            })
            .with_children(|parent| {
                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(200.),
                                height: Val::Px(65.),
                                // horizontally center child text
                                justify_content: JustifyContent::Center,
                                // vertically center child text
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: NORMAL_BUTTON.into(),
                            ..default()
                        },
                        MenuButton::Play,
                    ))
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

                parent
                    .spawn((
                        ButtonBundle {
                            style: Style {
                                width: Val::Px(200.),
                                height: Val::Px(65.),
                                // horizontally center child text
                                justify_content: JustifyContent::Center,
                                // vertically center child text
                                align_items: AlignItems::Center,
                                ..default()
                            },
                            background_color: match tutorial_state.get() {
                                TutorialState::Active => ACTIVE_BUTTON,
                                TutorialState::Inactive => NORMAL_BUTTON,
                            }
                            .into(),
                            ..default()
                        },
                        MenuButton::Tutorial,
                    ))
                    .with_children(|parent| {
                        parent.spawn(TextBundle::from_section(
                            "Tutorial",
                            TextStyle {
                                font_size: 40.0,
                                color: Color::srgb(0.9, 0.9, 0.9),
                                ..default()
                            },
                        ));
                    });
            })
            .id();
        commands.insert_resource(MenuData {
            root_entity: button_entity,
        });
    }

    pub fn cleanup_menu(mut commands: Commands, menu_data: Res<MenuData>) {
        commands.entity(menu_data.root_entity).despawn_recursive();
    }

    pub fn setup_game(mut commands: Commands, asset_server: Res<AssetServer>) {
        commands.spawn((
            StateScoped(InGame),
            SpriteBundle {
                texture: asset_server.load("branding/icon.png"),
                ..default()
            },
        ));
    }

    const SPEED: f32 = 100.0;
    const TURBO_SPEED: f32 = 300.0;

    pub fn movement(
        time: Res<Time>,
        input: Res<ButtonInput<KeyCode>>,
        turbo: Option<Res<State<TurboMode>>>,
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
                transform.translation += direction.normalize()
                    * if turbo.is_some() { TURBO_SPEED } else { SPEED }
                    * time.delta_seconds();
            }
        }
    }

    pub fn setup_paused_screen(mut commands: Commands) {
        info!("Printing Pause");
        commands
            .spawn((
                StateScoped(IsPaused::Paused),
                NodeBundle {
                    style: Style {
                        // center button
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.),
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent
                    .spawn((
                        NodeBundle {
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
                        },
                        MenuButton::Play,
                    ))
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

    pub fn setup_turbo_text(mut commands: Commands) {
        commands
            .spawn((
                StateScoped(TurboMode),
                NodeBundle {
                    style: Style {
                        // center button
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        justify_content: JustifyContent::Start,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.),
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    "TURBO MODE",
                    TextStyle {
                        font_size: 40.0,
                        color: Color::srgb(0.9, 0.3, 0.1),
                        ..default()
                    },
                ));
            });
    }

    pub fn change_color(time: Res<Time>, mut query: Query<&mut Sprite>) {
        for mut sprite in &mut query {
            let new_color = LinearRgba {
                blue: (time.elapsed_seconds() * 0.5).sin() + 2.0,
                ..LinearRgba::from(sprite.color)
            };

            sprite.color = new_color.into();
        }
    }

    pub fn movement_instructions(mut commands: Commands) {
        commands
            .spawn((
                StateScoped(Tutorial::MovementInstructions),
                NodeBundle {
                    style: Style {
                        // center button
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        justify_content: JustifyContent::End,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.),
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    "Move the bevy logo with the arrow keys",
                    TextStyle {
                        font_size: 40.0,
                        color: Color::srgb(0.3, 0.3, 0.7),
                        ..default()
                    },
                ));
                parent.spawn(TextBundle::from_section(
                    "Press T to enter TURBO MODE",
                    TextStyle {
                        font_size: 40.0,
                        color: Color::srgb(0.3, 0.3, 0.7),
                        ..default()
                    },
                ));

                parent.spawn(TextBundle::from_section(
                    "Press SPACE to pause",
                    TextStyle {
                        font_size: 40.0,
                        color: Color::srgb(0.3, 0.3, 0.7),
                        ..default()
                    },
                ));

                parent.spawn(TextBundle::from_section(
                    "Press ESCAPE to return to the menu",
                    TextStyle {
                        font_size: 40.0,
                        color: Color::srgb(0.3, 0.3, 0.7),
                        ..default()
                    },
                ));
            });
    }

    pub fn pause_instructions(mut commands: Commands) {
        commands
            .spawn((
                StateScoped(Tutorial::PauseInstructions),
                NodeBundle {
                    style: Style {
                        // center button
                        width: Val::Percent(100.),
                        height: Val::Percent(100.),
                        justify_content: JustifyContent::End,
                        align_items: AlignItems::Center,
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.),
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                    ..default()
                },
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section(
                    "Press SPACE to resume",
                    TextStyle {
                        font_size: 40.0,
                        color: Color::srgb(0.3, 0.3, 0.7),
                        ..default()
                    },
                ));

                parent.spawn(TextBundle::from_section(
                    "Press ESCAPE to return to the menu",
                    TextStyle {
                        font_size: 40.0,
                        color: Color::srgb(0.3, 0.3, 0.7),
                        ..default()
                    },
                ));
            });
    }
}
