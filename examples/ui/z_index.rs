//! Demonstrates how to use the z-index component on UI nodes to control their relative depth
//!
//! It uses colored boxes with different z-index values to demonstrate how it can affect the order of
//! depth of nodes compared to their siblings, but also compared to the entire UI.

use bevy::prelude::*;

fn main() {
    App::new()
        .insert_resource(ClearColor(Color::BLACK))
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());

    // spawn the container with default z-index.
    // the default z-index value is `ZIndex::Local(0)`.
    // because this is a root UI node, using local or global values will do the same thing.
    commands
        .spawn(NodeBundle {
            background_color: Color::GRAY.into(),
            style: Style {
                size: Size::new(Val::Px(180.0), Val::Px(100.0)),
                margin: UiRect::all(Val::Auto),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // spawn a node with default z-index.
            parent.spawn(NodeBundle {
                background_color: Color::RED.into(),
                style: Style {
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        left: Val::Px(10.0),
                        bottom: Val::Px(40.0),
                        ..default()
                    },
                    size: Size::new(Val::Px(100.0), Val::Px(50.0)),
                    ..default()
                },
                ..default()
            });

            // spawn a node with a positive local z-index of 2.
            // it will show above other nodes in the grey container.
            parent.spawn(NodeBundle {
                z_index: ZIndex::Local(2),
                background_color: Color::BLUE.into(),
                style: Style {
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        left: Val::Px(45.0),
                        bottom: Val::Px(30.0),
                        ..default()
                    },
                    size: Size::new(Val::Px(100.0), Val::Px(50.0)),
                    ..default()
                },
                ..default()
            });

            // spawn a node with a negative local z-index.
            // it will show under other nodes in the grey container.
            parent.spawn(NodeBundle {
                z_index: ZIndex::Local(-1),
                background_color: Color::GREEN.into(),
                style: Style {
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        left: Val::Px(70.0),
                        bottom: Val::Px(20.0),
                        ..default()
                    },
                    size: Size::new(Val::Px(100.0), Val::Px(75.0)),
                    ..default()
                },
                ..default()
            });

            // spawn a node with a positive global z-index of 1.
            // it will show above all other nodes, because it's the highest global z-index in this example.
            // by default, boxes all share the global z-index of 0 that the grey container is added to.
            parent.spawn(NodeBundle {
                z_index: ZIndex::Global(1),
                background_color: Color::PURPLE.into(),
                style: Style {
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        left: Val::Px(15.0),
                        bottom: Val::Px(10.0),
                        ..default()
                    },
                    size: Size::new(Val::Px(100.0), Val::Px(60.0)),
                    ..default()
                },
                ..default()
            });

            // spawn a node with a negative global z-index of -1.
            // this will show under all other nodes including its parent, because it's the lowest global z-index
            // in this example.
            parent.spawn(NodeBundle {
                z_index: ZIndex::Global(-1),
                background_color: Color::YELLOW.into(),
                style: Style {
                    position_type: PositionType::Absolute,
                    position: UiRect {
                        left: Val::Px(-15.0),
                        bottom: Val::Px(-15.0),
                        ..default()
                    },
                    size: Size::new(Val::Px(100.0), Val::Px(125.0)),
                    ..default()
                },
                ..default()
            });
        });
}
