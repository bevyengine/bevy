use bencher::*;

use bevy_ecs::{Command, Commands, Component, DynamicBundle, Entity, Resources, World};
use bevy_transform::prelude::*;

fn push_children_1_parent_to_many_children(bench: &mut Bencher) {
    let mut world = World::default();
    let mut commands = Commands::default();
    let mut resources = Resources::default();

    let entities = world
        .spawn_batch((0..1000).map(|i| (i,)))
        .collect::<Vec<_>>();

    bench.iter(|| {
        commands.push_children(entities[999], &entities[0..998]);
        commands.apply(&mut world, &mut resources);
    })
}

fn push_children_many_parents_with_1_child(bench: &mut Bencher) {
    let mut world = World::default();
    let mut commands = Commands::default();
    let mut resources = Resources::default();

    let entities = world
        .spawn_batch((0..2000).map(|i| (i,)))
        .collect::<Vec<_>>();

    bench.iter(|| {
        for parent in 0..1000 {
            let child = parent + 1000;
            commands.push_children(entities[parent], &entities[child..=child]);
        }
        commands.apply(&mut world, &mut resources);
    })
}

fn push_children_many_parents_with_8_children(bench: &mut Bencher) {
    let mut world = World::default();
    let mut commands = Commands::default();
    let mut resources = Resources::default();

    let entities = world
        .spawn_batch((0..9000).map(|i| (i,)))
        .collect::<Vec<_>>();

    bench.iter(|| {
        for parent in 0..1000 {
            let child_a = parent * 8 + 1000;
            let child_z = (parent + 1) * 8 + 1000;
            commands.push_children(entities[parent], &entities[child_a..child_z]);
        }
        commands.apply(&mut world, &mut resources);
    })
}

benchmark_group!(
    benches,
    push_children_1_parent_to_many_children,
    push_children_many_parents_with_1_child,
    push_children_many_parents_with_8_children
);
benchmark_main!(benches);
