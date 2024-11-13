//! Demonstrates the use of picking in Bevy's UI.
//!
//! This example displays a simple inventory system and an animated sprite. Items can be dragged
//! and dropped within the inventory, or onto the sprite with differing effects depending on the
//! item.
//!
//! The implementation is not intended for serious use as a comprehensive inventory system!

use bevy::prelude::*;
use std::fmt::Debug;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, (setup, spawn_text))
        .run();
}

/// Set up some UI to manipulate.
fn setup(asset_server: Res<AssetServer>, mut commands: Commands) {
    // We need a simple camera to display our UI.
    commands.spawn((Name::new("Camera"), Camera2d));

    let inventory_node = Node {
        margin: UiRect::all(Val::Px(5.)),
        height: Val::Px(50.),
        width: Val::Px(50.),
        padding: UiRect::all(Val::Px(5.)),
        ..default()
    };
    // Spawn some arbitrarily-positioned inventory slots as UI nodes.
    commands
        .spawn((
            Name::new("Inventory"),
            // This first node acts like a container. You can think of it as the box in which
            // inventory slots are arranged. It's useful to remember that picking events can
            // "bubble" up through layers of UI, so if required this parent node can act on events
            // that are also received by its child nodes. You could imagine using this for a subtle
            // color change or border highlight.
            Node {
                align_items: AlignItems::Center,
                align_self: AlignSelf::Center,
                justify_content: JustifyContent::Center,
                justify_self: JustifySelf::Center,
                padding: UiRect::all(Val::Px(10.)),
                ..default()
            },
            BackgroundColor(Color::WHITE.with_alpha(0.1)),
        ))
        .observe(|t: Trigger<Pointer<Over>>| {
            dbg!(t);
        })
        .with_children(|parent| {
            parent
                .spawn((
                    Name::new("Inventory Slot A"),
                    inventory_node.clone(),
                    BackgroundColor(Color::WHITE.with_alpha(0.5)),
                ))
                .with_children(|parent| {
                    parent.spawn(ImageNode::new(
                        asset_server.load("textures/rpg/props/generic-rpg-loot01.png"),
                    ));
                })
                .observe(drag_handler::<Pointer<DragEnd>>());

            parent
                .spawn((
                    Name::new("Inventory Slot B"),
                    inventory_node.clone(),
                    BackgroundColor(Color::WHITE.with_alpha(0.5)),
                ))
                .observe(drag_handler::<Pointer<DragStart>>())
                .with_children(|parent| {
                    parent.spawn((ImageNode::new(
                        asset_server.load("textures/rpg/props/generic-rpg-loot02.png"),
                    ),));
                });

            parent
                .spawn((
                    Name::new("Inventory Slot C"),
                    inventory_node.clone(),
                    BackgroundColor(Color::WHITE.with_alpha(0.5)),
                ))
                .observe(drag_handler::<Pointer<DragEnd>>());

            parent
                .spawn((
                    Name::new("Inventory Slot C"),
                    inventory_node.clone(),
                    BackgroundColor(Color::WHITE.with_alpha(0.5)),
                ))
                .observe(drag_handler::<Pointer<DragStart>>());
        });
}

/// Display instructions.
fn spawn_text(mut commands: Commands) {
    commands.spawn((
        Name::new("Instructions"),
        Text::new("Drag and drop birds within the inventory slots."),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.),
            left: Val::Px(12.),
            ..default()
        },
    ));
}

fn drag_handler<E: Debug + Clone + Reflect>() -> impl Fn(Trigger<E>, Query<&mut Sprite>) {
    move |ev, _sprites| {
        dbg!(ev);
    }
}
//
// fn drop_handler<E: Debug + Clone + Reflect>() -> impl Fn(Trigger<E>) {
//     move |ev| {}
// }
