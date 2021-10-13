use bevy::prelude::*;

const TEXT_COLOR: Color = Color::rgb(0.9, 0.9, 0.9);

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
enum GameState {
    Menu,
    Game,
}

#[derive(Debug, Component, PartialEq, Eq, Clone, Copy)]
enum DisplayQuality {
    Low,
    Medium,
    High,
}

#[derive(Debug)]
struct Settings {
    quality: DisplayQuality,
    volume: u32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Settings {
            quality: DisplayQuality::Medium,
            volume: 5,
        })
        .add_startup_system(setup)
        .add_state(GameState::Menu)
        .add_plugin(menu::MenuPlugin)
        .add_plugin(game::GamePlugin)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn_bundle(UiCameraBundle::default());
}

mod game {
    use bevy::prelude::*;

    pub struct GamePlugin;

    impl Plugin for GamePlugin {
        fn build(&self, app: &mut App) {
            app.add_system_set(SystemSet::on_enter(super::GameState::Game).with_system(game_setup))
                .add_system_set(SystemSet::on_update(super::GameState::Game).with_system(game))
                .add_system_set(
                    SystemSet::on_exit(super::GameState::Game)
                        .with_system(super::despawn_screen::<ScreenGame>),
                );
        }
    }

    #[derive(Component)]
    struct ScreenGame;

    #[derive(Component)]
    struct GameTimer(Timer);

    fn game_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        mut materials: ResMut<Assets<ColorMaterial>>,
        settings: Res<super::Settings>,
    ) {
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");

        commands
            .spawn_bundle(NodeBundle {
                style: Style {
                    margin: Rect::all(Val::Auto),
                    flex_direction: FlexDirection::ColumnReverse,
                    align_items: AlignItems::Center,
                    ..Default::default()
                },
                material: materials.add(Color::BLACK.into()),
                ..Default::default()
            })
            .insert(ScreenGame)
            .with_children(|parent| {
                parent.spawn_bundle(TextBundle {
                    style: Style {
                        margin: Rect::all(Val::Px(50.0)),
                        ..Default::default()
                    },
                    text: Text::with_section(
                        "Good Game!",
                        TextStyle {
                            font: font.clone(),
                            font_size: 80.0,
                            color: super::TEXT_COLOR,
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
                        "Will be back to the menu shortly...",
                        TextStyle {
                            font: font.clone(),
                            font_size: 80.0,
                            color: super::TEXT_COLOR,
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
                            font_size: 40.0,
                            color: super::TEXT_COLOR,
                        },
                        Default::default(),
                    ),
                    ..Default::default()
                });
            });
        commands.spawn_bundle((GameTimer(Timer::from_seconds(5.0, false)), ScreenGame));
    }

    fn game(
        time: Res<Time>,
        mut game_state: ResMut<State<super::GameState>>,
        mut timer: Query<&mut GameTimer>,
    ) {
        if timer.single_mut().0.tick(time.delta()).finished() {
            game_state.set(super::GameState::Menu).unwrap();
        }
    }
}

mod menu {
    use bevy::{app::AppExit, prelude::*};

    pub struct MenuPlugin;

    impl Plugin for MenuPlugin {
        fn build(&self, app: &mut bevy::prelude::App) {
            app.init_resource::<ButtonMaterials>()
                .add_state(MenuState::Disabled)
                .add_system_set(SystemSet::on_enter(super::GameState::Menu).with_system(menu_setup))
                .add_system_set(SystemSet::on_enter(MenuState::Main).with_system(main_menu_setup))
                .add_system_set(
                    SystemSet::on_exit(MenuState::Main)
                        .with_system(super::despawn_screen::<ScreenMenuMain>),
                )
                .add_system_set(
                    SystemSet::on_enter(MenuState::Preferences).with_system(preferences_menu_setup),
                )
                .add_system_set(
                    SystemSet::on_exit(MenuState::Preferences)
                        .with_system(super::despawn_screen::<ScreenMenuPreferences>),
                )
                .add_system_set(
                    SystemSet::on_enter(MenuState::PrefDisplay)
                        .with_system(display_preferences_menu_setup),
                )
                .add_system_set(
                    SystemSet::on_update(MenuState::PrefDisplay).with_system(quality_button),
                )
                .add_system_set(
                    SystemSet::on_exit(MenuState::PrefDisplay)
                        .with_system(super::despawn_screen::<ScreenMenuPrefDisplay>),
                )
                .add_system_set(
                    SystemSet::on_enter(MenuState::PrefSound)
                        .with_system(sound_preferences_menu_setup),
                )
                .add_system_set(
                    SystemSet::on_update(MenuState::PrefSound).with_system(volume_button),
                )
                .add_system_set(
                    SystemSet::on_exit(MenuState::PrefSound)
                        .with_system(super::despawn_screen::<ScreenMenuPrefSound>),
                )
                .add_system_set(
                    SystemSet::on_update(super::GameState::Menu)
                        .with_system(menu_action)
                        .with_system(button_system),
                );
        }
    }

    #[derive(Clone, Eq, PartialEq, Debug, Hash)]
    enum MenuState {
        Main,
        Preferences,
        PrefDisplay,
        PrefSound,
        Disabled,
    }
    #[derive(Component)]
    struct ScreenMenuMain;

    #[derive(Component)]
    struct ScreenMenuPreferences;
    #[derive(Component)]
    struct ScreenMenuPrefDisplay;
    #[derive(Component)]
    struct ScreenMenuPrefSound;

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

