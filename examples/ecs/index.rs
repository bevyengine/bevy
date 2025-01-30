//! Demonstrates how to query for a component _value_ using indexes.

use bevy::prelude::*;

// To query by component value, first we need to ensure our component is suitable
// for indexing.
//
// The hard requirements are:
// * Immutability
// * Eq + Hash + Clone

#[derive(Component, PartialEq, Eq, Hash, Clone, Debug)]
#[component(immutable)]
struct TilePosition {
    // It's strongly recommended to keep the range of possible values for an index component small.
    // Each unique value in use at the same time will require a ComponentId, so large spaces like `f32` are highly discouraged.
    x: i8,
    y: i8,
}

fn main() {
    App::new()
        // Simply call `add_index` to start indexing a component.
        // Make sure to call this _before_ any components are spawned, otherwise
        // the index will miss those entities!
        .add_index::<TilePosition>()
        .add_systems(Startup, spawn_tiles)
        // Simply spawn some indexed components and move them around.
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

// To query using an index, use `QueryByIndex`.
fn query_for_tiles(mut index_query: QueryByIndex<TilePosition, (Entity, &TilePosition)>) {
    // To specify the value you wish to lookup, use `at(...)` within the system.
    // This returns a `Query`, supporting all the same methods and APIs you're used to!
    let mut tiles_at_1_1: Query<(Entity, &TilePosition), _> =
        index_query.at(&TilePosition { x: 1, y: 1 });

    // The Query returned by `at(...)` will _only_ return entities with the given value.
    let count = tiles_at_1_1.iter().count();
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
