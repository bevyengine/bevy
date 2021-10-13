use bevy::prelude::*;

// This example will display a simple menu using Bevy UI where you can start a new game,
// change some settings or quit. There are no actual game, it will just display the current
// settings for 5 seconds before going back to the menu.

const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    Splash,
    Menu,
    Game,
}

#[derive(Debug, Component, PartialEq, Eq, Clone, Copy)]
enum DisplayQuality {
    Low,
    Medium,
    High,
}

// Simple settings struct, it will be added as a resource to the Bevy app
#[derive(Debug)]
struct Settings {
    quality: DisplayQuality,
    volume: u32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Insert the settings struct as a resource with its initial values
        .insert_resource(Settings {
            quality: DisplayQuality::Medium,
            volume: 7,
        })
        .add_startup_system(setup)
        // Start the game in the Splash state
        .add_state(GameState::Splash)
        // Adds the plugins for each state
        .add_plugin(splash::SplashPlugin)
        .add_plugin(menu::MenuPlugin)
        .add_plugin(game::GamePlugin)
        .run();
}

// As there isn't an actual game, setup is just adding a `UiCameraBundle`
fn setup(mut commands: Commands) {
    commands.spawn_bundle(UiCameraBundle::default());
}

mod splash {
    use bevy::prelude::*;

    use super::{despawn_screen, GameState};
    // This plugin will display a splash screen with Bevy logo for 1 second before switching to the menu
    pub struct SplashPlugin;

    impl Plugin for SplashPlugin {
        fn build(&self, app: &mut bevy::prelude::App) {
            app.add_system_set(SystemSet::on_enter(GameState::Splash).with_system(splash_setup))
                .add_system_set(SystemSet::on_update(GameState::Splash).with_system(countdown))
                .add_system_set(
                    SystemSet::on_exit(GameState::Splash)
                        .with_system(despawn_screen::<ScreenSplash>),
                );
        }
    }

    // Tag component used to tag entities added on the splash screen
    #[derive(Component)]
    struct ScreenSplash;

    #[derive(Component)]
    struct SplashTimer(Timer);

    fn splash_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        let icon = asset_server.load("branding/icon.png");
        // Display the logo
        commands
            .spawn_bundle(ImageBundle {
                style: Style {
                    // This will center the logo
                    margin: Rect::all(Val::Auto),
                    // This will set the logo to be 200px wide, and auto adjust its height
                    size: Size::new(Val::Px(200.0), Val::Auto),
                    ..Default::default()
                },
                material: materials.add(icon.into()),
                ..Default::default()
            })
            .insert(ScreenSplash)
            .insert(SplashTimer(Timer::from_seconds(1.0, false)));
    }

    // Tick the timer, and change state when finished
    fn countdown(
        mut game_state: ResMut<State<GameState>>,
        time: Res<Time>,
        mut timer: Query<&mut SplashTimer>,
    ) {
        if timer.single_mut().0.tick(time.delta()).finished() {
            game_state.set(GameState::Menu).unwrap();
        }
    }
}

mod game {
    use bevy::prelude::*;

    use super::{despawn_screen, GameState, Settings, TEXT_COLOR};

    // This plugin will contain the game. In this case, it's just be a screen that will
    // display the current settings for 5 seconds before returning to the menu
    pub struct GamePlugin;

    impl Plugin for GamePlugin {
        fn build(&self, app: &mut App) {
            app.add_system_set(SystemSet::on_enter(GameState::Game).with_system(game_setup))
                .add_system_set(SystemSet::on_update(GameState::Game).with_system(game))
                .add_system_set(
                    SystemSet::on_exit(GameState::Game).with_system(despawn_screen::<ScreenGame>),
                );
        }
    }

    // Tag component used to tag entities added on the game screen
    #[derive(Component)]
    struct ScreenGame;

    #[derive(Component)]
    struct GameTimer(Timer);

