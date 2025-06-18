mod commands;
mod despawn;
mod despawn_recursive;
mod entity_hash;
mod spawn;
mod world_get;

use commands::*;
use criterion::criterion_group;
use despawn::*;
use despawn_recursive::*;
use entity_hash::*;
use spawn::*;
use world_get::*;

criterion_group!(
    benches,
    empty_commands,
    spawn_commands,
    nonempty_spawn_commands,
    insert_commands,
    fake_commands,
    zero_sized_commands,
    medium_sized_commands,
    large_sized_commands,
    world_entity,
    world_get,
    world_query_get,
    world_query_iter,
    world_query_for_each,
    world_spawn,
    world_despawn,
    world_despawn_recursive,
    query_get,
    query_get_many::<2>,
    query_get_many::<5>,
    query_get_many::<10>,
    entity_set_build_and_lookup,
);
