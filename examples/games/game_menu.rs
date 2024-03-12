//! This example will display a simple menu using Bevy UI where you can start a new game,
//! change some settings or quit. There is no actual game, it will just display the current
//! settings for 5 seconds before going back to the menu.
//!
//! STATE DIAGRAM
//!
//! ```
//!          START                                 
//!            │                                   
//! ┌──────────▼──────────┐ ┌─────────────────────┐
//! │  GameState::Splash  │ │   GameState::Game   │
//! │ MenuState::Disabled │ │ MenuState::Disabled │
//! └───┬─────────────────┘ └─▲─────┬─────────────┘
//!     │                     │     │              
//! ┌───┼─────────────────────┼─────┼────────┐     
//! │   │                     │     │        │     
//! │ (timer)               Play  (timer)    │     
//! │   │                     │     │        │     
//! │ ┌─▼─────────────────────┴─────▼─┐      │     
//! │ │     MenuState::Main           ├─Quit─┼─►END
//! │ └─┬───────────────────────────▲─┘      │     
//! │   │                           │        │     
//! │ Settings           BackToMainMenu      │     
//! │   │                           │        │     
//! │ ┌─▼───────────────────────────┴──────┐ │     
//! │ │     MenuState::Settings            │ │     
//! │ └─┬───┬─────────────────────────▲──▲─┘ │     
//! │   │   │                         │  │   │     
//! │   │ SettingsDisplay     BackToSettings │     
//! │   │   │                         │  │   │     
//! │   │ ┌─▼─────────────────────────┴┐ │   │     
//! │   │ │ MenuState::SettingsDisplay │ │   │     
//! │   │ └────────────────────────────┘ │   │     
//! │   │                                │   │     
//! │ SettingsSound                      │   │     
//! │   │                                │   │     
//! │ ┌─▼────────────────────────────────┴─┐ │     
//! │ │     MenuState::SettingsSound       │ │     
//! │ └────────────────────────────────────┘ │     
//! │                                        │     
//! │            GameState::Menu             │     
//! │                                        │     
//! └────────────────────────────────────────┘     
//! ```

use bevy::prelude::*;

const TEXT_COLOR: Color = Color::srgb(0.9, 0.9, 0.9);

const RED: Color = Color::srgba(0.863, 0.078, 0.235, 1.0);

// Enum that will be used as a global state for the game
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
enum GameState {
    #[default]
    Splash,
    Menu,
    Game,
}

// One of the two settings that can be set through the menu. It will be a resource in the app
#[derive(Resource, Debug, Component, PartialEq, Eq, Clone, Copy)]
enum DisplayQuality {
    Low,
    Medium,
    High,
}

// One of the two settings that can be set through the menu. It will be a resource in the app
#[derive(Resource, Debug, Component, PartialEq, Eq, Clone, Copy)]
struct Volume(u32);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Insert as resource the initial value for the settings resources
        .insert_resource(DisplayQuality::Medium)
        .insert_resource(Volume(7))
        // Declare the game state, whose starting value is determined by the `Default` trait
        .init_state::<GameState>()
        .add_systems(Startup, setup)
        // Adds the plugins for each state
        .add_plugins((splash::splash_plugin, menu::menu_plugin, game::game_plugin))
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn default_font(font_size: f32, color: Color) -> TextStyle {
    TextStyle {
        font_size,
        color,
        ..default()
    }
}