    fn game_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut materials: ResMut<Assets<ColorMaterial>>,
        settings: Res<Settings>,
    ) {
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");

        commands
            // First create a `NodeBundle` for centering what we want to display
            .spawn_bundle(NodeBundle {
                style: Style {
                    // This will center the current node
                    margin: Rect::all(Val::Auto),
                    // This will display its children in a column, from top to bottom
                    flex_direction: FlexDirection::ColumnReverse,
                    // `align_items` will align children on the cross axis. Here the main axis is
                    // vertical (column), so the cross axis is horizontal. This will center the
                    // children
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                material: materials.add(Color::BLACK.into()),
                ..Default::default()
            })
            .insert(ScreenGame)
            .with_children(|parent| {
                // Display two lines of text, the second one with the current settings
                parent.spawn_bundle(TextBundle {
                    style: Style {
                        margin: Rect::all(Val::Px(50.0)),
                        ..Default::default()
                    },
                    text: Text::with_section(
                        "Will be back to the menu shortly...",
                        TextStyle {
                            font: font.clone(),
                            font_size: 80.0,
                            color: TEXT_COLOR,
                        },
                        Default::default(),
                    ),
                    ..Default::default()
                });
                parent.spawn_bundle(TextBundle {
                    style: Style {
                        margin: Rect::all(Val::Px(50.0)),
                        ..Default::default()
                    },
                    text: Text::with_section(
                        format!("{:?}", *settings),
                        TextStyle {
                            font: font.clone(),
                            font_size: 60.0,
                            color: TEXT_COLOR,
                        },
                        Default::default(),
                    ),
                    ..Default::default()
                });
            });
        // Spawn a 5 timer to trigger going back to the menu
        commands.spawn_bundle((GameTimer(Timer::from_seconds(5.0, false)), ScreenGame));
    }

    // Tick the timer, and change state when finished
    fn game(
        time: Res<Time>,
        mut game_state: ResMut<State<GameState>>,
        mut timer: Query<&mut GameTimer>,
    ) {
        if timer.single_mut().0.tick(time.delta()).finished() {
            game_state.set(GameState::Menu).unwrap();
        }
    }
}

mod menu {
    use bevy::{app::AppExit, prelude::*};

    use super::{despawn_screen, DisplayQuality, GameState, Settings, TEXT_COLOR};

    // This plugin manages the menu, with 5 different screens:
    // - a main menu with "New Game", "Settings", "Quit"
    // - a settings menu with two submenus and a back button
    // - two settings screen with a setting that can be set and a back button
    pub struct MenuPlugin;

    impl Plugin for MenuPlugin {
        fn build(&self, app: &mut bevy::prelude::App) {
            app.init_resource::<ButtonMaterials>()
                // At start, the menu is not enabled. This will be changed in `menu_setup` when
                // entering the `GameState::Menu` state.
                // Current screen in the menu is handled by an indepent state from `GameState`
                .add_state(MenuState::Disabled)
                .add_system_set(SystemSet::on_enter(GameState::Menu).with_system(menu_setup))
                // Systems to handle the main menu screen
                .add_system_set(SystemSet::on_enter(MenuState::Main).with_system(main_menu_setup))
                .add_system_set(
                    SystemSet::on_exit(MenuState::Main)
                        .with_system(despawn_screen::<ScreenMenuMain>),
                )
                // Systems to handle the settings menu screen
                .add_system_set(
                    SystemSet::on_enter(MenuState::Settings).with_system(settings_menu_setup),
                )
                .add_system_set(
                    SystemSet::on_exit(MenuState::Settings)
                        .with_system(despawn_screen::<ScreenMenuSettings>),
                )
                // Systems to handle the display settings screen
                .add_system_set(
                    SystemSet::on_enter(MenuState::SettingsDisplay)
                        .with_system(display_settings_menu_setup),
                )
                .add_system_set(
                    SystemSet::on_update(MenuState::SettingsDisplay).with_system(quality_button),
                )
                .add_system_set(
                    SystemSet::on_exit(MenuState::SettingsDisplay)
                        .with_system(despawn_screen::<ScreenMenuSettingsDisplay>),
                )
                // Systems to handle the sound settings screen
                .add_system_set(
                    SystemSet::on_enter(MenuState::SettingsSound)
                        .with_system(sound_settings_menu_setup),
                )
                .add_system_set(
                    SystemSet::on_update(MenuState::SettingsSound).with_system(volume_button),
                )
                .add_system_set(
                    SystemSet::on_exit(MenuState::SettingsSound)
                        .with_system(despawn_screen::<ScreenMenuSettingsSound>),
                )
                // Common systems to all screens that handles buttons behaviour
                .add_system_set(
                    SystemSet::on_update(GameState::Menu)
                        .with_system(menu_action)
                        .with_system(button_system),
                );
        }
    }

    // State used for the current menu screen
    #[derive(Clone, Eq, PartialEq, Debug, Hash)]
    enum MenuState {
        Main,
        Settings,
        SettingsDisplay,
        SettingsSound,
        Disabled,
    }

    // Tag component used to tag entities added on the main menu screen
    #[derive(Component)]
    struct ScreenMenuMain;

