//! Demonstrates how Display and Visibility work in the UI.

use bevy::{
    color::palettes::css::{DARK_CYAN, DARK_GRAY, YELLOW},
    ecs::{component::Mutable, hierarchy::ChildSpawnerCommands},
    prelude::*,
    winit::WinitSettings,
};

const PALETTE: [&str; 4] = ["27496D", "466B7A", "669DB3", "ADCBE3"];
const HIDDEN_COLOR: Color = Color::srgb(1.0, 0.7, 0.7);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                buttons_handler::<Display>,
                buttons_handler::<Visibility>,
                text_hover,
            ),
        )
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

trait TargetUpdate {
    type TargetComponent: Component<Mutability = Mutable>;
    const NAME: &'static str;
    fn update_target(&self, target: &mut Self::TargetComponent) -> String;
}

impl TargetUpdate for Target<Display> {
    type TargetComponent = Node;
    const NAME: &'static str = "Display";
    fn update_target(&self, node: &mut Self::TargetComponent) -> String {
        node.display = match node.display {
            Display::Flex => Display::None,
            Display::None => Display::Flex,
            Display::Block | Display::Grid => unreachable!(),
        };
        format!("{}::{:?} ", Self::NAME, node.display)
    }
}

impl TargetUpdate for Target<Visibility> {
    type TargetComponent = Visibility;
    const NAME: &'static str = "Visibility";
    fn update_target(&self, visibility: &mut Self::TargetComponent) -> String {
        *visibility = match *visibility {
            Visibility::Inherited => Visibility::Visible,
            Visibility::Visible => Visibility::Hidden,
            Visibility::Hidden => Visibility::Inherited,
        };
        format!("{}::{visibility:?}", Self::NAME)
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let palette: [Color; 4] = PALETTE.map(|hex| Srgba::hex(hex).unwrap().into());

    let text_font = TextFont {
        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
        ..default()
    };

    commands.spawn(Camera2d);
    commands
        .spawn((
            Node {
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                ..Default::default()
            },
            BackgroundColor(Color::BLACK),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("Use the panel on the right to change the Display and Visibility properties for the respective nodes of the panel on the left"),
                text_font.clone(),
                TextLayout::new_with_justify(Justify::Center),
                Node {
                    margin: UiRect::bottom(px(10)),
                    ..Default::default()
                },
            ));

            parent
                .spawn(Node {
                    width: percent(100),
                    ..default()
                })
                .with_children(|parent| {
                    let mut target_ids = vec![];
                    parent
                        .spawn(Node {
                            width: percent(50),
                            height: px(520),
                            justify_content: JustifyContent::Center,
                            ..default()
                        })
                        .with_children(|parent| {
                            target_ids = spawn_left_panel(parent, &palette);
                        });

                    parent
                        .spawn(Node {
                            width: percent(50),
                            justify_content: JustifyContent::Center,
                            ..default()
                        })
                        .with_children(|parent| {
                            spawn_right_panel(parent, text_font, &palette, target_ids);
                        });
                });

            parent
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Start,
                    justify_content: JustifyContent::Start,
                    column_gap: px(10),
                    ..default()
                })
                .with_children(|builder| {
                    let text_font = TextFont {
                        font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                        ..default()
                    };

                    builder.spawn((
                        Text::new("Display::None\nVisibility::Hidden\nVisibility::Inherited"),
                        text_font.clone(),
                        TextColor(HIDDEN_COLOR),
                        TextLayout::new_with_justify(Justify::Center),
                    ));
                    builder.spawn((
                        Text::new("-\n-\n-"),
                        text_font.clone(),
                        TextColor(DARK_GRAY.into()),
                        TextLayout::new_with_justify(Justify::Center),
                    ));
                    builder.spawn((Text::new("The UI Node and its descendants will not be visible and will not be allotted any space in the UI layout.\nThe UI Node will not be visible but will still occupy space in the UI layout.\nThe UI node will inherit the visibility property of its parent. If it has no parent it will be visible."), text_font));
                });
        });
}

