//! This example will display a simple menu using Bevy UI where you can start a new game,
//! change some settings or quit. There is no actual game, it will just display the current
//! settings for 5 seconds before going back to the menu.

use bevy::prelude::*;

const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);

// Enum that will be used as a global state for the game
#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
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
        .add_startup_system(setup)
        // Declare the game state, and set its startup value
        .add_state(GameState::Splash)
        // Adds the plugins for each state
        .add_plugin(splash::SplashPlugin)
        .add_plugin(menu::MenuPlugin)
        .add_plugin(game::GamePlugin)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());
}

mod splash {
    use bevy::prelude::*;

    use crate::{despawn_screen, GameState};

    // This plugin will display a splash screen with Bevy logo for 1 second before switching to the menu
    pub struct SplashPlugin;

    impl Plugin for SplashPlugin {
        fn build(&self, app: &mut App) {
            // As this plugin is managing the splash screen, it will focus on the state `GameState::Splash`
            app
                // When entering the state, spawn everything needed for this screen
                .add_system_set(SystemSet::on_enter(GameState::Splash).with_system(splash_setup))
                // While in this state, run the `countdown` system
                .add_system_set(SystemSet::on_update(GameState::Splash).with_system(countdown))
                // When exiting the state, despawn everything that was spawned for this screen
                .add_system_set(
                    SystemSet::on_exit(GameState::Splash)
                        .with_system(despawn_screen::<OnSplashScreen>),
                );
        }
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
        commands
            .spawn_bundle(ImageBundle {
                style: Style {
                    // This will center the logo
                    margin: UiRect::all(Val::Auto),
                    // This will set the logo to be 200px wide, and auto adjust its height
                    size: Size::new(Val::Px(200.0), Val::Auto),
                    ..default()
                },
                image: UiImage(icon),
                ..default()
            })
            .insert(OnSplashScreen);
        // Insert the timer as a resource
        commands.insert_resource(SplashTimer(Timer::from_seconds(1.0, false)));
    }

    // Tick the timer, and change state when finished
    fn countdown(
        mut game_state: ResMut<State<GameState>>,
        time: Res<Time>,
        mut timer: ResMut<SplashTimer>,
    ) {
        if timer.tick(time.delta()).finished() {
            game_state.set(GameState::Menu).unwrap();
        }
    }
}

mod game {
    use bevy::prelude::*;

    use super::{despawn_screen, DisplayQuality, GameState, Volume, TEXT_COLOR};

    // This plugin will contain the game. In this case, it's just be a screen that will
    // display the current settings for 5 seconds before returning to the menu
    pub struct GamePlugin;

    impl Plugin for GamePlugin {
        fn build(&self, app: &mut App) {
            app.add_system_set(SystemSet::on_enter(GameState::Game).with_system(game_setup))
                .add_system_set(SystemSet::on_update(GameState::Game).with_system(game))
                .add_system_set(
                    SystemSet::on_exit(GameState::Game).with_system(despawn_screen::<OnGameScreen>),
                );
        }
    }

    // Tag component used to tag entities added on the game screen
    #[derive(Component)]
    struct OnGameScreen;

    #[derive(Resource, Deref, DerefMut)]
    struct GameTimer(Timer);