    // Tag component used to tag entities added on the settings menu screen
    #[derive(Component)]
    struct ScreenMenuSettings;

    // Tag component used to tag entities added on the display settings menu screen
    #[derive(Component)]
    struct ScreenMenuSettingsDisplay;

    // Tag component used to tag entities added on the sound settings menu screen
    #[derive(Component)]
    struct ScreenMenuSettingsSound;

    struct ButtonMaterials {
        normal: Handle<ColorMaterial>,
        hovered: Handle<ColorMaterial>,
        hovered_pressed: Handle<ColorMaterial>,
        pressed: Handle<ColorMaterial>,
    }

    impl FromWorld for ButtonMaterials {
        fn from_world(world: &mut World) -> Self {
            let mut materials = world.get_resource_mut::<Assets<ColorMaterial>>().unwrap();
            ButtonMaterials {
                normal: materials.add(Color::rgb(0.15, 0.15, 0.15).into()),
                hovered: materials.add(Color::rgb(0.25, 0.25, 0.25).into()),
                hovered_pressed: materials.add(Color::rgb(0.25, 0.65, 0.25).into()),
                pressed: materials.add(Color::rgb(0.35, 0.75, 0.35).into()),
            }
        }
    }

    // Tag component used to mark wich setting is currently selected
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
        button_materials: Res<ButtonMaterials>,
        mut interaction_query: Query<
            (
                &Interaction,
                &mut Handle<ColorMaterial>,
                Option<&SelectedOption>,
            ),
            (Changed<Interaction>, With<Button>),
        >,
    ) {
        for (interaction, mut material, selected) in interaction_query.iter_mut() {
            match *interaction {
                Interaction::Clicked => {
                    *material = button_materials.pressed.clone();
                }
                Interaction::Hovered => {
                    if selected.is_some() {
                        *material = button_materials.hovered_pressed.clone();
                    } else {
                        *material = button_materials.hovered.clone();
                    }
                }
                Interaction::None => {
                    if selected.is_some() {
                        *material = button_materials.pressed.clone();
                    } else {
                        *material = button_materials.normal.clone();
                    }
                }
            }
        }
    }

    // This system updates the settings when a new value for display quality is selected
    fn quality_button(
        button_materials: Res<ButtonMaterials>,
        interaction_query: Query<
            (&Interaction, &DisplayQuality, Entity),
            (Changed<Interaction>, With<Button>),
        >,
        mut selected_query: Query<(Entity, &mut Handle<ColorMaterial>), With<SelectedOption>>,
        mut commands: Commands,
        mut settings: ResMut<Settings>,
    ) {
        for (interaction, quality, entity) in interaction_query.iter() {
            if *interaction == Interaction::Clicked && settings.quality != *quality {
                let (previous_button, mut previous_material) = selected_query.single_mut();
                *previous_material = button_materials.normal.clone();
                commands.entity(previous_button).remove::<SelectedOption>();
                commands.entity(entity).insert(SelectedOption);
                settings.quality = *quality;
            }
        }
    }

    #[derive(Component)]
    struct Volume(u32);

    // This system updates the settings when a new value for volume is selected
    fn volume_button(
        button_materials: Res<ButtonMaterials>,
        interaction_query: Query<
            (&Interaction, &Volume, Entity),
            (Changed<Interaction>, With<Button>),
        >,
        mut selected_query: Query<(Entity, &mut Handle<ColorMaterial>), With<SelectedOption>>,
        mut commands: Commands,
        mut settings: ResMut<Settings>,
    ) {
        for (interaction, volume, entity) in interaction_query.iter() {
            if *interaction == Interaction::Clicked && settings.volume != volume.0 {
                let (previous_button, mut previous_material) = selected_query.single_mut();
                *previous_material = button_materials.normal.clone();
                commands.entity(previous_button).remove::<SelectedOption>();
                commands.entity(entity).insert(SelectedOption);
                settings.volume = volume.0;
            }
        }
    }

    fn menu_setup(mut menu_state: ResMut<State<MenuState>>) {
        let _ = menu_state.set(MenuState::Main);
    }

    fn main_menu_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        button_materials: Res<ButtonMaterials>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");
        // Common style for all buttons on the screen
        let button_style = Style {
            size: Size::new(Val::Px(200.0), Val::Px(65.0)),
            margin: Rect::all(Val::Px(20.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        };
        let button_text_style = TextStyle {
            font: font.clone(),
            font_size: 40.0,
            color: TEXT_COLOR,
        };

        commands
            .spawn_bundle(NodeBundle {
                style: Style {
                    margin: Rect::all(Val::Auto),
                    flex_direction: FlexDirection::ColumnReverse,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                material: materials.add(Color::CRIMSON.into()),
                ..Default::default()
            })
            .insert(ScreenMenuMain)
            .with_children(|parent| {
                // Display the game name
                parent.spawn_bundle(TextBundle {
                    style: Style {
                        margin: Rect::all(Val::Px(50.0)),
                        ..Default::default()
                    },
                    text: Text::with_section(
                        "Bevy Game Menu UI",
                        TextStyle {
                            font: font.clone(),
                            font_size: 80.0,
                            color: TEXT_COLOR,
                        },
                        Default::default(),
                    ),
                    ..Default::default()
                });

                // Display three buttons for each action available from the main menu:
                // - new game
                // - settings
                // - quit
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style.clone(),
                        material: button_materials.normal.clone(),
                        ..Default::default()
                    })
                    .insert(MenuButtonAction::Play)
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section(
                                "New Game",
                                button_text_style.clone(),
                                Default::default(),
                            ),
                            ..Default::default()
                        });
                    });
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style.clone(),
                        material: button_materials.normal.clone(),
                        ..Default::default()
                    })
                    .insert(MenuButtonAction::Settings)
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section(
                                "Settings",
                                button_text_style.clone(),
                                Default::default(),
                            ),
                            ..Default::default()
                        });
                    });
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style,
                        material: button_materials.normal.clone(),
                        ..Default::default()
                    })
                    .insert(MenuButtonAction::Quit)
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section("Quit", button_text_style, Default::default()),
                            ..Default::default()
                        });
                    });
            });
    }

    fn settings_menu_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        button_materials: Res<ButtonMaterials>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        let button_style = Style {
            size: Size::new(Val::Px(200.0), Val::Px(65.0)),
            margin: Rect::all(Val::Px(20.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        };
        let button_text_style = TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 40.0,
            color: TEXT_COLOR,
        };

        commands
            .spawn_bundle(NodeBundle {
                style: Style {
                    margin: Rect::all(Val::Auto),
                    flex_direction: FlexDirection::ColumnReverse,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                material: materials.add(Color::CRIMSON.into()),
                ..Default::default()
            })
            .insert(ScreenMenuSettings)
            .with_children(|parent| {
                // Display two buttons for the submenus
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style.clone(),
                        material: button_materials.normal.clone(),
                        ..Default::default()
                    })
                    .insert(MenuButtonAction::SettingsDisplay)
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section(
                                "Display",
                                button_text_style.clone(),
                                Default::default(),
                            ),
                            ..Default::default()
                        });
                    });
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style.clone(),
                        material: button_materials.normal.clone(),
                        ..Default::default()
                    })
                    .insert(MenuButtonAction::SettingsSound)
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section(
                                "Sound",
                                button_text_style.clone(),
                                Default::default(),
                            ),
                            ..Default::default()
                        });
                    });
                // Display the back button to return to the main menu screen
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style,
                        material: button_materials.normal.clone(),
                        ..Default::default()
                    })
                    .insert(MenuButtonAction::BackToMainMenu)
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section("Back", button_text_style, Default::default()),
                            ..Default::default()
                        });
                    });
            });
    }

    fn display_settings_menu_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        button_materials: Res<ButtonMaterials>,
        mut materials: ResMut<Assets<ColorMaterial>>,
        settings: Res<Settings>,
    ) {
        let button_style = Style {
            size: Size::new(Val::Px(200.0), Val::Px(65.0)),
            margin: Rect::all(Val::Px(20.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        };
        let button_text_style = TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 40.0,
            color: TEXT_COLOR,
        };

        commands
            .spawn_bundle(NodeBundle {
                style: Style {
                    margin: Rect::all(Val::Auto),
                    flex_direction: FlexDirection::ColumnReverse,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                material: materials.add(Color::CRIMSON.into()),
                ..Default::default()
            })
            .insert(ScreenMenuSettingsDisplay)
            .with_children(|parent| {
                // Create a new `NodeBundle`, this time not setting its `flex_direction`. It will
                // use the default value, `FlexDirection::Row`, from left to right.
                parent
                    .spawn_bundle(NodeBundle {
                        style: Style {
                            align_items: AlignItems::Center,
                            ..Default::default()
                        },
                        material: materials.add(Color::CRIMSON.into()),
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        // Display a label for the current setting
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section(
                                "Display Quality",
                                button_text_style.clone(),
                                Default::default(),
                            ),
                            ..Default::default()
                        });
                        // Display a button for each possible value
                        for quality in [
                            DisplayQuality::Low,
                            DisplayQuality::Medium,
                            DisplayQuality::High,
                        ] {
                            let mut entity = parent.spawn_bundle(ButtonBundle {
                                style: Style {
                                    size: Size::new(Val::Px(150.0), Val::Px(65.0)),
                                    ..button_style.clone()
                                },
                                material: button_materials.normal.clone(),
                                ..Default::default()
                            });
                            entity.insert(quality).with_children(|parent| {
                                parent.spawn_bundle(TextBundle {
                                    text: Text::with_section(
                                        format!("{:?}", quality),
                                        button_text_style.clone(),
                                        Default::default(),
                                    ),
                                    ..Default::default()
                                });
                            });
                            if settings.quality == quality {
                                entity.insert(SelectedOption);
                            }
                        }
                    });
                // Display the back button to return to the settings screen
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style,
                        material: button_materials.normal.clone(),
                        ..Default::default()
                    })
                    .insert(MenuButtonAction::BackToSettings)
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section("Back", button_text_style, Default::default()),
                            ..Default::default()
                        });
                    });
            });
    }

    fn sound_settings_menu_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        button_materials: Res<ButtonMaterials>,
        mut materials: ResMut<Assets<ColorMaterial>>,
        settings: Res<Settings>,
    ) {
        let button_style = Style {
            size: Size::new(Val::Px(200.0), Val::Px(65.0)),
            margin: Rect::all(Val::Px(20.0)),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        };
        let button_text_style = TextStyle {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 40.0,
            color: TEXT_COLOR,
        };

        commands
            .spawn_bundle(NodeBundle {
                style: Style {
                    margin: Rect::all(Val::Auto),
                    flex_direction: FlexDirection::ColumnReverse,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                material: materials.add(Color::CRIMSON.into()),
                ..Default::default()
            })
            .insert(ScreenMenuSettingsSound)
            .with_children(|parent| {
                parent
                    .spawn_bundle(NodeBundle {
                        style: Style {
                            align_items: AlignItems::Center,
                            ..Default::default()
                        },
                        material: materials.add(Color::CRIMSON.into()),
                        ..Default::default()
                    })
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section(
                                "Volume",
                                button_text_style.clone(),
                                Default::default(),
                            ),
                            ..Default::default()
                        });
                        for volume in [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] {
                            let mut entity = parent.spawn_bundle(ButtonBundle {
                                style: Style {
                                    size: Size::new(Val::Px(30.0), Val::Px(65.0)),
                                    ..button_style.clone()
                                },
                                material: button_materials.normal.clone(),
                                ..Default::default()
                            });
                            entity.insert(Volume(volume));
                            if settings.volume == volume {
                                entity.insert(SelectedOption);
                            }
                        }
                    });
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style,
                        material: button_materials.normal.clone(),
                        ..Default::default()
                    })
                    .insert(MenuButtonAction::BackToSettings)
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section("Back", button_text_style, Default::default()),
                            ..Default::default()
                        });
                    });
            });
    }

    fn menu_action(
        interaction_query: Query<
            (&Interaction, &MenuButtonAction),
            (Changed<Interaction>, With<Button>),
        >,
        mut app_exit_events: EventWriter<AppExit>,
        mut menu_state: ResMut<State<MenuState>>,
        mut game_state: ResMut<State<GameState>>,
    ) {
        for (interaction, menu_button_action) in interaction_query.iter() {
            if *interaction == Interaction::Clicked {
                match menu_button_action {
                    MenuButtonAction::Quit => app_exit_events.send(AppExit),
                    MenuButtonAction::Play => {
                        game_state.set(GameState::Game).unwrap();
                        menu_state.set(MenuState::Disabled).unwrap()
                    }
                    MenuButtonAction::Settings => menu_state.set(MenuState::Settings).unwrap(),
                    MenuButtonAction::SettingsDisplay => {
                        menu_state.set(MenuState::SettingsDisplay).unwrap()
                    }
                    MenuButtonAction::SettingsSound => {
                        menu_state.set(MenuState::SettingsSound).unwrap()
                    }
                    MenuButtonAction::BackToMainMenu => menu_state.set(MenuState::Main).unwrap(),
                    MenuButtonAction::BackToSettings => {
                        menu_state.set(MenuState::Settings).unwrap()
                    }
                }
            }
        }
    }
}

fn despawn_screen<T: Component>(to_despawn: Query<Entity, With<T>>, mut commands: Commands) {
    for entity in to_despawn.iter() {
        commands.entity(entity).despawn_recursive();
    }
}