// This outermost node encompasses the entire Window (100% width and height)
// and centers all content vertically (align_items) and horizontally (justify_content)
fn outer_node(
    commands: &mut Commands,
    marker: impl Component,
    spawn_children: impl FnOnce(&mut ChildBuilder),
) {
    commands
        .spawn((
            NodeBundle {
                style: Style {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                ..default()
            },
            marker,
        ))
        .with_children(spawn_children);
}

// This node is always a direct child of the `outer` node, and arranges contents in a column, from top to bottom
fn inner_node<T: Into<BackgroundColor>>(
    commands: &mut Commands,
    marker: impl Component,
    color: T,
    spawn_children: impl FnOnce(&mut ChildBuilder),
) {
    outer_node(commands, marker, |parent| {
        parent
            .spawn(NodeBundle {
                style: Style {
                    // This will display its children in a column, from top to bottom
                    flex_direction: FlexDirection::Column,
                    // `align_items` will align children on the cross axis. Here the main axis is
                    // vertical (column), so the cross axis is horizontal. This will center the
                    // children
                    align_items: AlignItems::Center,
                    ..default()
                },
                background_color: color.into(),
                ..default()
            })
            .with_children(spawn_children);
    });
}

mod splash {
    use bevy::prelude::*;

    use super::{despawn_screen, outer_node, GameState};

    // This plugin will display a splash screen with Bevy logo for 1 second before switching to the menu
    pub fn splash_plugin(app: &mut App) {
        // As this plugin is managing the splash screen, it will focus on the state `GameState::Splash`
        app
            // When entering the state, spawn everything needed for this screen
            .add_systems(OnEnter(GameState::Splash), splash_setup)
            // While in this state, run the `countdown` system
            .add_systems(Update, countdown.run_if(in_state(GameState::Splash)))
            // When exiting the state, despawn everything that was spawned for this screen
            .add_systems(OnExit(GameState::Splash), despawn_screen::<OnSplashScreen>);
    }

    // Tag component used to tag entities added on the splash screen
    #[derive(Component)]
    struct OnSplashScreen;

    // Newtype to use a `Timer` for this screen as a resource
    #[derive(Resource, Deref, DerefMut)]
    struct SplashTimer(Timer);

    fn splash_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        let icon = asset_server.load("branding/icon.png");

        // Display the logo
        outer_node(&mut commands, OnSplashScreen, |parent| {
            parent.spawn(ImageBundle {
                style: Style {
                    // This will set the logo to be 200px wide, and auto adjust its height
                    width: Val::Px(200.0),
                    ..default()
                },
                image: UiImage::new(icon),
                ..default()
            });
        });

        // Insert the timer as a resource
        commands.insert_resource(SplashTimer(Timer::from_seconds(1.0, TimerMode::Once)));
    }

    // Tick the timer, and change state when finished
    fn countdown(
        mut game_state: ResMut<NextState<GameState>>,
        time: Res<Time>,
        mut timer: ResMut<SplashTimer>,
    ) {
        if timer.tick(time.delta()).finished() {
            game_state.set(GameState::Menu);
        }
    }
}

mod game {
    use bevy::prelude::*;

    use super::{
        default_font, despawn_screen, inner_node, DisplayQuality, GameState, Volume, TEXT_COLOR,
    };
    // This plugin will contain the game. In this case, it's just be a screen that will
    // display the current settings for 5 seconds before returning to the menu
    pub fn game_plugin(app: &mut App) {
        app.add_systems(OnEnter(GameState::Game), game_setup)
            .add_systems(Update, game.run_if(in_state(GameState::Game)))
            .add_systems(OnExit(GameState::Game), despawn_screen::<OnGameScreen>);
    }

    // Tag component used to tag entities added on the game screen
    #[derive(Component)]
    struct OnGameScreen;

    #[derive(Resource, Deref, DerefMut)]
    struct GameTimer(Timer);

    fn game_setup(
        mut commands: Commands,
        display_quality: Res<DisplayQuality>,
        volume: Res<Volume>,
    ) {
        inner_node(&mut commands, OnGameScreen, Color::BLACK, |parent| {
            // Display two lines of text, the second one with the current settings
            parent.spawn(
                TextBundle::from_section(
                    "Will be back to the menu shortly...",
                    default_font(80., TEXT_COLOR),
                )
                .with_style(Style {
                    margin: UiRect::all(Val::Px(50.0)),
                    ..default()
                }),
            );
            parent.spawn(
                TextBundle::from_sections([
                    TextSection::new(
                        format!("quality: {:?}", *display_quality),
                        default_font(60., Color::srgb(0.0, 0.0, 1.0)),
                    ),
                    TextSection::new(" - ", default_font(60., TEXT_COLOR)),
                    TextSection::new(
                        format!("volume: {:?}", *volume),
                        default_font(60., Color::srgb(0.0, 1.0, 0.0)),
                    ),
                ])
                .with_style(Style {
                    margin: UiRect::all(Val::Px(50.0)),
                    ..default()
                }),
            );
        });

        // Spawn a 5 seconds timer to trigger going back to the menu
        commands.insert_resource(GameTimer(Timer::from_seconds(5.0, TimerMode::Once)));
    }