    fn game_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        display_quality: Res<DisplayQuality>,
        volume: Res<Volume>,
    ) {
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");

        commands
            // First create a `NodeBundle` for centering what we want to display
            .spawn_bundle(NodeBundle {
                style: Style {
                    // This will center the current node
                    margin: UiRect::all(Val::Auto),
                    // This will display its children in a column, from top to bottom. Unlike
                    // in Flexbox, Bevy origin is on bottom left, so the vertical axis is reversed
                    flex_direction: FlexDirection::ColumnReverse,
                    // `align_items` will align children on the cross axis. Here the main axis is
                    // vertical (column), so the cross axis is horizontal. This will center the
                    // children
                    align_items: AlignItems::Center,
                    ..default()
                },
                color: Color::BLACK.into(),
                ..default()
            })
            .insert(OnGameScreen)
            .with_children(|parent| {
                // Display two lines of text, the second one with the current settings
                parent.spawn_bundle(
                    TextBundle::from_section(
                        "Will be back to the menu shortly...",
                        TextStyle {
                            font: font.clone(),
                            font_size: 80.0,
                            color: TEXT_COLOR,
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::all(Val::Px(50.0)),
                        ..default()
                    }),
                );
                parent.spawn_bundle(
                    TextBundle::from_sections([
                        TextSection::new(
                            format!("quality: {:?}", *display_quality),
                            TextStyle {
                                font: font.clone(),
                                font_size: 60.0,
                                color: Color::BLUE,
                            },
                        ),
                        TextSection::new(
                            " - ",
                            TextStyle {
                                font: font.clone(),
                                font_size: 60.0,
                                color: TEXT_COLOR,
                            },
                        ),
                        TextSection::new(
                            format!("volume: {:?}", *volume),
                            TextStyle {
                                font: font.clone(),
                                font_size: 60.0,
                                color: Color::GREEN,
                            },
                        ),
                    ])
                    .with_style(Style {
                        margin: UiRect::all(Val::Px(50.0)),
                        ..default()
                    }),
                );
            });
        // Spawn a 5 seconds timer to trigger going back to the menu
        commands.insert_resource(GameTimer(Timer::from_seconds(5.0, false)));
    }

    // Tick the timer, and change state when finished
    fn game(
        time: Res<Time>,
        mut game_state: ResMut<State<GameState>>,
        mut timer: ResMut<GameTimer>,
    ) {
        if timer.tick(time.delta()).finished() {
            game_state.set(GameState::Menu).unwrap();
        }
    }
}

mod menu {
    use bevy::{app::AppExit, prelude::*, ui_navigation::NavRequestSystem};

    use super::{
        despawn_screen, mark_buttons, DisplayQuality, GameState, MarkButtons, ParentMenu, Volume,
        TEXT_COLOR,
    };

    // This plugin manages the menu, with 5 different screens:
    // - a main menu with "New Game", "Settings", "Quit"
    // - a settings menu with two submenus and a back button
    // - two settings screen with a setting that can be set and a back button
    pub struct MenuPlugin;

    impl Plugin for MenuPlugin {
        fn build(&self, app: &mut App) {
            app.add_system_set(
                SystemSet::on_enter(GameState::Menu)
                    .with_system(main_menu_setup)
                    .with_system(settings_menu_setup)
                    .with_system(display_settings_menu_setup)
                    .with_system(sound_settings_menu_setup),
            )
            .add_system_set(
                SystemSet::on_update(GameState::Menu)
                    .with_system(mark_buttons)
                    .with_system(update_selected_option::<DisplayQuality>.after(NavRequestSystem))
                    .with_system(update_selected_option::<Volume>.after(NavRequestSystem))
                    .with_system(handle_menu_change.after(NavRequestSystem))
                    .with_system(menu_action.after(NavRequestSystem))
                    .with_system(update_button_color.after(NavRequestSystem)),
            )
            .add_system_set(
                SystemSet::on_exit(GameState::Menu).with_system(despawn_screen::<MenuSetting>),
            );
        }
    }

    const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
    const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
    const HOVERED_PRESSED_BUTTON: Color = Color::rgb(0.25, 0.65, 0.25);
    const PRESSED_BUTTON: Color = Color::rgb(0.35, 0.75, 0.35);

    // Tag component used to mark wich setting is currently selected
    #[derive(Component)]
    enum SelectedOption {
        Selected,
        Unselected,
    }

    // All actions that can be triggered from a button click
    #[derive(Component)]
    enum MenuButtonAction {
        Play,
        Quit,
    }

