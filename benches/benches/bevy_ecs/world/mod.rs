use criterion::criterion_group;

mod commands;
mod world_get;

use commands::*;
use world_get::*;

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
    query_get_component_simple,
    query_get_component,
    query_get,
);
