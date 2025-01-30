//! Demonstrates how to use indexes.

use bevy::prelude::*;

#[derive(Component, PartialEq, Eq, Hash, Clone, Debug)]
#[component(immutable)]
struct TilePosition {
    x: i32,
    y: i32,
}

fn main() {
    App::new()
        .add_index::<TilePosition>()
        .add_systems(Startup, spawn_tiles)
        .add_systems(
            Update,
            (
                query_for_tiles,
                move_all_tiles_to,
                query_for_tiles,
                move_all_tiles_from,
                query_for_tiles,
            )
                .chain(),
        )
        .run();
}

fn spawn_tiles(mut commands: Commands) {
    for _ in 0..10 {
        commands.spawn(TilePosition { x: 1, y: 1 });
        commands.spawn(TilePosition { x: -1, y: 1 });
    }
}

fn query_for_tiles(mut index_query: QueryByIndex<TilePosition, (Entity, &TilePosition)>) {
    let count = index_query.at(&TilePosition { x: 1, y: 1 }).iter().count();
    println!("Found {count} at (1,1)!");
}

fn move_all_tiles_to(query: Query<Entity, With<TilePosition>>, mut commands: Commands) {
    println!("Moving tiles to observation point...");
    for entity in query.iter() {
        commands.entity(entity).insert(TilePosition { x: 1, y: 1 });
    }
}

fn move_all_tiles_from(query: Query<Entity, With<TilePosition>>, mut commands: Commands) {
    println!("Moving tiles away from observation point...");
    for entity in query.iter() {
        commands
            .entity(entity)
            .insert(TilePosition { x: -1, y: -1 });
    }
}
