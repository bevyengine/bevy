//! This example shows a more advanced ui use case of managing a shared state via a set of button
//! components.
//! interaction state.

use bevy::ui::Display;
use bevy::{prelude::*, winit::WinitSettings};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct ToolbarState {
    selected: ButtonTags,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            selected: ButtonTags::Stop,
        }
    }
}

struct ToolbarEvent {
    tag: ButtonTags,
    message: Option<String>,
    interaction: Interaction,
}

#[derive(Component, Clone, Hash, Debug, Eq, PartialEq)]
enum ButtonTags {
    Play,
    Pause,
    Stop,
}

#[derive(Component)]
struct ToolbarMessage;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_startup_system(setup)
        .add_state(ToolbarState::default())
        .add_event::<ToolbarEvent>()
        .add_system(button_interaction_system)
        .add_system(toolbar_state_system)
        .add_system(toolbar_message_system)
        .add_system(toolbar_style_system)
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
    mut interaction_query: Query<(&Interaction, &ButtonTags), (Changed<Interaction>, With<Button>)>,
    mut ev_toolbar_button: EventWriter<ToolbarEvent>,
) {
    for (interaction, button_tag) in &mut interaction_query {
        match *interaction {
            Interaction::Clicked => {
                let message = match button_tag {
                    ButtonTags::Stop => None,
                    ButtonTags::Play => Some("Playing...".to_string()),
                    ButtonTags::Pause => Some("Pause...".to_string()),
                };

                ev_toolbar_button.send(ToolbarEvent {
                    tag: button_tag.clone(),
                    message,
                    interaction: interaction.clone(),
                });
            }
            Interaction::Hovered => {
                ev_toolbar_button.send(ToolbarEvent {
                    tag: button_tag.clone(),
                    message: None,
                    interaction: interaction.clone(),
                });
            }
            Interaction::None => {
                ev_toolbar_button.send(ToolbarEvent {
                    tag: button_tag.clone(),
                    message: None,
                    interaction: interaction.clone(),
                });
            }
        }
    }
}

fn toolbar_state_system(
    mut ev_toolbar_button: EventReader<ToolbarEvent>,
    mut toolbar_state: ResMut<State<ToolbarState>>,
) {
    for ev in ev_toolbar_button.iter() {
        if ev.interaction == Interaction::Clicked {
            if ev.tag != toolbar_state.current().selected {
                println!("Toolbar state: Setting {:?} as selected", ev.tag);
                toolbar_state
                    .set(ToolbarState {
                        selected: ev.tag.clone(),
                    })
                    .unwrap();
            }
        }
    }
}

fn toolbar_style_system(
    mut ev_toolbar_button: EventReader<ToolbarEvent>,
    mut style_query: Query<(&mut Style, &mut UiColor, &ButtonTags), With<Button>>,
    toolbar_state: Res<State<ToolbarState>>,
) {
    for ev in ev_toolbar_button.iter() {
        for (mut style, mut color, tag) in &mut style_query {
            // Update button colours
            let this_button = ev.tag.clone() == tag.clone();
            let hovered = this_button && ev.interaction == Interaction::Hovered;
            let clicked = this_button && ev.interaction == Interaction::Clicked;
            let selected = tag.clone() == toolbar_state.current().selected;

            // println!(
            //     "Checking {:?} hovered?: {:?} clicked: {:?} selected: {:?}",
            //     tag, hovered, clicked, selected
            // );

            if selected {
                if clicked {
                    *color = PRESSED_BUTTON.into();
                } else {
                    *color = SELECTED_BUTTON.into();
                }
            } else {
                if clicked {
                    *color = PRESSED_BUTTON.into();
                } else if hovered {
                    *color = HOVERED_BUTTON.into();
                } else {
                    *color = NORMAL_BUTTON.into();
                }
            }

            if tag.clone() == ButtonTags::Stop
                && toolbar_state.current().selected == ButtonTags::Stop
            {
                style.display = Display::None;
            } else {
                style.display = Display::Flex;
            }
        }
    }
}

/// System: Show and hide the toolbar status message when there is a message avaliable
fn toolbar_message_system(
    mut ev_toolbar_button: EventReader<ToolbarEvent>,
    mut message_el_query: Query<(&mut Text, &mut Style), With<ToolbarMessage>>,
) {
    let (mut message_text, mut message_style) = message_el_query.single_mut();
    for ev in ev_toolbar_button.iter() {
        if ev.interaction == Interaction::Clicked {
            if let Some(message) = ev.message.clone() {
                message_text.sections[0].value = message.clone();
                message_style.display = Display::Flex;
            } else {
                message_style.display = Display::None;
            }
        }
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
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

                    parent
                        .spawn_bundle(TextBundle::from_section(
                            "".to_string(),
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