    fn main_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");
        // Common style for all buttons on the screen
        let button_style = Style {
            size: Size::new(Val::Px(250.0), Val::Px(65.0)),
            margin: UiRect::all(Val::Px(20.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        };
        let button_icon_style = Style {
            size: Size::new(Val::Px(30.0), Val::Auto),
            // This takes the icons out of the flexbox flow, to be positioned exactly
            position_type: PositionType::Absolute,
            // The icon will be close to the left border of the button
            position: UiRect {
                left: Val::Px(10.0),
                right: Val::Auto,
                top: Val::Auto,
                bottom: Val::Auto,
            },
            ..default()
        };
        let button_text_style = TextStyle {
            font: font.clone(),
            font_size: 40.0,
            color: TEXT_COLOR,
        };

        commands
            .spawn_bundle(MenuBundle {
                node_bundle: NodeBundle {
                    style: Style {
                        margin: UiRect::all(Val::Auto),
                        flex_direction: FlexDirection::ColumnReverse,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    color: Color::CRIMSON.into(),
                    ..default()
                },
                setting: MenuSetting {
                    wrapping: true,
                    scope: false,
                },
                menu: MenuBuilder::Root,
            })
            .insert(MarkButtons)
            .with_children(|parent| {
                // Display the game name
                parent.spawn_bundle(
                    TextBundle::from_section(
                        "Bevy Game Menu UI",
                        TextStyle {
                            font: font.clone(),
                            font_size: 80.0,
                            color: TEXT_COLOR,
                        },
                    )
                    .with_style(Style {
                        margin: UiRect::all(Val::Px(50.0)),
                        ..default()
                    }),
                );

                // Display three buttons for each action available from the main menu:
                // - new game
                // - settings
                // - quit
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style.clone(),
                        color: NORMAL_BUTTON.into(),
                        // NOTE: we make sure to start navigation in the main menu
                        // on the "New Game" button
                        focusable: Focusable::new().prioritized(),
                        ..default()
                    })
                    .insert(MenuButtonAction::Play)
                    .with_children(|parent| {
                        let icon = asset_server.load("textures/Game Icons/right.png");
                        parent.spawn_bundle(ImageBundle {
                            style: button_icon_style.clone(),
                            image: UiImage(icon),
                            ..default()
                        });
                        parent.spawn_bundle(TextBundle::from_section(
                            "New Game",
                            button_text_style.clone(),
                        ));
                    });
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style.clone(),
                        color: NORMAL_BUTTON.into(),
                        ..default()
                    })
                    .insert(Name::new("Settings"))
                    .with_children(|parent| {
                        let icon = asset_server.load("textures/Game Icons/wrench.png");
                        parent.spawn_bundle(ImageBundle {
                            style: button_icon_style.clone(),
                            image: UiImage(icon),
                            ..default()
                        });
                        parent.spawn_bundle(TextBundle::from_section(
                            "Settings",
                            button_text_style.clone(),
                        ));
                    });
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style,
                        color: NORMAL_BUTTON.into(),
                        ..default()
                    })
                    .insert(MenuButtonAction::Quit)
                    .with_children(|parent| {
                        let icon = asset_server.load("textures/Game Icons/exitRight.png");
                        parent.spawn_bundle(ImageBundle {
                            style: button_icon_style,
                            image: UiImage(icon),
                            ..default()
                        });
                        parent.spawn_bundle(TextBundle::from_section("Quit", button_text_style));
                    });
            });
    }

    fn settings_menu_setup(mut commands: Commands, asset_server: Res<AssetServer>) {
        let button_style = Style {
            size: Size::new(Val::Px(200.0), Val::Px(65.0)),
            margin: UiRect::all(Val::Px(20.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        };

        let button_text_style = TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 40.0,
            color: TEXT_COLOR,
        };

        commands
            .spawn_bundle(MenuBundle {
                node_bundle: NodeBundle {
                    style: Style {
                        display: Display::None,
                        margin: UiRect::all(Val::Auto),
                        flex_direction: FlexDirection::ColumnReverse,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    color: Color::CRIMSON.into(),
                    ..default()
                },
                menu: MenuBuilder::NamedParent(Name::new("Settings")),
                setting: default(),
            })
            .insert(MarkButtons)
            .with_children(|parent| {
                for (action, text) in [
                    (Focusable::default(), "Display"),
                    (Focusable::default(), "Sound"),
                    (Focusable::cancel(), "Back"),
                ] {
                    parent
                        .spawn_bundle(ButtonBundle {
                            style: button_style.clone(),
                            color: NORMAL_BUTTON.into(),
                            focusable: action,
                            ..default()
                        })
                        .insert(Name::new(text))
                        .with_children(|parent| {
                            parent.spawn_bundle(TextBundle::from_section(
                                text,
                                button_text_style.clone(),
                            ));
                        });
                }
            });
    }

    fn display_settings_menu_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        display_quality: Res<DisplayQuality>,
    ) {
        let button_style = Style {
            size: Size::new(Val::Px(200.0), Val::Px(65.0)),
            margin: UiRect::all(Val::Px(20.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        };
        let button_text_style = TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 40.0,
            color: TEXT_COLOR,
        };

        commands
            .spawn_bundle(MenuBundle {
                node_bundle: NodeBundle {
                    style: Style {
                        display: Display::None,
                        margin: UiRect::all(Val::Auto),
                        flex_direction: FlexDirection::ColumnReverse,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    color: Color::CRIMSON.into(),
                    ..default()
                },
                menu: MenuBuilder::NamedParent(Name::new("Display")),
                setting: default(),
            })
            .insert(MarkButtons)
            .with_children(|parent| {
                // Create a new `NodeBundle`, this time not setting its `flex_direction`. It will
                // use the default value, `FlexDirection::Row`, from left to right.
                parent
                    .spawn_bundle(NodeBundle {
                        style: Style {
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        color: Color::CRIMSON.into(),
                        ..default()
                    })
                    .with_children(|parent| {
                        // Display a label for the current setting
                        parent.spawn_bundle(TextBundle::from_section(
                            "Display Quality",
                            button_text_style.clone(),
                        ));
                        // Display a button for each possible value
                        for quality_setting in [
                            DisplayQuality::Low,
                            DisplayQuality::Medium,
                            DisplayQuality::High,
                        ] {
                            let mut entity = parent.spawn_bundle(ButtonBundle {
                                style: Style {
                                    size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                                    ..button_style.clone()
                                },
                                color: NORMAL_BUTTON.into(),
                                ..default()
                            });
                            entity.insert(quality_setting).with_children(|parent| {
                                parent.spawn_bundle(TextBundle::from_section(
                                    format!("{quality_setting:?}"),
                                    button_text_style.clone(),
                                ));
                            });
                            let selected = if *display_quality == quality_setting {
                                SelectedOption::Selected
                            } else {
                                SelectedOption::Unselected
                            };
                            entity.insert(selected);
                        }
                    });
                // Display the back button to return to the settings screen
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style,
                        color: NORMAL_BUTTON.into(),
                        focusable: Focusable::cancel(),
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle::from_section("Back", button_text_style));
                    });
            });
    }

    fn sound_settings_menu_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        volume: Res<Volume>,
    ) {
        let button_style = Style {
            size: Size::new(Val::Px(200.0), Val::Px(65.0)),
            margin: UiRect::all(Val::Px(20.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        };
        let button_text_style = TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 40.0,
            color: TEXT_COLOR,
        };

        commands
            .spawn_bundle(MenuBundle {
                node_bundle: NodeBundle {
                    style: Style {
                        display: Display::None,
                        margin: UiRect::all(Val::Auto),
                        flex_direction: FlexDirection::ColumnReverse,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    color: Color::CRIMSON.into(),
                    ..default()
                },
                menu: MenuBuilder::NamedParent(Name::new("Sound")),
                setting: default(),
            })
            .insert(MarkButtons)
            .with_children(|parent| {
                parent
                    .spawn_bundle(NodeBundle {
                        style: Style {
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        color: Color::CRIMSON.into(),
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle::from_section(
                            "Volume",
                            button_text_style.clone(),
                        ));
                        for volume_setting in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] {
                            let mut entity = parent.spawn_bundle(ButtonBundle {
                                style: Style {
                                    size: Size::new(Val::Px(30.0), Val::Px(65.0)),
                                    ..button_style.clone()
                                },
                                color: NORMAL_BUTTON.into(),
                                ..default()
                            });
                            entity.insert(Volume(volume_setting));
                            let selected = if *volume == Volume(volume_setting) {
                                SelectedOption::Selected
                            } else {
                                SelectedOption::Unselected
                            };
                            entity.insert(selected);
                        }
                    });
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style,
                        color: NORMAL_BUTTON.into(),
                        focusable: Focusable::cancel(),
                        ..default()
                    })
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle::from_section("Back", button_text_style));
                    });
            });
    }

    // This system handles changing all buttons color based on mouse interaction
    fn update_button_color(
        mut interaction_query: Query<
            (&Focusable, Option<&SelectedOption>, &mut UiColor),
            Or<(Changed<Focusable>, Changed<SelectedOption>)>,
        >,
    ) {
        use FocusState::*;
        use SelectedOption::*;
        for (interaction, selected, mut color) in &mut interaction_query {
            let new_color = match (interaction.state(), selected) {
                (Focused, Some(Selected)) => HOVERED_PRESSED_BUTTON,
                (Focused, Some(Unselected) | None) => HOVERED_BUTTON,
                (_, Some(Selected)) => PRESSED_BUTTON,
                (_, Some(Unselected) | None) => NORMAL_BUTTON,
            };
            *color = new_color.into();
        }
    }

    fn update_selected_option<T: Resource + Component + PartialEq + Copy>(
        mut nav_events: EventReader<NavEvent>,
        mut select_query: Query<(Entity, &mut SelectedOption, &T)>,
        mut setting: ResMut<T>,
    ) {
        for event in nav_events.iter() {
            if let NavEvent::NoChanges {
                from,
                request: NavRequest::Action,
            } = event
            {
                let activated = *from.first();
                // skip if the update is from another kind of setting option
                if select_query.get(activated).is_err() {
                    continue;
                }
                let old_setting = *setting;
                for (entity, mut to_change, option_value) in &mut select_query {
                    if *option_value == old_setting {
                        *to_change = SelectedOption::Unselected;
                    }
                    if entity == activated {
                        *to_change = SelectedOption::Selected;
                        *setting = *option_value;
                    }
                }
            }
        }
    }

    fn handle_menu_change(
        mut nav_events: EventReader<NavEvent>,
        mut styles: Query<&mut Style>,
        menu_query: Query<&ParentMenu>,
    ) {
        for event in nav_events.iter() {
            if let NavEvent::FocusChanged { to, from } = event {
                let menu_query = (menu_query.get(*from.first()), menu_query.get(*to.first()));
                if let (Ok(from), Ok(to)) = menu_query {
                    // Could improve perf by using `if from != to {...}` here
                    let mut to_hide = styles.get_mut(from.0).unwrap();
                    to_hide.display = Display::None;

                    let mut to_show = styles.get_mut(to.0).unwrap();
                    to_show.display = Display::Flex;
                }
            }
        }
    }

    fn menu_action(
        button_query: Query<&MenuButtonAction>,
        mut nav_events: EventReader<NavEvent>,
        mut app_exit_events: EventWriter<AppExit>,
        mut game_state: ResMut<State<GameState>>,
    ) {
        for event in nav_events.iter() {
            if let NavEvent::NoChanges {
                from,
                request: NavRequest::Action,
            } = event
            {
                match button_query.get(*from.first()) {
                    Ok(MenuButtonAction::Quit) => app_exit_events.send(AppExit),
                    Ok(MenuButtonAction::Play) => {
                        game_state.set(GameState::Game).unwrap();
                    }
                    _ => {}
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

#[derive(Component, Clone, Copy, PartialEq)]
struct ParentMenu(Entity);
#[derive(Component)]
struct MarkButtons;

// TODO: note that bevy-ui-navigation had a dedicated module to automate this.
/// This system adds a component that links directly to the parent menu of a focusable.
fn mark_buttons(
    mut cmds: Commands,
    menu_markers: Query<Entity, With<MarkButtons>>,
    focusables: Query<(), With<Focusable>>,
    menus: Query<(), With<MenuSetting>>,
    children: Query<&Children>,
) {
    fn mark_focusable(
        entity_children: &Children,
        marker: ParentMenu,
        commands: &mut Commands,
        focusables: &Query<(), With<Focusable>>,
        menus: &Query<(), With<MenuSetting>>,
        children: &Query<&Children>,
    ) {
        for entity in entity_children {
            match () {
                () if focusables.get(*entity).is_ok() => {
                    commands.entity(*entity).insert(marker);
                }
                () if menus.get(*entity).is_ok() => {}
                () => {
                    if let Ok(entities) = children.get(*entity) {
                        mark_focusable(entities, marker, commands, focusables, menus, children);
                    }
                }
            }
        }
    }
    for menu in &menu_markers {
        if let Ok(entities) = children.get(menu) {
            let marker = ParentMenu(menu);
            mark_focusable(entities, marker, &mut cmds, &focusables, &menus, &children);
        }
        cmds.entity(menu).remove::<MarkButtons>();
    }
}
