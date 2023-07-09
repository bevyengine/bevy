//! Demonstrates how Display and Visibility work in the UI.

use bevy::prelude::*;
use bevy::winit::WinitSettings;

const PALETTE: [&str; 4] = ["27496D", "466B7A", "669DB3", "ADCBE3"];
const HIDDEN_COLOR: Color = Color::rgb(1.0, 0.7, 0.7);

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
    type TargetComponent: Component;
    const NAME: &'static str;
    fn update_target(&self, target: &mut Self::TargetComponent) -> String;
}

impl TargetUpdate for Target<Display> {
    type TargetComponent = Style;
    const NAME: &'static str = "Display";
    fn update_target(&self, style: &mut Self::TargetComponent) -> String {
        style.display = match style.display {
            Display::Flex => Display::None,
            Display::None => Display::Flex,
            Display::Grid => unreachable!(),
        };
        format!("{}::{:?} ", Self::NAME, style.display)
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
                margin: UiRect::bottom(Val::Px(10.)),
                ..Default::default()
            },
            ..Default::default()
        });

        parent
            .spawn(NodeBundle {
                style: Style {
                    width: Val::Percent(100.),
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|parent| {
                let mut target_ids = vec![];
                parent.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(50.),
                        height: Val::Px(520.),
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                }).with_children(|parent| {
                    target_ids = spawn_left_panel(parent, &palette);
                });

                parent.spawn(NodeBundle {
                    style: Style {
                        width: Val::Percent(50.),
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                }).with_children(|parent| {
                    spawn_right_panel(parent, text_style, &palette, target_ids);
                });
            });

            parent.spawn(NodeBundle {
                style: Style {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Start,
                    justify_content: JustifyContent::Start,
                    column_gap: Val::Px(10.),
                    ..Default::default()
                },
                ..default() })
            .with_children(|builder| {
                let text_style = TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 20.0,
                    color: Color::WHITE,
                };

                builder.spawn(TextBundle {
                    text: Text::from_section(
                        "Display::None\nVisibility::Hidden\nVisbility::Inherited",
                        TextStyle { color: HIDDEN_COLOR, ..text_style.clone() }
                        ).with_alignment(TextAlignment::Center),
                    ..Default::default()
                    });
                    builder.spawn(TextBundle {
                        text: Text::from_section(
                            "-\n-\n-",
                            TextStyle { color: Color::DARK_GRAY, ..text_style.clone() }
                            ).with_alignment(TextAlignment::Center),
                        ..Default::default()
                        });
                    builder.spawn(TextBundle::from_section(
                        "The UI Node and its descendants will not be visible and will not be alloted any space in the UI layout.\nThe UI Node will not be visible but will still occupy space in the UI layout.\nThe UI node will inherit the visibility property of its parent. If it has no parent it will be visible.",
                        text_style
                    ));
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
                                    width: Val::Px(100.),
                                    height: Val::Px(500.),
                                    ..Default::default()
                                },
                                ..Default::default()
                            });

                            let id = parent
                                .spawn((NodeBundle {
                                    style: Style {
                                        height: Val::Px(400.),
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
                                            width: Val::Px(100.),
                                            height: Val::Px(400.),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    });

                                    let id = parent
                                        .spawn((NodeBundle {
                                            style: Style {
                                                height: Val::Px(300.),
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
                                                    width: Val::Px(100.),
                                                    height: Val::Px(300.),
                                                    ..Default::default()
                                                },
                                                ..Default::default()
                                            });

                                            let id = parent
                                                .spawn((NodeBundle {
                                                    style: Style {
                                                        width: Val::Px(200.),
                                                        height: Val::Px(200.),
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
    let spawn_buttons = |parent: &mut ChildBuilder, target_id| {
        spawn_button::<Display>(parent, text_style.clone(), target_id);
        spawn_button::<Visibility>(parent, text_style.clone(), target_id);
    };
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
                        width: Val::Px(500.),
                        height: Val::Px(500.),
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
                    spawn_buttons(parent, target_ids.pop().unwrap());

                    parent
                        .spawn((NodeBundle {
                            style: Style {
                                width: Val::Px(400.),
                                height: Val::Px(400.),
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
                            spawn_buttons(parent, target_ids.pop().unwrap());

                            parent
                                .spawn((NodeBundle {
                                    style: Style {
                                        width: Val::Px(300.),
                                        height: Val::Px(300.),
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
                                    spawn_buttons(parent, target_ids.pop().unwrap());

                                    parent
                                        .spawn((NodeBundle {
                                            style: Style {
                                                width: Val::Px(200.),
                                                height: Val::Px(200.),
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
                                            spawn_buttons(parent, target_ids.pop().unwrap());

                                            parent.spawn(NodeBundle {
                                                style: Style {
                                                    width: Val::Px(100.),
                                                    height: Val::Px(100.),
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

fn spawn_button<T>(parent: &mut ChildBuilder, text_style: TextStyle, target: Entity)
where
    T: Default + std::fmt::Debug + Send + Sync + 'static,
    Target<T>: TargetUpdate,
{
    parent
        .spawn((
            ButtonBundle {
                style: Style {
                    //height: Val::Px(24.),
                    align_self: AlignSelf::FlexStart,
                    padding: UiRect::axes(Val::Px(5.), Val::Px(1.)),
                    ..Default::default()
                },
                background_color: BackgroundColor(Color::BLACK.with_a(0.5)),
                ..Default::default()
            },
            Target::<T>::new(target),
        ))
        .with_children(|builder| {
            builder.spawn(
                TextBundle::from_section(
                    format!("{}::{:?}", Target::<T>::NAME, T::default()),
                    text_style,
                )
                .with_text_alignment(TextAlignment::Center),
            );
        });
}

fn buttons_handler<T>(
    mut left_panel_query: Query<&mut <Target<T> as TargetUpdate>::TargetComponent>,
    mut visibility_button_query: Query<(&Target<T>, &Interaction, &Children), Changed<Interaction>>,
    mut text_query: Query<&mut Text>,
) where
    T: Send + Sync,
    Target<T>: TargetUpdate + Component,
{
    for (target, interaction, children) in visibility_button_query.iter_mut() {
        if matches!(interaction, Interaction::Pressed) {
            let mut target_value = left_panel_query.get_mut(target.id).unwrap();
            for &child in children {
                if let Ok(mut text) = text_query.get_mut(child) {
                    text.sections[0].value = target.update_target(target_value.as_mut());
                    text.sections[0].style.color = if text.sections[0].value.contains("None")
                        || text.sections[0].value.contains("Hidden")
                    {
                        Color::rgb(1.0, 0.7, 0.7)
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
    mut text_query: Query<&mut Text>,
) {
    for (interaction, mut background_color, children) in button_query.iter_mut() {
        match interaction {
            Interaction::Hovered => {
                *background_color = BackgroundColor(Color::BLACK.with_a(0.6));
                for &child in children {
                    if let Ok(mut text) = text_query.get_mut(child) {
                        // Bypass change detection to avoid recomputation of the text when only changing the color
                        text.bypass_change_detection().sections[0].style.color = Color::YELLOW;
                    }
                }
            }
            _ => {
                *background_color = BackgroundColor(Color::BLACK.with_a(0.5));
                for &child in children {
                    if let Ok(mut text) = text_query.get_mut(child) {
                        text.bypass_change_detection().sections[0].style.color =
                            if text.sections[0].value.contains("None")
                                || text.sections[0].value.contains("Hidden")
                            {
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
