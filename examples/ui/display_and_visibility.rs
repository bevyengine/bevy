//! Demonstrates how Display and Visibility work in the UI.

use bevy::prelude::*;

const PALETTE: [&str; 4] = ["4059AD", "6B9AC4", "A5C8E1", "EFF2F1"];
const SELECTION_COLOR: &str = "F4B942";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(update)
        .add_system(update_text)
        .run();
}

#[derive(Component)]
struct ButtonTarget {
    id: Entity,
    color: Color,
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let palette = PALETTE.map(|hex| Color::hex(hex).unwrap());

    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        font_size: 24.0,
        color: Color::WHITE,
    };

    commands.spawn(Camera2dBundle::default());
    commands.spawn(NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            flex_basis: Val::Percent(100.),
            align_items: AlignItems::Center,
            ..Default::default()
        },
        background_color: BackgroundColor(Color::BLACK),
        ..Default::default()
    }).with_children(|parent| {
        parent.spawn(TextBundle {
            text: Text::from_section(
                "Use the panel on the right to change the Display and Visibility properties for the respective nodes of the panel on the left.\n\nLeft Click to change Display\nRight Click to change Visibility",
                text_style.clone(),
            ).with_alignment(TextAlignment::Center),
            style: Style {
                margin: UiRect::vertical(Val::Px(10.)),
                ..Default::default()
            },
            ..Default::default()
        });

        parent
            .spawn(NodeBundle {
                style: Style {
                    size: Size::all(Val::Percent(100.)),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::SpaceEvenly,
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|parent| {
                let target_ids = spawn_left_panel(parent, &palette);
                spawn_right_panel(parent, text_style, &palette, target_ids)

        });
    });
}

fn spawn_left_panel(builder: &mut ChildBuilder, palette: &[Color; 4]) -> Vec<Entity> {
    let mut target_ids = vec![];
    builder
        .spawn(NodeBundle {
            style: Style {
                padding: UiRect::all(Val::Px(10.)),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::WHITE),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::all(Val::Px(500.)),
                        ..Default::default()
                    },
                    background_color: BackgroundColor(Color::BLACK),
                    ..Default::default()
                })
                .with_children(|parent| {
                    let id = parent
                        .spawn((NodeBundle {
                            style: Style {
                                size: Size::all(Val::Px(500.)),
                                align_items: AlignItems::FlexEnd,
                                justify_content: JustifyContent::FlexEnd,
                                padding: UiRect {
                                    left: Val::Px(5.),
                                    top: Val::Px(5.),
                                    ..Default::default()
                                },
                                ..Default::default()
                            },
                            background_color: BackgroundColor(palette[0]),
                            ..Default::default()
                        },))
                        .with_children(|parent| {
                            let id = parent
                                .spawn((NodeBundle {
                                    style: Style {
                                        size: Size::all(Val::Px(400.)),
                                        align_items: AlignItems::FlexEnd,
                                        justify_content: JustifyContent::FlexEnd,
                                        padding: UiRect {
                                            left: Val::Px(5.),
                                            top: Val::Px(5.),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    background_color: BackgroundColor(palette[1]),
                                    ..Default::default()
                                },))
                                .with_children(|parent| {
                                    let id = parent
                                        .spawn((NodeBundle {
                                            style: Style {
                                                size: Size::all(Val::Px(300.)),
                                                align_items: AlignItems::FlexEnd,
                                                justify_content: JustifyContent::FlexEnd,
                                                padding: UiRect {
                                                    left: Val::Px(5.),
                                                    top: Val::Px(5.),
                                                    ..Default::default()
                                                },
                                                ..Default::default()
                                            },
                                            background_color: BackgroundColor(palette[2]),
                                            ..Default::default()
                                        },))
                                        .with_children(|parent| {
                                            let id = parent
                                                .spawn((NodeBundle {
                                                    style: Style {
                                                        size: Size::all(Val::Px(200.)),
                                                        align_items: AlignItems::FlexEnd,
                                                        justify_content: JustifyContent::FlexEnd,
                                                        ..Default::default()
                                                    },
                                                    background_color: BackgroundColor(palette[3]),
                                                    ..Default::default()
                                                },))
                                                .id();
                                            target_ids.push(id);
                                        })
                                        .id();
                                    target_ids.push(id);
                                })
                                .id();
                            target_ids.push(id);
                        })
                        .id();
                    target_ids.push(id);
                });
        });
    target_ids
}