    #[derive(Component)]
    struct SelectedOption;

    #[derive(Component)]
    enum MenuButtonAction {
        Play,
        Preferences,
        PrefDisplay,
        PrefSound,
        BackToMainMenu,
        BackToPreferences,
        Quit,
    }

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

    fn quality_button(
        button_materials: Res<ButtonMaterials>,
        interaction_query: Query<
            (&Interaction, &super::DisplayQuality, Entity),
            (Changed<Interaction>, With<Button>),
        >,
        mut selected_query: Query<(Entity, &mut Handle<ColorMaterial>), With<SelectedOption>>,
        mut commands: Commands,
        mut settings: ResMut<super::Settings>,
    ) {
        for (interaction, quality, entity) in interaction_query.iter() {
            match *interaction {
                Interaction::Clicked => {
                    if settings.quality != *quality {
                        let (previous_button, mut previous_material) = selected_query.single_mut();
                        *previous_material = button_materials.normal.clone();
                        commands.entity(previous_button).remove::<SelectedOption>();
                        commands.entity(entity).insert(SelectedOption);
                        settings.quality = *quality;
                    }
                }
                _ => (),
            }
        }
    }

    #[derive(Component)]
    struct Volume(u32);

    fn volume_button(
        button_materials: Res<ButtonMaterials>,
        interaction_query: Query<
            (&Interaction, &Volume, Entity),
            (Changed<Interaction>, With<Button>),
        >,
        mut selected_query: Query<(Entity, &mut Handle<ColorMaterial>), With<SelectedOption>>,
        mut commands: Commands,
        mut settings: ResMut<super::Settings>,
    ) {
        for (interaction, volume, entity) in interaction_query.iter() {
            match *interaction {
                Interaction::Clicked => {
                    if settings.volume != volume.0 {
                        let (previous_button, mut previous_material) = selected_query.single_mut();
                        *previous_material = button_materials.normal.clone();
                        commands.entity(previous_button).remove::<SelectedOption>();
                        commands.entity(entity).insert(SelectedOption);
                        settings.volume = volume.0;
                    }
                }
                _ => (),
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
            color: super::TEXT_COLOR,
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
                            color: super::TEXT_COLOR,
                        },
                        Default::default(),
                    ),
                    ..Default::default()
                });
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
                    .insert(MenuButtonAction::Preferences)
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section(
                                "Preferences",
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

    fn preferences_menu_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        button_materials: Res<ButtonMaterials>,
        mut materials: ResMut<Assets<ColorMaterial>>,
    ) {
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");

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
            color: super::TEXT_COLOR,
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
            .insert(ScreenMenuPreferences)
            .with_children(|parent| {
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style.clone(),
                        material: button_materials.normal.clone(),
                        ..Default::default()
                    })
                    .insert(MenuButtonAction::PrefDisplay)
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
                    .insert(MenuButtonAction::PrefSound)
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

    fn display_preferences_menu_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        button_materials: Res<ButtonMaterials>,
        mut materials: ResMut<Assets<ColorMaterial>>,
        settings: Res<super::Settings>,
    ) {
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");

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
            color: super::TEXT_COLOR,
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
            .insert(ScreenMenuPrefDisplay)
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
                                "Display Quality",
                                button_text_style.clone(),
                                Default::default(),
                            ),
                            ..Default::default()
                        });
                        for quality in [
                            super::DisplayQuality::Low,
                            super::DisplayQuality::Medium,
                            super::DisplayQuality::High,
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
                parent
                    .spawn_bundle(ButtonBundle {
                        style: button_style,
                        material: button_materials.normal.clone(),
                        ..Default::default()
                    })
                    .insert(MenuButtonAction::BackToPreferences)
                    .with_children(|parent| {
                        parent.spawn_bundle(TextBundle {
                            text: Text::with_section("Back", button_text_style, Default::default()),
                            ..Default::default()
                        });
                    });
            });
    }

    fn sound_preferences_menu_setup(
        mut commands: Commands,
        asset_server: Res<AssetServer>,
        button_materials: Res<ButtonMaterials>,
        mut materials: ResMut<Assets<ColorMaterial>>,
        settings: Res<super::Settings>,
    ) {
        let font = asset_server.load("fonts/FiraSans-Bold.ttf");

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
            color: super::TEXT_COLOR,
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
            .insert(ScreenMenuPrefSound)
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
                    .insert(MenuButtonAction::BackToPreferences)
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
        mut game_state: ResMut<State<super::GameState>>,
    ) {
        for (interaction, menu_button_action) in interaction_query.iter() {
            if *interaction == Interaction::Clicked {
                match menu_button_action {
                    MenuButtonAction::Quit => app_exit_events.send(AppExit),
                    MenuButtonAction::Play => {
                        game_state.set(super::GameState::Game).unwrap();
                        menu_state.set(MenuState::Disabled).unwrap()
                    }
                    MenuButtonAction::Preferences => {
                        menu_state.set(MenuState::Preferences).unwrap()
                    }
                    MenuButtonAction::PrefDisplay => {
                        menu_state.set(MenuState::PrefDisplay).unwrap()
                    }
                    MenuButtonAction::PrefSound => menu_state.set(MenuState::PrefSound).unwrap(),
                    MenuButtonAction::BackToMainMenu => menu_state.set(MenuState::Main).unwrap(),
                    MenuButtonAction::BackToPreferences => {
                        menu_state.set(MenuState::Preferences).unwrap()
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