    // Tick the timer, and change state when finished
    fn game(
        time: Res<Time>,
        mut game_state: ResMut<NextState<GameState>>,
        mut timer: ResMut<GameTimer>,
    ) {
        if timer.tick(time.delta()).finished() {
            game_state.set(GameState::Menu);
        }
    }
}

mod menu {
    use bevy::{app::AppExit, prelude::*};

    use super::{default_font, despawn_screen, inner_node, DisplayQuality, GameState, Volume, TEXT_COLOR, RED};
    // This plugin manages the menu, with 5 different screens:
    // - a main menu with "New Game", "Settings", "Quit"
    // - a settings menu with two submenus and a back button
    // - two settings screen with a setting that can be set and a back button
    pub fn menu_plugin(app: &mut App) {
        app
            // At start, the menu is not enabled. This will be changed in `menu_setup` when
            // entering the `GameState::Menu` state.
            // Current screen in the menu is handled by an independent state from `GameState`
            .init_state::<MenuState>()
            .add_systems(OnEnter(GameState::Menu), menu_setup)
            // Systems to handle the main menu screen
            .add_systems(OnEnter(MenuState::Main), main_menu_setup)
            .add_systems(OnExit(MenuState::Main), despawn_screen::<OnMainMenuScreen>)
            // Systems to handle the settings menu screen
            .add_systems(OnEnter(MenuState::Settings), settings_menu_setup)
            .add_systems(
                OnExit(MenuState::Settings),
                despawn_screen::<OnSettingsMenuScreen>,
            )
            // Systems to handle the display settings screen
            .add_systems(
                OnEnter(MenuState::SettingsDisplay),
                display_settings_menu_setup,
            )
            .add_systems(
                Update,
                (setting_button::<DisplayQuality>.run_if(in_state(MenuState::SettingsDisplay)),),
            )
            .add_systems(
                OnExit(MenuState::SettingsDisplay),
                despawn_screen::<OnDisplaySettingsMenuScreen>,
            )
            // Systems to handle the sound settings screen
            .add_systems(OnEnter(MenuState::SettingsSound), sound_settings_menu_setup)
            .add_systems(
                Update,
                setting_button::<Volume>.run_if(in_state(MenuState::SettingsSound)),
            )
            .add_systems(
                OnExit(MenuState::SettingsSound),
                despawn_screen::<OnSoundSettingsMenuScreen>,
            )
            // Common systems to all screens that handles buttons behavior
            .add_systems(
                Update,
                (menu_action, button_system).run_if(in_state(GameState::Menu)),
            );
    }

    // State used for the current menu screen
    #[derive(Clone, Copy, Default, Eq, PartialEq, Debug, Hash, States)]
    enum MenuState {
        Main,
        Settings,
        SettingsDisplay,
        SettingsSound,
        #[default]
        Disabled,
    }

    // Tag component used to tag entities added on the main menu screen
    #[derive(Component)]
    struct OnMainMenuScreen;

    // Tag component used to tag entities added on the settings menu screen
    #[derive(Component)]
    struct OnSettingsMenuScreen;

    // Tag component used to tag entities added on the display settings menu screen
    #[derive(Component)]
    struct OnDisplaySettingsMenuScreen;

    // Tag component used to tag entities added on the sound settings menu screen
    #[derive(Component)]
    struct OnSoundSettingsMenuScreen;

    const NORMAL_BUTTON: Color = Color::srgb(0.15, 0.15, 0.15);
    const HOVERED_BUTTON: Color = Color::srgb(0.25, 0.25, 0.25);
    const HOVERED_PRESSED_BUTTON: Color = Color::srgb(0.25, 0.65, 0.25);
    const PRESSED_BUTTON: Color = Color::srgb(0.35, 0.75, 0.35);

    // Tag component used to mark which setting is currently selected
    #[derive(Component)]
    struct SelectedOption;

    // All actions that can be triggered from a button click
    #[derive(Component)]
    enum MenuButtonAction {
        Play,
        Settings,
        SettingsDisplay,
        SettingsSound,
        BackToMainMenu,
        BackToSettings,
        Quit,
    }

