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

    let title = commands.spawn(TextBundle {
        text: Text::from_section(
            "Use the panel on the right to change the Display and Visibility properties for the respective nodes of the panel on the left",
            text_style.clone(),
        ).with_alignment(TextAlignment::Center),
        style: Style {
            margin: UiRect::bottom(Val::Px(10.)),
            ..Default::default()
        },
        ..Default::default()
    }).id();

    let left_frame = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(50.),
                height: Val::Px(520.),
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            ..Default::default()
        })
        .id();
    let (left_panel, target_ids) = spawn_left_panel(&mut commands, &palette);
    commands.entity(left_frame).add_child(left_panel);

    let right_frame = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(50.),
                justify_content: JustifyContent::Center,
                ..Default::default()
            },
            ..Default::default()
        })
        .id();
    let right_panel = spawn_right_panel(&mut commands, text_style, &palette, target_ids);
    commands.entity(right_frame).add_child(right_panel);

    let top_frame = commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.),
                ..Default::default()
            },
            ..Default::default()
        })
        .push_children(&[left_panel, right_frame])
        .id();

    let bottom_frame = commands.spawn(NodeBundle {
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
                    "Display::None\nVisibility::Hidden\nVisibility::Inherited",
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
                "The UI Node and its descendants will not be visible and will not be allotted any space in the UI layout.\nThe UI Node will not be visible but will still occupy space in the UI layout.\nThe UI node will inherit the visibility property of its parent. If it has no parent it will be visible.",
                text_style
            ));
        }).id();
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                flex_basis: Val::Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::SpaceEvenly,
                ..Default::default()
            },
            background_color: BackgroundColor(Color::BLACK),
            ..Default::default()
        })
        .push_children(&[title, top_frame, bottom_frame]);
}

// Spawn a simple square node with a background color
fn spawn_square_node(commands: &mut Commands, color: Color, width: f32, height: f32) -> Entity {
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Px(width),
                height: Val::Px(height),
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
            background_color: BackgroundColor(color),
            ..Default::default()
        })
        .id()
}

// Spawn the left panel with squares that will be affected by the buttons on the right panel
fn spawn_left_panel(commands: &mut Commands, palette: &[Color; 4]) -> (Entity, Vec<Entity>) {
    let mut target_ids = vec![];
    let frame = commands
        .spawn(NodeBundle {
            style: Style {
                padding: UiRect::all(Val::Px(10.)),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::WHITE),
            ..Default::default()
        })
        .id();

    // build all
    let mut parent_node = frame;
    for i in 0..4 {
        // create a square of the right size, that will affect the corresponding square on the left panel
        let width = 500. - i as f32 * 100.;
        let height = width;
        let node = spawn_square_node(commands, palette[i], width, height);

        // each square is a child of the previous one
        commands.entity(parent_node).add_child(node);
        parent_node = node;

        // the entities will be stored from outer square to inner square
        target_ids.push(node);
    }
    (frame, target_ids)
}

// Spawn the right panel with buttons that will affect the corresponding squares on the left panel
fn spawn_right_panel(
    commands: &mut Commands,
    text_style: TextStyle,
    palette: &[Color; 4],
    target_ids: Vec<Entity>,
) -> Entity {
    let frame = commands
        .spawn(NodeBundle {
            style: Style {
                padding: UiRect::all(Val::Px(10.)),
                ..Default::default()
            },
            background_color: BackgroundColor(Color::WHITE),
            ..Default::default()
        })
        .id();

    // build all
    let mut parent_node = frame;
    for i in 0..4 {
        // create a square of the right size
        let width = 500. - i as f32 * 100.;
        let height = width;
        let node = spawn_square_node(commands, palette[i], width, height);

        // add buttons that will affect the corresponding square on the left panel
        let target_id = target_ids.get(i).unwrap();
        commands.entity(node).with_children(|parent| {
            spawn_button::<Display>(parent, text_style.clone(), *target_id);
            spawn_button::<Visibility>(parent, text_style.clone(), *target_id);
        });

        // each square is a child of the previous one
        commands.entity(parent_node).add_child(node);
        parent_node = node;

        // NOTE: apparently this invisible node is necessary so that the 'Visibility::Inherited' button
        // is aligned correctly on the last square
        if i == 3 {
            commands.entity(node).with_children(|parent| {
                parent.spawn(NodeBundle {
                    style: Style {
                        width: Val::Px(100.),
                        height: Val::Px(100.),
                        ..Default::default()
                    },
                    ..Default::default()
                });
            });
        }
    }
    frame
}

// Spawn a button that will affect the corresponding square on the left panel
fn spawn_button<T>(parent: &mut ChildBuilder, text_style: TextStyle, target: Entity)
where
    T: Default + std::fmt::Debug + Send + Sync + 'static,
    Target<T>: TargetUpdate,
{
    parent
        .spawn((
            ButtonBundle {
                style: Style {
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

// System that will run to handle button presses
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

// System that will run to handle text hover
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
