use criterion::criterion_group;

mod commands;
use commands::*;

mod spawn;
use spawn::*;

mod world_get;
use world_get::*;

mod entity_hash;
use entity_hash::*;

criterion_group!(
    world_benches,
    empty_commands,
    spawn_commands,
    insert_commands,
    fake_commands,
    zero_sized_commands,
    medium_sized_commands,
    large_sized_commands,
    get_or_spawn,
    world_entity,
    world_get,
    world_query_get,
    world_query_iter,
    world_query_for_each,
    world_spawn,
    query_get,
    query_get_many::<2>,
    query_get_many::<5>,
    query_get_many::<10>,
    entity_set_build_and_lookup
);
