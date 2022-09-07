//! Demonstrates how to use z-index
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

#[derive(Component)]
struct ZIndexText;

fn setup(mut commands: Commands) {
    commands.spawn_bundle(Camera2dBundle::default());

    // spawn the container with default z-index.
    // ommiting the z-index component is equivalent to inserting `ZIndex::Local(0)`.
    // because this is a root UI node, it is also equivalent to inserting `ZIndex::Global(0)`.
    commands
        .spawn_bundle(NodeBundle {
            color: Color::GRAY.into(),
            style: Style {
                size: Size::new(Val::Px(180.0), Val::Px(100.0)),
                margin: UiRect::all(Val::Auto),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // spawn a node with no z-index.
            // this has the same result as using `ZIndex::Local(0)`.
            // this uses the default depth ordering which is based on where this node is in the hierarchy.
            parent.spawn_bundle(ButtonBundle {
                color: Color::RED.into(),
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
            parent
                .spawn_bundle(ButtonBundle {
                    color: Color::BLUE.into(),
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
                })
                .insert(ZIndex::Local(2));

            // spawn a node with a negative local z-index.
            // it will show under all other nodes in the grey container.
            parent
                .spawn_bundle(ButtonBundle {
                    color: Color::GREEN.into(),
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
                })
                .insert(ZIndex::Local(-1));

            // spawn a node with a positive global z-index of 1.
            // it will show above all other nodes, because it's the highest global z-index in this example app.
            // by default, boxes all share the global z-index of 0 that the grey container is implicitly added to.
            parent
                .spawn_bundle(ButtonBundle {
                    color: Color::PINK.into(),
                    style: Style {
                        position_type: PositionType::Absolute,
                        position: UiRect {
                            left: Val::Px(15.0),
                            bottom: Val::Px(-5.0),
                            ..default()
                        },
                        size: Size::new(Val::Px(100.0), Val::Px(60.0)),
                        ..default()
                    },
                    ..default()
                })
                .insert(ZIndex::Global(1));

            // spawn a node with a negative global z-index.
            // this will show under all other nodes, including its parent, because -1 is the lowest global z-index
            // in this example app.
            parent
                .spawn_bundle(ButtonBundle {
                    color: Color::YELLOW.into(),
                    style: Style {
                        position_type: PositionType::Absolute,
                        position: UiRect {
                            left: Val::Px(-10.0),
                            bottom: Val::Px(-10.0),
                            ..default()
                        },
                        size: Size::new(Val::Px(100.0), Val::Px(50.0)),
                        ..default()
                    },
                    ..default()
                })
                .insert(ZIndex::Global(-1));
        });
}
