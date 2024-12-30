use core::hint::black_box;

use benches::bench;
use bevy_ecs::bundle::Bundle;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::{component::Component, reflect::ReflectComponent, world::World};
use bevy_hierarchy::{BuildChildren, CloneEntityHierarchyExt};
use bevy_math::Mat4;
use bevy_reflect::{GetTypeRegistration, Reflect};
use criterion::{criterion_group, Bencher, Criterion};

criterion_group!(benches, with_reflect, with_clone);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct C1(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct C2(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct C3(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct C4(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct C5(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct C6(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct C7(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct C8(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct C9(Mat4);

#[derive(Component, Reflect, Default, Clone)]
#[reflect(Component)]
struct C10(Mat4);

type ComplexBundle = (C1, C2, C3, C4, C5, C6, C7, C8, C9, C10);

fn hierarchy<C: Bundle + Default + GetTypeRegistration>(
    b: &mut Bencher,
    width: usize,
    height: usize,
    clone_via_reflect: bool,
) {
    let mut world = World::default();
    let registry = AppTypeRegistry::default();
    {
        let mut r = registry.write();
        r.register::<C>();
    }
    world.insert_resource(registry);
    world.register_bundle::<C>();
    if clone_via_reflect {
        let mut components = Vec::new();
        C::get_component_ids(world.components(), &mut |id| components.push(id.unwrap()));
        for component in components {
            world
                .get_component_clone_handlers_mut()
                .set_component_handler(
                    component,
                    bevy_ecs::component::ComponentCloneHandler::reflect_handler(),
                );
        }
    }

    let id = world.spawn(black_box(C::default())).id();

    let mut hierarchy_level = vec![id];

    for _ in 0..height {
        let current_hierarchy_level = hierarchy_level.clone();
        hierarchy_level.clear();
        for parent_id in current_hierarchy_level {
            for _ in 0..width {
                let child_id = world
                    .spawn(black_box(C::default()))
                    .set_parent(parent_id)
                    .id();
                hierarchy_level.push(child_id);
            }
        }
    }
    world.flush();

    b.iter(move || {
        world.commands().entity(id).clone_and_spawn_with(|builder| {
            builder.recursive(true);
        });
        world.flush();
    });
}

fn simple<C: Bundle + Default + GetTypeRegistration>(b: &mut Bencher, clone_via_reflect: bool) {
    let mut world = World::default();
    let registry = AppTypeRegistry::default();
    {
        let mut r = registry.write();
        r.register::<C>();
    }
    world.insert_resource(registry);
    world.register_bundle::<C>();
    if clone_via_reflect {
        let mut components = Vec::new();
        C::get_component_ids(world.components(), &mut |id| components.push(id.unwrap()));
        for component in components {
            world
                .get_component_clone_handlers_mut()
                .set_component_handler(
                    component,
                    bevy_ecs::component::ComponentCloneHandler::reflect_handler(),
                );
        }
    }
    let id = world.spawn(black_box(C::default())).id();

    b.iter(move || {
        world.commands().entity(id).clone_and_spawn();
        world.flush();
    });
}

fn with_reflect(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("with_reflect"));

    group.bench_function("simple", |b| {
        simple::<ComplexBundle>(b, true);
    });

    group.bench_function("hierarchy_wide", |b| {
        hierarchy::<C1>(b, 10, 4, true);
    });

    group.bench_function("hierarchy_tall", |b| {
        hierarchy::<C1>(b, 1, 50, true);
    });

    group.bench_function("hierarchy_many", |b| {
        hierarchy::<ComplexBundle>(b, 5, 5, true);
    });

    group.finish();
}

fn with_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("with_clone"));

    group.bench_function("simple", |b| {
        simple::<ComplexBundle>(b, false);
    });

    group.bench_function("hierarchy_wide", |b| {
        hierarchy::<C1>(b, 10, 4, false);
    });

    group.bench_function("hierarchy_tall", |b| {
        hierarchy::<C1>(b, 1, 50, false);
    });

    group.bench_function("hierarchy_many", |b| {
        hierarchy::<ComplexBundle>(b, 5, 5, false);
    });

    group.finish();
}