fn spawn_left_panel(builder: &mut ChildSpawnerCommands, palette: &[Color; 4]) -> Vec<Entity> {
    let mut target_ids = vec![];
    builder
        .spawn((
            Node {
                padding: UiRect::all(px(10)),
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ))
        .with_children(|parent| {
            parent
                .spawn((Node::default(), BackgroundColor(Color::BLACK)))
                .with_children(|parent| {
                    let id = parent
                        .spawn((
                            Node {
                                align_items: AlignItems::FlexEnd,
                                justify_content: JustifyContent::FlexEnd,
                                ..default()
                            },
                            BackgroundColor(palette[0]),
                            Outline {
                                width: px(4),
                                color: DARK_CYAN.into(),
                                offset: px(10),
                            },
                        ))
                        .with_children(|parent| {
                            parent.spawn(Node {
                                width: px(100),
                                height: px(500),
                                ..default()
                            });

                            let id = parent
                                .spawn((
                                    Node {
                                        height: px(400),
                                        align_items: AlignItems::FlexEnd,
                                        justify_content: JustifyContent::FlexEnd,
                                        ..default()
                                    },
                                    BackgroundColor(palette[1]),
                                ))
                                .with_children(|parent| {
                                    parent.spawn(Node {
                                        width: px(100),
                                        height: px(400),
                                        ..default()
                                    });

                                    let id = parent
                                        .spawn((
                                            Node {
                                                height: px(300),
                                                align_items: AlignItems::FlexEnd,
                                                justify_content: JustifyContent::FlexEnd,
                                                ..default()
                                            },
                                            BackgroundColor(palette[2]),
                                        ))
                                        .with_children(|parent| {
                                            parent.spawn(Node {
                                                width: px(100),
                                                height: px(300),
                                                ..default()
                                            });

                                            let id = parent
                                                .spawn((
                                                    Node {
                                                        width: px(200),
                                                        height: px(200),
                                                        ..default()
                                                    },
                                                    BackgroundColor(palette[3]),
                                                ))
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
    parent: &mut ChildSpawnerCommands,
    text_font: TextFont,
    palette: &[Color; 4],
    mut target_ids: Vec<Entity>,
) {
    let spawn_buttons = |parent: &mut ChildSpawnerCommands, target_id| {
        spawn_button::<Display>(parent, text_font.clone(), target_id);
        spawn_button::<Visibility>(parent, text_font.clone(), target_id);
    };
    parent
        .spawn((
            Node {
                padding: UiRect::all(px(10)),
                ..default()
            },
            BackgroundColor(Color::WHITE),
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        width: px(500),
                        height: px(500),
                        flex_direction: FlexDirection::Column,
                        align_items: AlignItems::FlexEnd,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect {
                            left: px(5),
                            top: px(5),
                            ..default()
                        },
                        ..default()
                    },
                    BackgroundColor(palette[0]),
                    Outline {
                        width: px(4),
                        color: DARK_CYAN.into(),
                        offset: px(10),
                    },
                ))
                .with_children(|parent| {
                    spawn_buttons(parent, target_ids.pop().unwrap());

                    parent
                        .spawn((
                            Node {
                                width: px(400),
                                height: px(400),
                                flex_direction: FlexDirection::Column,
                                align_items: AlignItems::FlexEnd,
                                justify_content: JustifyContent::SpaceBetween,
                                padding: UiRect {
                                    left: px(5),
                                    top: px(5),
                                    ..default()
                                },
                                ..default()
                            },
                            BackgroundColor(palette[1]),
                        ))
                        .with_children(|parent| {
                            spawn_buttons(parent, target_ids.pop().unwrap());

                            parent
                                .spawn((
                                    Node {
                                        width: px(300),
                                        height: px(300),
                                        flex_direction: FlexDirection::Column,
                                        align_items: AlignItems::FlexEnd,
                                        justify_content: JustifyContent::SpaceBetween,
                                        padding: UiRect {
                                            left: px(5),
                                            top: px(5),
                                            ..default()
                                        },
                                        ..default()
                                    },
                                    BackgroundColor(palette[2]),
                                ))
                                .with_children(|parent| {
                                    spawn_buttons(parent, target_ids.pop().unwrap());

                                    parent
                                        .spawn((
                                            Node {
                                                width: px(200),
                                                height: px(200),
                                                align_items: AlignItems::FlexStart,
                                                justify_content: JustifyContent::SpaceBetween,
                                                flex_direction: FlexDirection::Column,
                                                padding: UiRect {
                                                    left: px(5),
                                                    top: px(5),
                                                    ..default()
                                                },
                                                ..default()
                                            },
                                            BackgroundColor(palette[3]),
                                        ))
                                        .with_children(|parent| {
                                            spawn_buttons(parent, target_ids.pop().unwrap());

                                            parent.spawn(Node {
                                                width: px(100),
                                                height: px(100),
                                                ..default()
                                            });
                                        });
                                });
                        });
                });
        });
}

fn spawn_button<T>(parent: &mut ChildSpawnerCommands, text_font: TextFont, target: Entity)
where
    T: Default + std::fmt::Debug + Send + Sync + 'static,
    Target<T>: TargetUpdate,
{
    parent
        .spawn((
            Button,
            Node {
                align_self: AlignSelf::FlexStart,
                padding: UiRect::axes(px(5), px(1)),
                ..default()
            },
            BackgroundColor(Color::BLACK.with_alpha(0.5)),
            Target::<T>::new(target),
        ))
        .with_children(|builder| {
            builder.spawn((
                Text(format!("{}::{:?}", Target::<T>::NAME, T::default())),
                text_font,
                TextLayout::new_with_justify(Justify::Center),
            ));
        });
}

fn buttons_handler<T>(
    mut left_panel_query: Query<&mut <Target<T> as TargetUpdate>::TargetComponent>,
    mut visibility_button_query: Query<(&Target<T>, &Interaction, &Children), Changed<Interaction>>,
    mut text_query: Query<(&mut Text, &mut TextColor)>,
) where
    T: Send + Sync,
    Target<T>: TargetUpdate + Component,
{
    for (target, interaction, children) in visibility_button_query.iter_mut() {
        if matches!(interaction, Interaction::Pressed) {
            let mut target_value = left_panel_query.get_mut(target.id).unwrap();
            for &child in children {
                if let Ok((mut text, mut text_color)) = text_query.get_mut(child) {
                    **text = target.update_target(target_value.as_mut());
                    text_color.0 = if text.contains("None") || text.contains("Hidden") {
                        Color::srgb(1.0, 0.7, 0.7)
                    } else {
                        Color::WHITE
                    };
                }
            }
        }
    }
}

fn text_hover(
    mut button_query: Query<(&Interaction, &mut BackgroundColor, &Children), Changed<Interaction>>,
    mut text_query: Query<(&Text, &mut TextColor)>,
) {
    for (interaction, mut color, children) in button_query.iter_mut() {
        match interaction {
            Interaction::Hovered => {
                *color = Color::BLACK.with_alpha(0.6).into();
                for &child in children {
                    if let Ok((_, mut text_color)) = text_query.get_mut(child) {
                        // Bypass change detection to avoid recomputation of the text when only changing the color
                        text_color.bypass_change_detection().0 = YELLOW.into();
                    }
                }
            }
            _ => {
                *color = Color::BLACK.with_alpha(0.5).into();
                for &child in children {
                    if let Ok((text, mut text_color)) = text_query.get_mut(child) {
                        text_color.bypass_change_detection().0 =
                            if text.contains("None") || text.contains("Hidden") {
                                HIDDEN_COLOR
                            } else {
                                Color::WHITE
                            };
                    }
                }
            }
        }
    }
}
