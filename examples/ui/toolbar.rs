//! This example shows a more advanced ui use case of managing a shared state via a set of button
//! components.
//! interaction state.
#[derive(Component, Clone, Hash, Debug, Eq, PartialEq)]
enum ButtonTags {
    Play,
    Pause,
    Stop,
}
#[derive(Component)]
struct ToolbarMessage;

/// Help wanted: Is this best practice? Should I group relevant data together
/// or split the `current_selected` and `message` into their own seperate states?
#[derive(Hash, Clone, Debug, Eq, PartialEq)]
struct ToolbarState {
    current_selected: ButtonTags,
    message: Option<String>,
}

use bevy::{prelude::*, winit::WinitSettings};
use bevy::ui::Display;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_startup_system(setup)
        .add_system(button_interaction_system)
        .add_system(toolbar_message_system)
        .add_state(ToolbarState {
            current_selected: ButtonTags::Stop,
            message: None,
        })
        .run();
}

const BG_COLOR: Color = Color::rgb(0.1, 0.1, 0.1);
const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.4, 0.4, 0.8);
const SELECTED_BUTTON: Color = Color::rgb(0.35, 0.35, 0.75);

/// System: Manages the click interaction and styling of all the button elements
///
/// Help wanted: I had to remove `Changed<Interaction>` from the query so that
/// the buttons that weren't clicked would update.
///
/// Is there a way of listening to changes in `ResMut<State<ToolbarState>>`?
fn button_interaction_system(
    mut interaction_query: Query<
        (&Interaction, &mut UiColor, &ButtonTags, &mut Style),
        With<Button>, // HELP: Had to remove Changed<Interaction> to address 
    >,
    mut toolbar_state: ResMut<State<ToolbarState>>,
) {
    for (interaction, mut color, button_tag, mut style) in &mut interaction_query {
        // Hide Stop button if already stopped
        if *button_tag == ButtonTags::Stop && toolbar_state.current().current_selected == ButtonTags::Stop {
            style.display = Display::None;
        } else {
            style.display = Display::Flex;
        }
        match *interaction {
            Interaction::Clicked => {
                *color = PRESSED_BUTTON.into();
                match button_tag {
                    ButtonTags::Play => {
                        if toolbar_state.current().current_selected != ButtonTags::Play {
                            toolbar_state
                                .push(ToolbarState {
                                    current_selected: ButtonTags::Play,
                                    message: Some("Playing...".to_string()),
                                })
                                .unwrap();
                        }
                    }
                    ButtonTags::Pause => {
                        if toolbar_state.current().current_selected != ButtonTags::Pause {
                            toolbar_state
                                .push(ToolbarState {
                                    current_selected: ButtonTags::Pause,
                                    message: Some("Paused...".to_string()),
                                })
                                .unwrap();
                        }
                    }
                    ButtonTags::Stop => {
                        if toolbar_state.current().current_selected != ButtonTags::Stop {
                            toolbar_state
                                .push(ToolbarState {
                                    current_selected: ButtonTags::Stop,
                                    message: None,
                                })
                                .unwrap();
                        }
                    }
                }
            }
            Interaction::Hovered => {
                if button_tag == &toolbar_state.current().current_selected {
                    *color = PRESSED_BUTTON.into();
                } else {
                    *color = HOVERED_BUTTON.into();
                }
            }
            Interaction::None => {
                if button_tag == &toolbar_state.current().current_selected {
                    *color = SELECTED_BUTTON.into();
                } else {
                    *color = NORMAL_BUTTON.into();
                }
            }
        }
    }
}

/// System: Show and hide the toolbar status message when there is a message avaliable
fn toolbar_message_system(
    toolbar_state: ResMut<State<ToolbarState>>,
    mut toolbar_message: Query<(&mut Text, &mut Style), With<ToolbarMessage>>,
) {
    let (mut toolbar_entity, mut toolbar_style) = toolbar_message.single_mut();
    if let Some(message) = &toolbar_state.current().message {
        toolbar_entity.sections[0].value = message.clone();
        toolbar_style.display = Display::Flex;
    } else {
        toolbar_style.display = Display::None;
    }
}

/// Helper: Create a toolbar button
///
/// * `parent`: Parent element
/// * `asset_server`: For loading assets
/// * `title`: Text to display in button
/// * `tag`: Tag to identify the button by
fn create_toolbar_button(
    parent: &mut ChildBuilder,
    asset_server: &Res<AssetServer>,
    title: &str,
    tag: ButtonTags,
) {
    parent
        .spawn_bundle(ButtonBundle {
            style: Style {
                padding: UiRect {
                    left: Val::Px(10.),
                    right: Val::Px(10.),
                    top: Val::Px(5.),
                    bottom: Val::Px(5.),
                },
                margin: UiRect {
                    right: Val::Px(5.),
                    ..default()
                },
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            color: NORMAL_BUTTON.into(),
            ..default()
        })
        .insert(tag)
        .with_children(|parent| {
            parent.spawn_bundle(TextBundle::from_section(
                title.to_string(),
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 14.0,
                    color: Color::rgb(0.9, 0.9, 0.9),
                },
            ));
        });
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    toolbar_state: Res<State<ToolbarState>>,
) {
    let default_message = "".to_string();
    // ui camera
    commands.spawn_bundle(Camera2dBundle::default());
    commands
        .spawn_bundle(NodeBundle {
            style: Style {
                margin: UiRect::all(Val::Auto),
                flex_direction: FlexDirection::ColumnReverse,
                align_items: AlignItems::FlexStart,
                ..default()
            },
            color: BG_COLOR.into(),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn_bundle(NodeBundle {
                    style: Style {
                        size: Size {
                            width: Val::Px(400.),
                            height: Val::Auto,
                        },
                            padding: UiRect {
                            left: Val::Px(5.),
                            right: Val::Px(5.),
                            top: Val::Px(3.),
                            bottom: Val::Px(3.),
                        },
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    color: BG_COLOR.into(),
                    ..default()
                })
                .with_children(|parent| {
                    create_toolbar_button(parent, &asset_server, "play", ButtonTags::Play);
                    create_toolbar_button(parent, &asset_server, "pause", ButtonTags::Pause);
                    create_toolbar_button(parent, &asset_server, "stop", ButtonTags::Stop);

                    let message = match &toolbar_state.current().message {
                        Some(message) => message,
                        None => &default_message,
                    };

                    parent
                        .spawn_bundle(TextBundle::from_section(
                            message,
                            TextStyle {
                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                font_size: 14.0,
                                color: Color::rgb(0.7, 0.7, 0.7),
                            },
                        ))
                        .insert(ToolbarMessage);
                });
        });
}