fn spawn_right_panel(
    parent: &mut ChildBuilder,
    text_style: TextStyle,
    palette: &[Color; 4],
    mut target_ids: Vec<Entity>,
) {
    parent
        .spawn(NodeBundle {
            style: Style {
                padding: UiRect::all(Val::Px(10.)),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::WHITE),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                .spawn((
                    ButtonBundle {
                        style: Style {
                            size: Size::all(Val::Px(500.)),
                            flex_direction: FlexDirection::Column,
                            align_items: AlignItems::FlexEnd,
                            justify_content: JustifyContent::SpaceBetween,
                            padding: UiRect {
                                left: Val::Px(5.),
                                top: Val::Px(5.),
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        background_color: BackgroundColor(palette[0]),
                        ..Default::default()
                    },
                    ButtonTarget {
                        id: target_ids.pop().unwrap(),
                        color: palette[0],
                    },
                ))
                .with_children(|parent| {
                    parent.spawn((
                        TextBundle {
                            text: Text::from_section("", text_style.clone()),
                            style: Style {
                                align_self: AlignSelf::FlexStart,
                                ..Default::default()
                            },
                            ..Default::default()
                        },
                        BackgroundColor(Color::BLACK.with_a(0.5)),
                    ));

                    parent
                        .spawn((
                            ButtonBundle {
                                style: Style {
                                    size: Size::all(Val::Px(400.)),
                                    flex_direction: FlexDirection::Column,
                                    align_items: AlignItems::FlexEnd,
                                    justify_content: JustifyContent::SpaceBetween,
                                    padding: UiRect {
                                        left: Val::Px(5.),
                                        top: Val::Px(5.),
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                background_color: BackgroundColor(palette[1]),
                                ..Default::default()
                            },
                            ButtonTarget {
                                id: target_ids.pop().unwrap(),
                                color: palette[1],
                            },
                        ))
                        .with_children(|parent| {
                            parent.spawn((
                                TextBundle {
                                    text: Text::from_section("", text_style.clone()),
                                    style: Style {
                                        align_self: AlignSelf::FlexStart,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                },
                                BackgroundColor(Color::BLACK.with_a(0.5)),
                            ));

                            parent
                                .spawn((
                                    ButtonBundle {
                                        style: Style {
                                            size: Size::all(Val::Px(300.)),
                                            flex_direction: FlexDirection::Column,
                                            align_items: AlignItems::FlexEnd,
                                            justify_content: JustifyContent::SpaceBetween,
                                            padding: UiRect {
                                                left: Val::Px(5.),
                                                top: Val::Px(5.),
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        },
                                        background_color: BackgroundColor(palette[2]),
                                        ..Default::default()
                                    },
                                    ButtonTarget {
                                        id: target_ids.pop().unwrap(),
                                        color: palette[2],
                                    },
                                ))
                                .with_children(|parent| {
                                    parent.spawn((
                                        TextBundle {
                                            text: Text::from_section("", text_style.clone()),
                                            style: Style {
                                                align_self: AlignSelf::FlexStart,
                                                ..Default::default()
                                            },
                                            ..Default::default()
                                        },
                                        BackgroundColor(Color::BLACK.with_a(0.5)),
                                    ));

                                    parent
                                        .spawn((
                                            ButtonBundle {
                                                style: Style {
                                                    size: Size::all(Val::Px(200.)),
                                                    align_items: AlignItems::FlexStart,
                                                    justify_content: JustifyContent::FlexStart,
                                                    padding: UiRect {
                                                        left: Val::Px(5.),
                                                        top: Val::Px(5.),
                                                        ..Default::default()
                                                    },
                                                    ..Default::default()
                                                },
                                                background_color: BackgroundColor(palette[3]),
                                                ..Default::default()
                                            },
                                            ButtonTarget {
                                                id: target_ids.pop().unwrap(),
                                                color: palette[3],
                                            },
                                        ))
                                        .with_children(|parent| {
                                            parent.spawn((
                                                TextBundle {
                                                    text: Text::from_section(
                                                        "",
                                                        text_style.clone(),
                                                    ),
                                                    ..Default::default()
                                                },
                                                BackgroundColor(Color::BLACK.with_a(0.5)),
                                            ));
                                        });
                                });
                        });
                });
        });
}

fn update(
    mouse: Res<Input<MouseButton>>,
    mut left_panel_query: Query<
        (&mut BackgroundColor, &mut Style, &mut Visibility),
        Without<Interaction>,
    >,
    mut right_panel_query: Query<(&mut BackgroundColor, &ButtonTarget, &Interaction)>,
) {
    for (mut background_color, button_target, interaction) in right_panel_query.iter_mut() {
        match interaction {
            Interaction::Hovered | Interaction::Clicked => {
                let (mut left_background_color, mut style, mut visibility) =
                    left_panel_query.get_mut(button_target.id).unwrap();

                if mouse.just_pressed(MouseButton::Left) {
                    style.display = match style.display {
                        Display::Flex => Display::None,
                        Display::None => Display::Flex,
                    };
                }

                if mouse.just_pressed(MouseButton::Right) {
                    *visibility = match *visibility {
                        Visibility::Inherited => Visibility::Visible,
                        Visibility::Visible => Visibility::Hidden,
                        Visibility::Hidden => Visibility::Inherited,
                    };
                }
                let selection_color = Color::hex(SELECTION_COLOR).unwrap();
                background_color.0 = selection_color;
                left_background_color.0 = selection_color;
            }
            Interaction::None => {
                let (mut left_background_color, ..) =
                    left_panel_query.get_mut(button_target.id).unwrap();
                background_color.0 = button_target.color;
                left_background_color.0 = button_target.color;
            }
        }
    }
}

fn update_text(
    left_panel_query: Query<(&Style, &Visibility), Or<(Changed<Style>, Changed<Visibility>)>>,
    mut text_query: Query<(&mut Text, &Parent)>,
    mut right_panel_query: Query<&ButtonTarget>,
) {
    text_query.for_each_mut(|(mut text, parent)| {
        if let Ok(target) = right_panel_query.get_mut(parent.get()) {
            if let Ok((style, visibility)) = left_panel_query.get(target.id) {
                text.sections[0].value =
                    format!("Display::{:?}\nVisbility::{visibility:?}", style.display);
            }
        }
    });
}
