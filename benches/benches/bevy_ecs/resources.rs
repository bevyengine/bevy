use std::{alloc::Layout, hint::black_box, ptr::NonNull};

use benches::bench;
use bevy_ecs::{
    change_detection::MaybeLocation,
    component::{ComponentCloneBehavior, ComponentDescriptor, StorageType},
    prelude::*,
    ptr::OwningPtr,
};
use criterion::{criterion_group, Criterion};

criterion_group!(benches, get, get_mut, insert_remove);

fn create_world() -> World {
    let mut world = World::new();
    for _ in 0..500 {
        // SAFETY: Uses zero-sized value, never drops
        unsafe {
            let resource_id =
                world.register_component_with_descriptor(ComponentDescriptor::new_with_layout(
                    "",
                    StorageType::SparseSet,
                    Layout::new::<()>(),
                    None,
                    true,
                    ComponentCloneBehavior::Default,
                    None,
                ));
            world.insert_resource_by_id(
                resource_id,
                OwningPtr::new(NonNull::dangling()),
                MaybeLocation::caller(),
            );
        }
    }
    world
}

#[derive(Resource)]
struct R;

pub fn get(criterion: &mut Criterion) {
    let mut world = create_world();
    world.insert_resource(R);
    criterion.bench_function(bench!("get"), |bencher| {
        bencher.iter(|| world.get_resource::<R>());
    });
}

pub fn get_mut(criterion: &mut Criterion) {
    let mut world = create_world();
    world.insert_resource(R);
    criterion.bench_function(bench!("get_mut"), |bencher| {
        bencher.iter(|| {
            black_box(world.get_resource_mut::<R>());
        });
    });
}

pub fn insert_remove(criterion: &mut Criterion) {
    let mut world = create_world();
    criterion.bench_function(bench!("insert_remove"), |bencher| {
        bencher.iter(|| {
            world.insert_resource(R);
            black_box(&mut world);
            world.remove_resource::<R>()
        });
    });
}