    // This system handles changing all buttons color based on mouse interaction
    fn button_system(
        mut interaction_query: Query<
            (&Interaction, &mut UiImage, Option<&SelectedOption>),
            (Changed<Interaction>, With<Button>),
        >,
    ) {
        for (interaction, mut color, selected) in &mut interaction_query {
            let new_color = match (*interaction, selected) {
                (Interaction::Pressed, _) | (Interaction::None, Some(_)) => PRESSED_BUTTON,
                (Interaction::Hovered, Some(_)) => HOVERED_PRESSED_BUTTON,
                (Interaction::Hovered, None) => HOVERED_BUTTON,
                (Interaction::None, None) => NORMAL_BUTTON,
            };

            *color = UiImage::default().with_color(new_color);
        }
    }

    // This system updates the settings when a new value for a setting is selected, and marks
    // the button as the one currently selected
    fn setting_button<T: Resource + Component + PartialEq + Copy>(
        interaction_query: Query<(&Interaction, &T, Entity), (Changed<Interaction>, With<Button>)>,
        mut selected_query: Query<(Entity, &mut UiImage), With<SelectedOption>>,
        mut commands: Commands,
        mut setting: ResMut<T>,
    ) {
        for (interaction, button_setting, entity) in &interaction_query {
            if *interaction == Interaction::Pressed && *setting != *button_setting {
                let (previous_button, mut previous_color) = selected_query.single_mut();
                *previous_color = UiImage::default().with_color(NORMAL_BUTTON);
                commands.entity(previous_button).remove::<SelectedOption>();
                commands.entity(entity).insert(SelectedOption);
                *setting = *button_setting;
            }
        }
    }

    fn menu_setup(mut menu_state: ResMut<NextState<MenuState>>) {
        menu_state.set(MenuState::Main);
    }

