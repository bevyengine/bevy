//! Demonstrates how the to use the size constraints to control the size of a UI node.

use bevy::{color::palettes::css::*, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_event::<ButtonActivatedEvent>()
        .add_systems(Startup, setup)
        .add_systems(Update, (update_buttons, update_radio_buttons_colors))
        .run();
}

const ACTIVE_BORDER_COLOR: Color = Color::Srgba(ANTIQUE_WHITE);
const INACTIVE_BORDER_COLOR: Color = Color::BLACK;

const ACTIVE_INNER_COLOR: Color = Color::WHITE;
const INACTIVE_INNER_COLOR: Color = Color::Srgba(NAVY);

const ACTIVE_TEXT_COLOR: Color = Color::BLACK;
const HOVERED_TEXT_COLOR: Color = Color::WHITE;
const UNHOVERED_TEXT_COLOR: Color = Color::srgb(0.5, 0.5, 0.5);

#[derive(Component)]
struct Bar;

#[derive(Copy, Clone, Debug, Component, PartialEq)]
enum Constraint {
    FlexBasis,
    Width,
    MinWidth,
    MaxWidth,
}

#[derive(Copy, Clone, Component)]
struct ButtonValue(Val);

#[derive(BufferedEvent)]
struct ButtonActivatedEvent(Entity);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2d);

    let text_font = (
        TextFont {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 33.0,
            ..Default::default()
        },
        TextColor(Color::srgb(0.9, 0.9, 0.9)),
    );

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("Size Constraints Example"),
                        text_font.clone(),
                        Node {
                            margin: UiRect::bottom(Val::Px(25.)),
                            ..Default::default()
                        },
                    ));

                    spawn_bar(parent);

                    parent
                        .spawn((
                            Node {
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::Stretch,
                                padding: UiRect::all(Val::Px(10.)),
                                margin: UiRect::top(Val::Px(50.)),
                                ..default()
                            },
                            BackgroundColor(YELLOW.into()),
                        ))
                        .with_children(|parent| {
                            for constraint in [
                                Constraint::MinWidth,
                                Constraint::FlexBasis,
                                Constraint::Width,
                                Constraint::MaxWidth,
                            ] {
                                spawn_button_row(parent, constraint, text_font.clone());
                            }
                        });
                });
        });
}

fn spawn_bar(parent: &mut ChildSpawnerCommands) {
    parent
        .spawn((
            Node {
                flex_basis: Val::Percent(100.0),
                align_self: AlignSelf::Stretch,
                padding: UiRect::all(Val::Px(10.)),
                ..default()
            },
            BackgroundColor(YELLOW.into()),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        align_items: AlignItems::Stretch,
                        width: Val::Percent(100.),
                        height: Val::Px(100.),
                        padding: UiRect::all(Val::Px(4.)),
                        ..default()
                    },
                    BackgroundColor(Color::BLACK),
                ))
                .with_children(|parent| {
                    parent.spawn((Node::default(), BackgroundColor(Color::WHITE), Bar));
                });
        });
}

fn spawn_button_row(
    parent: &mut ChildSpawnerCommands,
    constraint: Constraint,
    text_style: (TextFont, TextColor),
) {
    let label = match constraint {
        Constraint::FlexBasis => "flex_basis",
        Constraint::Width => "size",
        Constraint::MinWidth => "min_size",
        Constraint::MaxWidth => "max_size",
    };

    parent
        .spawn((
            Node {
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(2.)),
                align_items: AlignItems::Stretch,
                ..default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|parent| {
            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    justify_content: JustifyContent::End,
                    padding: UiRect::all(Val::Px(2.)),
                    ..default()
                })
                .with_children(|parent| {
                    // spawn row label
                    parent
                        .spawn((Node {
                            min_width: Val::Px(200.),
                            max_width: Val::Px(200.),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },))
                        .with_child((Text::new(label), text_style.clone()));

                    // spawn row buttons
                    parent.spawn(Node::default()).with_children(|parent| {
                        spawn_button(
                            parent,
                            constraint,
                            ButtonValue(Val::Auto),
                            "Auto".to_string(),
                            text_style.clone(),
                            true,
                        );
                        for percent in [0., 25., 50., 75., 100., 125.] {
                            spawn_button(
                                parent,
                                constraint,
                                ButtonValue(Val::Percent(percent)),
                                format!("{percent}%"),
                                text_style.clone(),
                                false,
                            );
                        }
                    });
                });
        });
}

