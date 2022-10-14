//! This example shows a more advanced ui use case of managing a shared state via a set of button
//! components.

use bevy::ui::Display;
use bevy::{prelude::*, winit::WinitSettings};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
struct ToolbarState {
    selected: ButtonTag,
}

impl Default for ToolbarState {
    fn default() -> Self {
        Self {
            selected: ButtonTag::Stop,
        }
    }
}

struct ToolbarEvent {
    tag: ButtonTag,
    message: Option<String>,
    interaction: Interaction,
}

#[derive(Component, Clone, Hash, Debug, Eq, PartialEq)]
enum ButtonTag {
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
        .add_system(toolbar_state_system.after(button_interaction_system))
        .add_system(toolbar_message_system.after(button_interaction_system))
        .add_system(toolbar_style_system.after(toolbar_state_system))
        .run();
}

const BG_COLOR: Color = Color::rgb(0.1, 0.1, 0.1);
const NORMAL_BUTTON: Color = Color::rgb(0.15, 0.15, 0.15);
const HOVERED_BUTTON: Color = Color::rgb(0.25, 0.25, 0.25);
const PRESSED_BUTTON: Color = Color::rgb(0.4, 0.4, 0.8);
const SELECTED_BUTTON: Color = Color::rgb(0.35, 0.35, 0.75);

/// Manages the click interaction
fn button_interaction_system(
    mut interaction_query: Query<(&Interaction, &ButtonTag), (Changed<Interaction>, With<Button>)>,
    mut ev_toolbar_button: EventWriter<ToolbarEvent>,
) {
    for (interaction, button_tag) in &mut interaction_query {
        match *interaction {
            Interaction::Clicked => {
                let message = match button_tag {
                    ButtonTag::Stop => None,
                    ButtonTag::Play => Some("Playing...".to_string()),
                    ButtonTag::Pause => Some("Pause...".to_string()),
                };

                ev_toolbar_button.send(ToolbarEvent {
                    tag: button_tag.clone(),
                    message,
                    interaction: interaction.clone(),
                });
            }
            Interaction::Hovered | Interaction::None => {
                ev_toolbar_button.send(ToolbarEvent {
                    tag: button_tag.clone(),
                    message: None,
                    interaction: interaction.clone(),
                });
            }
        }
    }
}

/// Commits the last selected button to ToolbarState
fn toolbar_state_system(
    mut ev_toolbar_button: EventReader<ToolbarEvent>,
    mut toolbar_state: ResMut<State<ToolbarState>>,
) {
    for ev in ev_toolbar_button.iter() {
        if ev.interaction == Interaction::Clicked {
            if ev.tag != toolbar_state.current().selected {
                toolbar_state
                    .set(ToolbarState {
                        selected: ev.tag.clone(),
                    })
                    .unwrap();
            }
        }
    }
}

/// Manages the background colour / display of the toolbar buttons
fn toolbar_style_system(
    mut ev_toolbar_button: EventReader<ToolbarEvent>,
    mut style_query: Query<(&mut Style, &mut UiColor, &ButtonTag), With<Button>>,
    toolbar_state: Res<State<ToolbarState>>,
) {
    for ev in ev_toolbar_button.iter() {
        for (mut style, mut color, tag) in &mut style_query {
            // If this button is the button that the event is referring to
            let is_event_button = &ev.tag == tag;
            // If this button is the currently selected button
            let selected = tag == &toolbar_state.current().selected;

            *color = match (is_event_button, selected, ev.interaction) {
                (_, true, _) => SELECTED_BUTTON.into(),
                (true, _, Interaction::Clicked) => PRESSED_BUTTON.into(),
                (true, _, Interaction::Hovered) => HOVERED_BUTTON.into(),
                (_, _, _) => NORMAL_BUTTON.into(),
            };

            // Hide stop button if not currently playing / paused
            if *tag == ButtonTag::Stop && toolbar_state.current().selected == ButtonTag::Stop {
                style.display = Display::None;
            } else {
                style.display = Display::Flex;
            }
        }
    }
}

/// Show and hide the toolbar status message when there is a message avaliable
fn toolbar_message_system(
    mut ev_toolbar_button: EventReader<ToolbarEvent>,
    mut message_el_query: Query<(&mut Text, &mut Style), With<ToolbarMessage>>,
) {
    let (mut message_text, mut message_style) = message_el_query.single_mut();
    for ev in ev_toolbar_button.iter() {
        if ev.interaction == Interaction::Clicked {
            if let Some(message) = ev.message.clone() {
                message_text.sections[0].value = message;
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
    tag: ButtonTag,
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
                            left: Val::Px(3.),
                            right: Val::Px(3.),
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
                    create_toolbar_button(parent, &asset_server, "play", ButtonTag::Play);
                    create_toolbar_button(parent, &asset_server, "pause", ButtonTag::Pause);
                    create_toolbar_button(parent, &asset_server, "stop", ButtonTag::Stop);

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
