//! This example illustrates how to create a button that changes color and text based on its
//! interaction state.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(check_buttons)
        .run();
}

#[derive(Component)]
struct Bar;

#[derive(Copy, Clone, Component)]
enum Constraint {
    FlexBasis,
    Width,
    MinWidth,
    MaxWidth,
}

#[derive(Copy, Clone, Component)]
pub struct ButtonValue(Val);

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2dBundle::default());

    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        font_size: 40.0,
        color: Color::rgb(0.9, 0.9, 0.9),
    };

    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                flex_basis: Val::Percent(100.0),
                justify_content: JustifyContent::SpaceAround,
                align_items: AlignItems::Center,
                ..Default::default()
            },
            background_color: Color::BLACK.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Size Constraints Example",
                text_style.clone(),
            ));

            spawn_bar(parent);

            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::width(Val::Px(1000.)),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::Stretch,
                        margin: UiRect::all(Val::Px(2.)),
                        ..Default::default()
                    },
                    background_color: Color::CYAN.into(),
                    ..Default::default()
                })
                .with_children(|parent| {
                    for constaint in [
                        Constraint::MinWidth,
                        Constraint::FlexBasis,
                        Constraint::Width,
                        Constraint::MaxWidth,
                    ] {
                        spawn_button_row(parent, constaint, text_style.clone());
                    }
                });
        });
}

fn spawn_bar(parent: &mut ChildBuilder) {
    parent
        .spawn(NodeBundle {
            style: Style {
                padding: UiRect::all(Val::Px(5.)),
                ..Default::default()
            },
            background_color: Color::WHITE.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        align_items: AlignItems::Stretch,
                        size: Size::new(Val::Px(1000.), Val::Px(100.)),
                        padding: UiRect::all(Val::Px(2.)),
                        ..Default::default()
                    },
                    background_color: Color::BLACK.into(),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        NodeBundle {
                            style: Style {
                                ..Default::default()
                            },
                            background_color: Color::RED.into(),
                            ..Default::default()
                        },
                        Bar,
                    ));
                });
        });
}

fn spawn_button_row(parent: &mut ChildBuilder, constraint: Constraint, text_style: TextStyle) {
    let label = match constraint {
        Constraint::FlexBasis => "flex_basis",
        Constraint::Width => "size",
        Constraint::MinWidth => "min_size",
        Constraint::MaxWidth => "max_size",
    };
    parent
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                margin: UiRect::all(Val::Px(2.)),
                padding: UiRect::all(Val::Px(2.)),
                align_items: AlignItems::Stretch,
                ..Default::default()
            },
            background_color: Color::DARK_GRAY.into(),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::all(Val::Px(2.)),
                        ..Default::default()
                    },
                    background_color: Color::RED.into(),
                    ..Default::default()
                })
                .with_children(|parent| {
                    // spawn row label
                    parent.spawn(TextBundle {
                        text: Text::from_section(label.to_string(), text_style.clone()),
                        background_color: Color::BLUE.into(),
                        ..Default::default()
                    });

                    // spawn row buttons
                    parent
                        .spawn(NodeBundle {
                            background_color: Color::DARK_GREEN.into(),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            spawn_button(
                                parent,
                                constraint,
                                ButtonValue(Val::Auto),
                                "Auto".to_string(),
                                text_style.clone(),
                            );
                            for percent in [0., 25., 50., 75., 100., 125.] {
                                spawn_button(
                                    parent,
                                    constraint,
                                    ButtonValue(Val::Percent(percent)),
                                    format!("{percent}%"),
                                    text_style.clone(),
                                );
                            }
                        });
                });
        });
}

fn spawn_button(
    parent: &mut ChildBuilder,
    constraint: Constraint,
    action: ButtonValue,
    label: String,
    text_style: TextStyle,
) {
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    size: Size::width(Val::Px(100.)),
                    ..Default::default()
                },
                background_color: Color::BLACK.into(),
                ..Default::default()
            },
            constraint,
            action,
        ))
        .with_children(|parent| {
            parent.spawn(TextBundle {
                text: Text::from_section(label, text_style),
                ..Default::default()
            });
        });
}

fn check_buttons(
    mut button_query: Query<
        (
            &Interaction,
            &Constraint,
            &ButtonValue,
            &mut BackgroundColor,
        ),
        Changed<Interaction>,
    >,
    mut bar_query: Query<&mut Style, With<Bar>>,
) {
    let mut style = bar_query.single_mut();
    for (interaction, constraint, value, mut background_color) in button_query.iter_mut() {
        match interaction {
            Interaction::Clicked => {
                match constraint {
                    Constraint::FlexBasis => {
                        style.flex_basis = value.0;
                    }
                    Constraint::Width => {
                        style.size.width = value.0;
                    }
                    Constraint::MinWidth => {
                        style.min_size.width = value.0;
                    }
                    Constraint::MaxWidth => {
                        style.max_size.width = value.0;
                    }
                }
                background_color.0 = Color::rgb(0.5, 0.5, 0.5);
            }
            Interaction::Hovered => {
                background_color.0 = Color::rgb(0.7, 0.7, 0.7);
            }
            _ => {
                background_color.0 = Color::BLACK;
            }
        }
    }
}
