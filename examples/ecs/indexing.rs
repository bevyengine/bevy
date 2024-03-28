//! This example demonstrates how to access `Component` data through an `Index`.
#![allow(clippy::type_complexity)]

use bevy::{
    hierarchy::{Index, IndexPlugin, Indexer},
    prelude::*,
};

/// Flag for an inventory item.
#[derive(Component)]
struct Item;

/// Represents something with an owner. Similar to parent-child relationships, but distinct.
#[derive(Component)]
struct Owner(Entity);

/// Flag for a player.
#[derive(Component)]
struct Player;

/// Flag for an NPC.
#[derive(Component)]
struct Npc;

/// Index [`Owner`] by the contained [`Entity`]
struct OwnerIndexer;

impl Indexer for OwnerIndexer {
    type Input = Owner;

    type Index = Entity;

    fn index(input: &Self::Input) -> Self::Index {
        input.0
    }
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // This will only index owned items
        .add_plugins(IndexPlugin::<Owner, With<Item>, OwnerIndexer>::default())
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, print_player_items)
        .run();
}

fn setup(mut commands: Commands) {
    // Spawn a single player with 10 items, all in their possession.
    let mut player = commands.spawn(Player);
    let player_id = player.id();

    player.with_children(|builder| {
        for _ in 0..10 {
            builder.spawn((Item, Owner(builder.parent_entity())));
        }
    });

    // Spawn 100 NPCs with 10 items each.
    for _ in 0..100 {
        commands.spawn(Npc).with_children(|builder| {
            for _ in 0..10 {
                builder.spawn((Item, Owner(builder.parent_entity())));
            }
        });
    }

    // This NPC is a thief! They're holding one of the player's items
    commands.spawn(Npc).with_children(|builder| {
        builder.spawn((Item, Owner(player_id)));
    });
}

fn print_player_items(
    player: Query<Entity, With<Player>>,
    mut items_by_owner: Index<Owner, With<Item>, OwnerIndexer>,
    mut printed: Local<bool>,
) {
    if *printed {
        return;
    }

    // This is a single-player "game"
    let player = player.single();

    // With an index, their isn't a need for an "Owned" component, akin to "Children" for "Parent".
    // Instead, we can ask the index itself for what entities are "Owned" by a particular entity.
    let item_count = items_by_owner.get_by_index(&player).count();

    info!("Player owns {item_count} item(s)");

    *printed = true;
}
