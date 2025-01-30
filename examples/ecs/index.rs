//! Demonstrates how to use indexes.

use bevy::{ecs::index::WorldIndexExtension, prelude::*};

#[derive(Component, PartialEq, Eq, Hash, Clone, Debug)]
#[component(immutable)]
struct TilePosition {
    x: i32,
    y: i32,
}

fn main() {
    let mut app = App::new();

    // TODO: Expose directly on App
    app.world_mut().index_component::<TilePosition>();

    app.add_systems(Startup, spawn_tiles)
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

fn query_for_tiles(world: &mut World) {
    // TODO: Create SystemParam since this shouldn't need anything more than access to &Index<C> and Query<D, (F, With<C>)>
    let mut query =
        world.query_by_index::<_, (Entity, &TilePosition), ()>(&TilePosition { x: 1, y: 1 });

    let count = query.iter(world).count();
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
