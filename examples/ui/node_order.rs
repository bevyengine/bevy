/// An example that uses the `NodeOrder` component to reorder UI elements.
use bevy::prelude::*;
use bevy_internal::window::PrimaryWindow;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (update_ordered_buttons, update_direction_button))
        .run();
}

#[derive(Component)]
struct OrderedButtonContainer;

#[derive(Component)]
struct OrderedButton(Color);

#[derive(Component)]
struct DirectionButton;

#[derive(Component)]
struct DirectionLabel;

fn setup(mut commands: Commands) {
    let initial_grid_direction = GridAutoFlow::Row;
    commands.spawn(Camera2dBundle::default());

    commands
        .spawn(NodeBundle {
            style: Style {
                flex_direction: FlexDirection::Column,
                size: Size::width(Val::Percent(100.)),
                justify_content: JustifyContent::Start,
                align_items: AlignItems::Center,
                gap: Size::all(Val::Px(20.)),
                margin: UiRect::all(Val::Px(10.)),
                border: UiRect::all(Val::Px(20.)),
                ..Default::default()
            },
            background_color: Color::GRAY.into(),
            ..Default::default()
        })
        .with_children(|builder| {
            builder.spawn(TextBundle::from_section(
                "Click on a grid cell to change its order",
                TextStyle {
                    font_size: 40.,
                    ..Default::default()
                },
            ));

            builder
                .spawn(NodeBundle {
                    style: Style {
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|builder| {
                    builder
                        .spawn((
                            OrderedButtonContainer,
                            NodeBundle {
                                style: Style {
                                    display: Display::Grid,
                                    border: UiRect::all(Val::Px(10.)),
                                    grid_template_rows: vec![RepeatedGridTrack::px(4, 100.)],
                                    grid_template_columns: vec![RepeatedGridTrack::px(4, 100.)],
                                    gap: Size::all(Val::Px(10.)),
                                    grid_auto_flow: initial_grid_direction,
                                    ..Default::default()
                                },
                                background_color: Color::DARK_GRAY.into(),
                                ..Default::default()
                            },
                        ))
                        .with_children(|builder| {
                            for (i, color) in [
                                Color::RED,
                                Color::GREEN,
                                Color::YELLOW,
                                Color::CYAN,
                                Color::AQUAMARINE,
                                Color::CRIMSON,
                                Color::FUCHSIA,
                                Color::PINK,
                                Color::ORANGE,
                                Color::ORANGE_RED,
                                Color::LIME_GREEN,
                                Color::YELLOW_GREEN,
                                Color::GOLD,
                                Color::VIOLET,
                                Color::SILVER,
                                Color::TEAL,
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                builder
                                    .spawn(NodeBundle {
                                        order: NodeOrder(i as i32),
                                        background_color: Color::BLACK.into(),
                                        ..Default::default()
                                    })
                                    .with_children(|builder| {
                                        builder
                                            .spawn((
                                                OrderedButton(color),
                                                ButtonBundle {
                                                    style: Style {
                                                        justify_content: JustifyContent::Center,
                                                        align_items: AlignItems::Center,
                                                        margin: UiRect::all(Val::Px(5.)),
                                                        flex_grow: 1.0,
                                                        ..Default::default()
                                                    },
                                                    background_color: color.into(),
                                                    ..Default::default()
                                                },
                                            ))
                                            .with_children(|builder| {
                                                builder.spawn(TextBundle::from_section(
                                                    format!("{i}"),
                                                    TextStyle {
                                                        font_size: 80.,
                                                        color: Color::BLACK,
                                                        ..Default::default()
                                                    },
                                                ));
                                            });
                                    });
                            }
                        });
                });
            builder
                .spawn(NodeBundle {
                    style: Style {
                        border: UiRect::all(Val::Px(10.)),
                        ..Default::default()
                    },
                    background_color: Color::DARK_GRAY.into(),
                    ..Default::default()
                })
                .with_children(|builder| {
                    builder
                        .spawn((
                            DirectionButton,
                            ButtonBundle {
                                style: Style {
                                    padding: UiRect::all(Val::Px(10.)),
                                    ..Default::default()
                                },
                                background_color: Color::WHITE.into(),
                                ..Default::default()
                            },
                        ))
                        .with_children(|builder| {
                            builder.spawn((
                                DirectionLabel,
                                TextBundle::from_section(
                                    format!("GridAutoFlow::{initial_grid_direction:?}"),
                                    TextStyle {
                                        font_size: 30.,
                                        color: Color::BLACK,
                                        ..Default::default()
                                    },
                                ),
                            ));
                        });
                });
        });
}

fn update_ordered_buttons(
    primary_window_query: Query<&Window, With<PrimaryWindow>>,
    mut button_query: Query<(
        &Node,
        &GlobalTransform,
        &OrderedButton,
        &mut BackgroundColor,
        &Children,
        &Parent,
    )>,
    mut order_query: Query<&mut NodeOrder>,
    mut label_query: Query<&mut Text>,
    mouse_buttons: Res<Input<MouseButton>>,
) {
    let mut n = 0;
    if mouse_buttons.just_pressed(MouseButton::Left) {
        n += 1;
    }
    if mouse_buttons.just_pressed(MouseButton::Right) {
        n -= 1;
    }
    let cursor_position = primary_window_query.single().cursor_position();
    for (node, global_transform, ordered_button, mut background_color, children, parent) in
        &mut button_query
    {
        if cursor_position
            .map(|cursor_position| {
                Rect::from_center_size(global_transform.translation().truncate(), node.size())
                    .contains(cursor_position)
            })
            .unwrap_or(false)
        {
            if n != 0 {
                let mut node_order = order_query.get_mut(parent.get()).unwrap();
                node_order.0 += n;
                let mut text = label_query.get_mut(children[0]).unwrap();
                text.sections[0].value = format!("{}", node_order.0);
            } else {
                // hovered
                background_color.0 = ordered_button.0.with_a(0.25);
                let mut text = label_query.get_mut(children[0]).unwrap();
                text.bypass_change_detection().sections[0].style.color = ordered_button.0;
            }
        } else {
            // none
            background_color.0 = ordered_button.0;
            let mut text = label_query.get_mut(children[0]).unwrap();
            text.bypass_change_detection().sections[0].style.color = Color::BLACK;
        }
    }
}

fn update_direction_button(
    mut direction_button_query: Query<
        (&Interaction, &mut BackgroundColor),
        (Changed<Interaction>, With<DirectionButton>),
    >,
    mut direction_label_query: Query<&mut Text, With<DirectionLabel>>,
    mut button_container_query: Query<&mut Style, With<OrderedButtonContainer>>,
) {
    for (interaction, mut color) in &mut direction_button_query {
        match interaction {
            Interaction::Clicked => {
                color.0 = Color::RED;
                let mut style = button_container_query.single_mut();
                style.grid_auto_flow = match style.grid_auto_flow {
                    GridAutoFlow::Row => GridAutoFlow::Column,
                    _ => GridAutoFlow::Row,
                };
                let mut text = direction_label_query.single_mut();
                text.sections[0].value = format!("GridAutoFlow::{:?}", style.grid_auto_flow);
            }
            Interaction::Hovered => {
                color.0 = Color::YELLOW;
            }
            Interaction::None => {
                color.0 = Color::WHITE;
            }
        }
    }
}