    fn button_style() -> Style {
        Style {
            width: Val::Px(250.0),
            height: Val::Px(65.0),
            margin: UiRect::all(Val::Px(20.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        }
    }

    fn button_text_style() -> TextStyle {
        default_font(40., TEXT_COLOR)
    }

    // In the main menu and in the main settings menu, we create a column of buttons
    fn button(
        parent: &mut ChildBuilder,
        action: impl Bundle,
        spawn_children: impl FnOnce(&mut ChildBuilder),
    ) {
        parent
            .spawn((
                ButtonBundle {
                    style: button_style(),
                    image: UiImage::default().with_color(NORMAL_BUTTON),
                    ..default()
                },
                action,
            ))
            .with_children(spawn_children);
    }

    fn main_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        let button_icon_style = Style {
            width: Val::Px(30.0),
            // This takes the icons out of the flexbox flow, to be positioned exactly
            position_type: PositionType::Absolute,
            // The icon will be close to the left border of the button
            left: Val::Px(10.0),
            ..default()
        };

        inner_node(&mut commands, OnMainMenuScreen, RED, |parent| {
            // Display the game name
            parent.spawn(
                TextBundle::from_section("Bevy Game Menu UI", default_font(80., TEXT_COLOR))
                    .with_style(Style {
                        margin: UiRect::all(Val::Px(50.0)),
                        ..default()
                    }),
            );

            // Display three buttons for each action available from the main menu
            for (action, icon, text) in [
                (MenuButtonAction::Play, "right", "New Game"),
                (MenuButtonAction::Settings, "wrench", "Settings"),
                (MenuButtonAction::Quit, "exitRight", "Quit"),
            ] {
                button(parent, action, |parent| {
                    let icon = asset_server.load(format!("textures/Game Icons/{}.png", icon));
                    parent.spawn(ImageBundle {
                        style: button_icon_style.clone(),
                        image: UiImage::new(icon),
                        ..default()
                    });
                    parent.spawn(TextBundle::from_section(text, button_text_style()));
                });
            }
        });
    }

    fn settings_menu_setup(mut commands: Commands) {
        inner_node(&mut commands, OnSettingsMenuScreen, RED, |parent| {
            // Display three buttons for each action available from the main settings menu
            for (action, text) in [
                (MenuButtonAction::SettingsDisplay, "Display"),
                (MenuButtonAction::SettingsSound, "Sound"),
                (MenuButtonAction::BackToMainMenu, "Back"),
            ] {
                button(parent, action, |parent| {
                    parent.spawn(TextBundle::from_section(text, button_text_style()));
                });
            }
        });
    }

    // A button to go back to the main settings menu from settings submenus
    fn back_to_settings(parent: &mut ChildBuilder) {
        // Display the back button to return to the settings screen
        parent
            .spawn((
                ButtonBundle {
                    style: button_style(),
                    image: UiImage::default().with_color(NORMAL_BUTTON),
                    ..default()
                },
                MenuButtonAction::BackToSettings,
            ))
            .with_children(|parent| {
                parent.spawn(TextBundle::from_section("Back", button_text_style()));
            });
    }

    fn display_settings_menu_setup(mut commands: Commands, display_quality: Res<DisplayQuality>) {
        inner_node(&mut commands, OnDisplaySettingsMenuScreen, RED, |parent| {
            // Create a new `NodeBundle`, this time not setting its `flex_direction`. It will
            // use the default value, `FlexDirection::Row`, from left to right.
            parent
                .spawn(NodeBundle {
                    style: Style {
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: RED.into(),
                    ..default()
                })
                .with_children(|parent| {
                    // Display a label for the current setting
                    parent.spawn(TextBundle::from_section(
                        "Display Quality",
                        button_text_style(),
                    ));
                    // Display a button for each possible value
                    for (quality_setting, text) in [
                        (DisplayQuality::Low, "Low"),
                        (DisplayQuality::Medium, "Medium"),
                        (DisplayQuality::High, "High"),
                    ] {
                        let mut entity = parent.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(150.0),
                                    height: Val::Px(65.0),
                                    ..button_style()
                                },
                                image: UiImage::default().with_color(NORMAL_BUTTON),
                                ..default()
                            },
                            quality_setting,
                        ));
                        entity.with_children(|parent| {
                            parent.spawn(TextBundle::from_section(text, button_text_style()));
                        });
                        if *display_quality == quality_setting {
                            entity.insert(SelectedOption);
                        }
                    }
                });

            // Display the back button to return to the settings screen
            back_to_settings(parent);
        });
    }

    fn sound_settings_menu_setup(mut commands: Commands, volume: Res<Volume>) {
        inner_node(&mut commands, OnSoundSettingsMenuScreen, RED, |parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    background_color: RED.into(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle::from_section("Volume", button_text_style()));
                    for volume_setting in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] {
                        let mut entity = parent.spawn((
                            ButtonBundle {
                                style: Style {
                                    width: Val::Px(30.0),
                                    height: Val::Px(65.0),
                                    ..button_style()
                                },
                                image: UiImage::default().with_color(NORMAL_BUTTON),
                                ..default()
                            },
                            Volume(volume_setting),
                        ));
                        if *volume == Volume(volume_setting) {
                            entity.insert(SelectedOption);
                        }
                    }
                });

            // Display the back button to return to the settings screen
            back_to_settings(parent);
        });
    }

    fn menu_action(
        interaction_query: Query<
            (&Interaction, &MenuButtonAction),
            (Changed<Interaction>, With<Button>),
        >,
        mut app_exit_events: EventWriter<AppExit>,
        mut menu_state: ResMut<NextState<MenuState>>,
        mut game_state: ResMut<NextState<GameState>>,
    ) {
        for (interaction, menu_button_action) in &interaction_query {
            if *interaction == Interaction::Pressed {
                match menu_button_action {
                    MenuButtonAction::Quit => {
                        app_exit_events.send(AppExit);
                    }
                    MenuButtonAction::Play => {
                        game_state.set(GameState::Game);
                        menu_state.set(MenuState::Disabled);
                    }
                    MenuButtonAction::Settings => menu_state.set(MenuState::Settings),
                    MenuButtonAction::SettingsDisplay => {
                        menu_state.set(MenuState::SettingsDisplay);
                    }
                    MenuButtonAction::SettingsSound => {
                        menu_state.set(MenuState::SettingsSound);
                    }
                    MenuButtonAction::BackToMainMenu => menu_state.set(MenuState::Main),
                    MenuButtonAction::BackToSettings => {
                        menu_state.set(MenuState::Settings);
                    }
                }
            }
        }
    }
}

// Generic system that takes a component as a parameter, and will despawn all entities with that component
fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in &to_despawn {
        commands.entity(entity).despawn_recursive();
    }
}
