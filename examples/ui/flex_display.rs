//! Demonstrates how Display and Visibility work in the UI.

use bevy::prelude::*;

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
    let palette = [
        Color::hex("4059AD").unwrap(),
        Color::hex("6B9AC4").unwrap(),
        Color::hex("A5C8E1").unwrap(),
        Color::hex("EFF2F1").unwrap(),
    ];

    let text_style = TextStyle {
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        font_size: 16.0,
        color: Color::WHITE,
    };

    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                size: Size::all(Val::Percent(100.)),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::rgb(0.0, 0.0, 0.0)),
            ..Default::default()
        })
        .with_children(|parent| {
            let mut target_ids = vec![];
            parent
                .spawn(NodeBundle {
                    style: Style {
                        size: Size::all(Val::Px(520.)),
                        padding: UiRect::all(Val::Px(10.)),
                        ..Default::default()
                    },
                    background_color: BackgroundColor(Color::WHITE),
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
                                                    text: Text::from_section(
                                                        "",
                                                        text_style.clone(),
                                                    ),
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
                                                            justify_content:
                                                                JustifyContent::FlexStart,
                                                            padding: UiRect {
                                                                left: Val::Px(5.),
                                                                top: Val::Px(5.),
                                                                ..Default::default()
                                                            },
                                                            ..Default::default()
                                                        },
                                                        background_color: BackgroundColor(
                                                            palette[3],
                                                        ),
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
        });
}

fn update(
    mouse: Res<Input<MouseButton>>,
    keyboard: Res<Input<KeyCode>>,
    mut left_query: Query<
        (&mut BackgroundColor, &mut Style, &mut Visibility),
        Without<Interaction>,
    >,
    mut right_query: Query<(&mut BackgroundColor, &ButtonTarget, &Interaction)>,
) {
    right_query.for_each_mut(|(mut background_color, button_target, interaction)| {
        match interaction {
            Interaction::Hovered => {
                let (mut left_background_color, mut style, mut visibility) =
                    left_query.get_mut(button_target.id).unwrap();
                //if mouse.just_pressed(MouseButton::Left) {
                if keyboard.just_pressed(KeyCode::Space) {
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
                background_color.0 = Color::hex("F4B942").unwrap();
                left_background_color.0 = Color::hex("F4B942").unwrap();
            }
            Interaction::None => {
                let (mut left_background_color, ..) = left_query.get_mut(button_target.id).unwrap();
                background_color.0 = button_target.color;
                left_background_color.0 = button_target.color;
            }
            _ => {}
        }
    });
}

fn update_text(
    left_query: Query<(&Style, &Visibility), Or<(Changed<Style>, Changed<Visibility>)>>,
    mut text_query: Query<(&mut Text, &Parent)>,
    mut right_query: Query<&ButtonTarget>,
) {
    text_query.for_each_mut(|(mut text, parent)| {
        let target_id = right_query.get_mut(parent.get()).unwrap().id;
        if let Ok((style, visibility)) = left_query.get(target_id) {
            text.sections[0].value =
                format!("Display::{:?}\nVisbility::{:?}", style.display, visibility);
        }
    });
}
