//! Demonstrates how Display and Visibility work in the UI.

use bevy::prelude::*;

const PALETTE: [&str; 4] = ["4059AD", "6B9AC4", "A5C8E1", "F4B942"];
const SELECTION_COLOR: &str = "EFF2F1";

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(display_buttons)
        .add_system(visibility_buttons)
        .run();
}

#[derive(Component)]
struct Target<T> {
    id: Entity,
    phantom: std::marker::PhantomData<T>,
}

impl<T> Target<T> {
    fn new(id: Entity) -> Self {
        Self {
            id,
            phantom: std::marker::PhantomData,
        }
    }
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
            justify_content: JustifyContent::SpaceEvenly,
            ..Default::default()
        },
        background_color: BackgroundColor(Color::BLACK),
        ..Default::default()
    }).with_children(|parent| {
        parent.spawn(TextBundle {
            text: Text::from_section(
                "Use the panel on the right to change the Display and Visibility properties for the respective nodes of the panel on the left",                
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
                    justify_content: JustifyContent::SpaceEvenly,
                    size: Size::width(Val::Percent(100.)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|parent| {
                let target_ids = spawn_left_panel(parent, &palette);
                spawn_right_panel(parent, text_style, &palette, target_ids);
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
                        ..Default::default()
                    },
                    background_color: BackgroundColor(Color::BLACK),
                    ..Default::default()
                })
                .with_children(|parent| {
                    let id = parent
                        .spawn((NodeBundle {
                            style: Style {
                                align_items: AlignItems::FlexEnd,
                                justify_content: JustifyContent::FlexEnd,

                                ..Default::default()
                            },
                            background_color: BackgroundColor(palette[0]),
                            ..Default::default()
                        },))
                        .with_children(|parent| {
                            parent.spawn(NodeBundle {
                                style: Style {
                                    size: Size::new(Val::Px(100.), Val::Px(500.)),
                                    ..Default::default()
                                },
                                ..Default::default()
                            });

                            let id = parent
                                .spawn((NodeBundle {
                                    style: Style {
                                        size: Size::height(Val::Px(400.)),
                                        align_items: AlignItems::FlexEnd,
                                        justify_content: JustifyContent::FlexEnd,

                                        ..Default::default()
                                    },
                                    background_color: BackgroundColor(palette[1]),
                                    ..Default::default()
                                },))
                                .with_children(|parent| {
                                    parent.spawn(NodeBundle {
                                        style: Style {
                                            size: Size::new(Val::Px(100.), Val::Px(400.)),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    });

                                    let id = parent
                                        .spawn((NodeBundle {
                                            style: Style {
                                                size: Size::height(Val::Px(300.)),
                                                align_items: AlignItems::FlexEnd,
                                                justify_content: JustifyContent::FlexEnd,

                                                ..Default::default()
                                            },
                                            background_color: BackgroundColor(palette[2]),
                                            ..Default::default()
                                        },))
                                        .with_children(|parent| {
                                            parent.spawn(NodeBundle {
                                                style: Style {
                                                    size: Size::new(Val::Px(100.), Val::Px(300.)),
                                                    ..Default::default()
                                                },
                                                ..Default::default()
                                            });

                                            let id = parent
                                                .spawn((NodeBundle {
                                                    style: Style {
                                                        size: Size::all(Val::Px(200.)),
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
                .spawn(NodeBundle {
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
                })
                .with_children(|parent| {
                    let target = target_ids.pop().unwrap();
                    spawn_button::<Display>(
                        parent,
                        format!("Display::{:?}", Display::default()),
                        text_style.clone(),
                        Target::new(target),
                    );
                    spawn_button::<Visibility>(
                        parent,
                        format!("Visibility::{:?}", Visibility::default()),
                        text_style.clone(),
                        Target::new(target),
                    );

                    parent
                        .spawn((NodeBundle {
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
                        },))
                        .with_children(|parent| {
                            let target = target_ids.pop().unwrap();
                            spawn_button::<Display>(
                                parent,
                                format!("Display::{:?}", Display::default()),
                                text_style.clone(),
                                Target::new(target),
                            );
                            spawn_button::<Visibility>(
                                parent,
                                format!("Visibility::{:?}", Visibility::default()),
                                text_style.clone(),
                                Target::new(target),
                            );

                            parent
                                .spawn((NodeBundle {
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
                                },))
                                .with_children(|parent| {
                                    let target = target_ids.pop().unwrap();
                                    spawn_button::<Display>(
                                        parent,
                                        format!("Display::{:?}", Display::default()),
                                        text_style.clone(),
                                        Target::new(target),
                                    );
                                    spawn_button::<Visibility>(
                                        parent,
                                        format!("Visibility::{:?}", Visibility::default()),
                                        text_style.clone(),
                                        Target::new(target),
                                    );

                                    parent
                                        .spawn((NodeBundle {
                                            style: Style {
                                                size: Size::all(Val::Px(200.)),
                                                align_items: AlignItems::FlexStart,
                                                justify_content: JustifyContent::SpaceBetween,
                                                flex_direction: FlexDirection::Column,
                                                padding: UiRect {
                                                    left: Val::Px(5.),
                                                    top: Val::Px(5.),
                                                    ..Default::default()
                                                },
                                                ..Default::default()
                                            },
                                            background_color: BackgroundColor(palette[3]),
                                            ..Default::default()
                                        },))
                                        .with_children(|parent| {
                                            let target = target_ids.pop().unwrap();
                                            spawn_button::<Display>(
                                                parent,
                                                format!("Display::{:?}", Display::default()),
                                                text_style.clone(),
                                                Target::new(target),
                                            );
                                            spawn_button::<Visibility>(
                                                parent,
                                                format!("Visibility::{:?}", Visibility::default()),
                                                text_style.clone(),
                                                Target::new(target),
                                            );
                                            parent.spawn(NodeBundle {
                                                style: Style {
                                                    size: Size::all(Val::Px(100.)),
                                                    ..Default::default()
                                                },
                                                ..Default::default()
                                            });
                                        });
                                });
                        });
                });
        });
}

fn spawn_button<T>(
    parent: &mut ChildBuilder,
    label: String,
    text_style: TextStyle,
    target: Target<T>,
) where
    Target<T>: Bundle,
{
    parent.spawn((
        ButtonBundle {
            style: Style {
                size: Size::new(Val::Px(180.), Val::Px(24.)),
                align_self: AlignSelf::FlexStart,
                align_items: AlignItems::Stretch,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..Default::default()
        },
        target,
        Text::from_section(label, text_style.clone()).with_alignment(TextAlignment::Center),
        CalculatedSize::default(),
    ));
}

fn display_buttons(
    mut left_panel_query: Query<&mut Style>,
    mut display_button_query: Query<
        (&mut Text, &mut Target<Display>, &Interaction),
        Changed<Interaction>,
    >,
) {
    for (mut text, target, interaction) in display_button_query.iter_mut() {
        match interaction {
            Interaction::Clicked => {
                let mut style = left_panel_query.get_mut(target.id).unwrap();
                style.display = match style.display {
                    Display::Flex => Display::None,
                    Display::None => Display::Flex,
                };
                text.sections[0].value = format!("Display::{:?}", style.display);
            }
            _ => {}
        }
    }
}

fn visibility_buttons(
    mut left_panel_query: Query<&mut Visibility>,
    mut visibility_button_query: Query<
        (&mut Text, &mut Target<Visibility>, &Interaction),
        Changed<Interaction>,
    >,
) {
    for (mut text, target, interaction) in visibility_button_query.iter_mut() {
        match interaction {
            Interaction::Clicked => {
                let mut visibility = left_panel_query.get_mut(target.id).unwrap();
                *visibility = match *visibility {
                    Visibility::Inherited => Visibility::Visible,
                    Visibility::Visible => Visibility::Hidden,
                    Visibility::Hidden => Visibility::Inherited,
                };
                text.sections[0].value = format!("Visibility::{:?}", *visibility);
            }
            _ => {}
        }
    }
}