fn spawn_button(
    parent: &mut ChildSpawnerCommands,
    constraint: Constraint,
    action: ButtonValue,
    label: String,
    text_style: (TextFont, TextColor),
    active: bool,
) {
    parent
        .spawn((
            Button,
            Node {
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                border: UiRect::all(Val::Px(2.)),
                margin: UiRect::horizontal(Val::Px(2.)),
                ..Default::default()
            },
            BorderColor::all(if active {
                ACTIVE_BORDER_COLOR
            } else {
                INACTIVE_BORDER_COLOR
            }),
            constraint,
            action,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: Val::Px(100.),
                        justify_content: JustifyContent::Center,
                        ..default()
                    },
                    BackgroundColor(if active {
                        ACTIVE_INNER_COLOR
                    } else {
                        INACTIVE_INNER_COLOR
                    }),
                ))
                .with_child((
                    Text::new(label),
                    text_style.0,
                    TextColor(if active {
                        ACTIVE_TEXT_COLOR
                    } else {
                        UNHOVERED_TEXT_COLOR
                    }),
                    TextLayout::new_with_justify(Justify::Center),
                ));
        });
}

fn update_buttons(
    mut button_query: Query<
        (Entity, &Interaction, &Constraint, &ButtonValue),
        Changed<Interaction>,
    >,
    mut bar_node: Single<&mut Node, With<Bar>>,
    mut text_query: Query<&mut TextColor>,
    children_query: Query<&Children>,
    mut button_activated_event: EventWriter<ButtonActivatedEvent>,
) {
    for (button_id, interaction, constraint, value) in button_query.iter_mut() {
        match interaction {
            Interaction::Pressed => {
                button_activated_event.write(ButtonActivatedEvent(button_id));
                match constraint {
                    Constraint::FlexBasis => {
                        bar_node.flex_basis = value.0;
                    }
                    Constraint::Width => {
                        bar_node.width = value.0;
                    }
                    Constraint::MinWidth => {
                        bar_node.min_width = value.0;
                    }
                    Constraint::MaxWidth => {
                        bar_node.max_width = value.0;
                    }
                }
            }
            Interaction::Hovered => {
                if let Ok(children) = children_query.get(button_id) {
                    for &child in children {
                        if let Ok(grand_children) = children_query.get(child) {
                            for &grandchild in grand_children {
                                if let Ok(mut text_color) = text_query.get_mut(grandchild)
                                    && text_color.0 != ACTIVE_TEXT_COLOR
                                {
                                    text_color.0 = HOVERED_TEXT_COLOR;
                                }
                            }
                        }
                    }
                }
            }
            Interaction::None => {
                if let Ok(children) = children_query.get(button_id) {
                    for &child in children {
                        if let Ok(grand_children) = children_query.get(child) {
                            for &grandchild in grand_children {
                                if let Ok(mut text_color) = text_query.get_mut(grandchild)
                                    && text_color.0 != ACTIVE_TEXT_COLOR
                                {
                                    text_color.0 = UNHOVERED_TEXT_COLOR;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn update_radio_buttons_colors(
    mut event_reader: EventReader<ButtonActivatedEvent>,
    button_query: Query<(Entity, &Constraint, &Interaction)>,
    mut border_query: Query<&mut BorderColor>,
    mut color_query: Query<&mut BackgroundColor>,
    mut text_query: Query<&mut TextColor>,
    children_query: Query<&Children>,
) {
    for &ButtonActivatedEvent(button_id) in event_reader.read() {
        let (_, target_constraint, _) = button_query.get(button_id).unwrap();
        for (id, constraint, interaction) in button_query.iter() {
            if target_constraint == constraint {
                let (border_color, inner_color, label_color) = if id == button_id {
                    (ACTIVE_BORDER_COLOR, ACTIVE_INNER_COLOR, ACTIVE_TEXT_COLOR)
                } else {
                    (
                        INACTIVE_BORDER_COLOR,
                        INACTIVE_INNER_COLOR,
                        if matches!(interaction, Interaction::Hovered) {
                            HOVERED_TEXT_COLOR
                        } else {
                            UNHOVERED_TEXT_COLOR
                        },
                    )
                };

                *border_query.get_mut(id).unwrap() = BorderColor::all(border_color);
                for &child in children_query.get(id).into_iter().flatten() {
                    color_query.get_mut(child).unwrap().0 = inner_color;
                    for &grandchild in children_query.get(child).into_iter().flatten() {
                        if let Ok(mut text_color) = text_query.get_mut(grandchild) {
                            text_color.0 = label_color;
                        }
                    }
                }
            }
        }
    }
}
