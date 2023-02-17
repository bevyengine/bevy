//! Demonstrates how Display and Visibility work in the UI.

use bevy::prelude::*;
use bevy::winit::WinitSettings;

const PALETTE: [&str; 4] = ["4059AD", "6B9AC4", "A5C8E1", "F4B942"];

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_startup_system(setup)
        .add_system(buttons_handler::<Display>)
        .add_system(buttons_handler::<Visibility>)
        .add_system(text_hover)
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
        };
        format!(" {}::{:?} ", Self::NAME, style.display)
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
        format!(" {}::{visibility:?}", Self::NAME)
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
                    size: Size::width(Val::Percent(100.)),
                    ..Default::default()
                },
                ..Default::default()
            })
            .with_children(|parent| {
                let mut target_ids = vec![];
                parent.spawn(NodeBundle {
                    style: Style {
                        size: Size::new(Val::Percent(50.), Val::Px(520.)),
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                }).with_children(|parent| {
                    target_ids = spawn_left_panel(parent, &palette);
                });

                parent.spawn(NodeBundle {
                    style: Style {
                        size: Size::width(Val::Percent(50.)),
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                }).with_children(|parent| {
                    spawn_right_panel(parent, text_style, &palette, target_ids);
                });
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
                    spawn_buttons(parent, target_ids.pop().unwrap());

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
                            spawn_buttons(parent, target_ids.pop().unwrap());

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
                                    spawn_buttons(parent, target_ids.pop().unwrap());

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
                                            spawn_buttons(parent, target_ids.pop().unwrap());

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

fn spawn_button<T>(parent: &mut ChildBuilder, text_style: TextStyle, target: Entity)
where
    T: Default + std::fmt::Debug + Send + Sync + 'static,
    Target<T>: TargetUpdate,
{
    parent.spawn((
        ButtonBundle {
            style: Style {
                size: Size::height(Val::Px(24.)),
                align_self: AlignSelf::FlexStart,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::BLACK.with_a(0.5)),
            ..Default::default()
        },
        Target::<T>::new(target),
        Text::from_section(
            format!("{}::{:?}", Target::<T>::NAME, T::default()),
            text_style,
        )
        .with_alignment(TextAlignment::Center),
        CalculatedSize::default(),
    ));
}

fn buttons_handler<T>(
    mut left_panel_query: Query<&mut <Target<T> as TargetUpdate>::TargetComponent>,
    mut visibility_button_query: Query<(&mut Text, &Target<T>, &Interaction), Changed<Interaction>>,
) where
    T: Send + Sync,
    Target<T>: TargetUpdate + Component,
{
    for (mut text, target, interaction) in visibility_button_query.iter_mut() {
        if matches!(interaction, Interaction::Clicked) {
            let mut target_value = left_panel_query.get_mut(target.id).unwrap();
            text.sections[0].value = target.update_target(target_value.as_mut());
        }
    }
}

fn text_hover(
    mut text_query: Query<(&mut Text, &mut BackgroundColor, &Interaction), Changed<Interaction>>,
) {
    for (mut text, mut background_color, interaction) in text_query.iter_mut() {
        match interaction {
            Interaction::Hovered => {
                text.sections[0].style.color = Color::YELLOW;
                *background_color = BackgroundColor(Color::BLACK.with_a(0.6));
            }
            _ => {
                text.sections[0].style.color = Color::WHITE;
                *background_color = BackgroundColor(Color::BLACK.with_a(0.5));
            }
        }
    }
}
