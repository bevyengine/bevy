/// An example that uses the `NodeOrder` component to reorder UI elements.
use bevy::{prelude::*, winit::WinitSettings};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, (update_ordered_buttons, update_direction_button))
        .run();
}

#[derive(Component)]
struct OrderedButtonContainer;

#[derive(Component)]
struct OrderedButton;

#[derive(Component)]
struct DirectionButton;

#[derive(Component)]
struct DirectionLabel;

fn setup(mut commands: Commands) {
    let initial_direction = FlexDirection::Row;
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
                ..Default::default()
            },
            background_color: Color::GRAY.into(),
            ..Default::default()
        })
        .with_children(|builder| {
            builder
                .spawn(NodeBundle {
                    style: Style {
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        size: Size::all(Val::Px(600.)),
                        min_size: Size::all(Val::Px(600.)),
                        max_size: Size::all(Val::Px(600.)),
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
                                    padding: UiRect::all(Val::Px(10.)),
                                    gap: Size::all(Val::Px(10.)),
                                    flex_direction: initial_direction,
                                    ..Default::default()
                                },
                                background_color: Color::BLACK.into(),
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
                            ]
                            .into_iter()
                            .enumerate()
                            {
                                builder
                                    .spawn((
                                        OrderedButton,
                                        ButtonBundle {
                                            style: Style {
                                                size: Size::all(Val::Px(60.)),
                                                justify_content: JustifyContent::Center,
                                                align_items: AlignItems::Center,
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
                                                font_size: 40.,
                                                color: Color::BLACK,
                                                ..Default::default()
                                            },
                                        ));
                                    });
                            }
                        });
                });

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
                            format!("FlexDirection::{initial_direction:?}"),
                            TextStyle {
                                font_size: 30.,
                                color: Color::BLACK,
                                ..Default::default()
                            },
                        ),
                    ));
                });
        });
}

fn update_ordered_buttons(
    mut order: Local<i32>,
    mut button_query: Query<
        (&Interaction, &mut NodeOrder),
        (Changed<Interaction>, With<OrderedButton>),
    >,
) {
    for (interaction, mut node_order) in &mut button_query {
        if *interaction == Interaction::Clicked {
            *order -= 1;
            node_order.0 = *order;
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
                style.flex_direction = match style.flex_direction {
                    FlexDirection::Row => FlexDirection::RowReverse,
                    FlexDirection::RowReverse => FlexDirection::Column,
                    FlexDirection::Column => FlexDirection::ColumnReverse,
                    FlexDirection::ColumnReverse => FlexDirection::Row,
                };
                let mut text = direction_label_query.single_mut();
                text.sections[0].value = format!("FlexDirection::{:?}", style.flex_direction);
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
