//! Demonstrates how to use the z-index component on UI nodes to control their relative depth
//!
//! It uses colored boxes with different z-index values to demonstrate how it can affect the order of
//! depth of nodes compared to their siblings, but also compared to the entire UI.

use bevy::{
    color::palettes::basic::{BLUE, GRAY, LIME, PURPLE, RED, YELLOW},
    prelude::*,
};

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // spawn the container with default z-index.
    // the default z-index value is `ZIndex::Local(0)`.
    // because this is a root UI node, using local or global values will do the same thing.
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Percent(100.),
                height: Percent(100.),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    background_color: GRAY.into(),
                    style: Style {
                        width: Px(180.0),
                        height: Px(100.0),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    // spawn a node with default z-index.
                    parent.spawn(NodeBundle {
                        background_color: RED.into(),
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Px(10.0),
                            bottom: Px(40.0),
                            width: Px(100.0),
                            height: Px(50.0),
                            ..default()
                        },
                        ..default()
                    });

                    // spawn a node with a positive local z-index of 2.
                    // it will show above other nodes in the gray container.
                    parent.spawn(NodeBundle {
                        z_index: ZIndex(2),
                        background_color: BLUE.into(),
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Px(45.0),
                            bottom: Px(30.0),
                            width: Px(100.),
                            height: Px(50.),
                            ..default()
                        },
                        ..default()
                    });

                    // spawn a node with a negative local z-index.
                    // it will show under other nodes in the gray container.
                    parent.spawn(NodeBundle {
                        z_index: ZIndex(-1),
                        background_color: LIME.into(),
                        style: Style {
                            position_type: PositionType::Absolute,
                            left: Px(70.0),
                            bottom: Px(20.0),
                            width: Px(100.),
                            height: Px(75.),
                            ..default()
                        },
                        ..default()
                    });

                    // spawn a node with a positive global z-index of 1.
                    // it will show above all other nodes, because it's the highest global z-index in this example.
                    // by default, boxes all share the global z-index of 0 that the gray container is added to.
                    parent.spawn((
                        NodeBundle {
                            background_color: PURPLE.into(),
                            style: Style {
                                position_type: PositionType::Absolute,
                                left: Px(15.0),
                                bottom: Px(10.0),
                                width: Px(100.),
                                height: Px(60.),
                                ..default()
                            },
                            ..Default::default()
                        },
                        GlobalZIndex(1),
                    ));

                    // spawn a node with a negative global z-index of -1.
                    // this will show under all other nodes including its parent, because it's the lowest global z-index
                    // in this example.
                    parent.spawn((
                        NodeBundle {
                            background_color: YELLOW.into(),
                            style: Style {
                                position_type: PositionType::Absolute,
                                left: Px(-15.0),
                                bottom: Px(-15.0),
                                width: Px(100.),
                                height: Px(125.),
                                ..default()
                            },
                            ..Default::default()
                        },
                        GlobalZIndex(-1),
                    ));
                });
        });
}
