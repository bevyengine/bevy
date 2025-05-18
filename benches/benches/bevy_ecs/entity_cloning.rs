use core::hint::black_box;

use benches::bench;
use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::ComponentCloneBehavior;
use bevy_ecs::entity::EntityCloner;
use bevy_ecs::hierarchy::ChildOf;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::{component::Component, world::World};
use bevy_math::Mat4;
use bevy_reflect::{GetTypeRegistration, Reflect};
use criterion::{criterion_group, Bencher, Criterion, Throughput};

criterion_group!(
    benches,
    single,
    hierarchy_tall,
    hierarchy_wide,
    hierarchy_many,
);

#[derive(Component, Reflect, Default, Clone)]
struct C1(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C2(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C3(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C4(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C5(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C6(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C7(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C8(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C9(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C10(Mat4);

type ComplexBundle = (C1, C2, C3, C4, C5, C6, C7, C8, C9, C10);

/// Sets the [`ComponentCloneHandler`] for all explicit and required components in a bundle `B` to
/// use the [`Reflect`] trait instead of [`Clone`].
fn reflection_cloner<B: Bundle + GetTypeRegistration>(
    world: &mut World,
    linked_cloning: bool,
) -> EntityCloner {
    // Get mutable access to the type registry, creating it if it does not exist yet.
    let registry = world.get_resource_or_init::<AppTypeRegistry>();

    // Recursively register all components in the bundle to the reflection type registry.
    {
        let mut r = registry.write();
        r.register::<B>();
    }

    // Recursively register all components in the bundle, then save the component IDs to a list.
    // This uses `contributed_components()`, meaning both explicit and required component IDs in
    // this bundle are saved.
    let component_ids: Vec<_> = world.register_bundle::<B>().contributed_components().into();

    let mut builder = EntityCloner::build(world);

    // Overwrite the clone handler for all components in the bundle to use `Reflect`, not `Clone`.
    for component in component_ids {
        builder.override_clone_behavior_with_id(component, ComponentCloneBehavior::reflect());
    }
    builder.linked_cloning(linked_cloning);

    builder.finish()
}

/// A helper function that benchmarks running the [`EntityCommands::clone_and_spawn()`] command on a
/// bundle `B`.
///
/// The bundle must implement [`Default`], which is used to create the first entity that gets cloned
/// in the benchmark.
///
/// If `clone_via_reflect` is false, this will use the default [`ComponentCloneHandler`] for all
/// components (which is usually [`ComponentCloneHandler::clone_handler()`]). If `clone_via_reflect`
/// is true, it will overwrite the handler for all components in the bundle to be
/// [`ComponentCloneHandler::reflect_handler()`].
fn bench_clone<B: Bundle + Default + GetTypeRegistration>(
    b: &mut Bencher,
    clone_via_reflect: bool,
) {
    let mut world = World::default();

    let mut cloner = if clone_via_reflect {
        reflection_cloner::<B>(&mut world, false)
    } else {
        EntityCloner::default()
    };

    // Spawn the first entity, which will be cloned in the benchmark routine.
    let id = world.spawn(B::default()).id();

    b.iter(|| {
        // clones the given entity
        cloner.spawn_clone(&mut world, black_box(id));
        world.flush();
    });
}

/// A helper function that benchmarks running the [`EntityCommands::clone_and_spawn()`] command on a
/// bundle `B`.
///
/// As compared to [`bench_clone()`], this benchmarks recursively cloning an entity with several
/// children. It does so by setting up an entity tree with a given `height` where each entity has a
/// specified number of `children`.
///
/// For example, setting `height` to 5 and `children` to 1 creates a single chain of entities with
/// no siblings. Alternatively, setting `height` to 1 and `children` to 5 will spawn 5 direct
/// children of the root entity.
fn bench_clone_hierarchy<B: Bundle + Default + GetTypeRegistration>(
    b: &mut Bencher,
    height: usize,
    children: usize,
    clone_via_reflect: bool,
) {
    let mut world = World::default();

    let mut cloner = if clone_via_reflect {
        reflection_cloner::<B>(&mut world, true)
    } else {
        let mut builder = EntityCloner::build(&mut world);
        builder.linked_cloning(true);
        builder.finish()
    };

    // Make the clone command recursive, so children are cloned as well.

    // Spawn the first entity, which will be cloned in the benchmark routine.
    let id = world.spawn(B::default()).id();

    let mut hierarchy_level = vec![id];

    // Set up the hierarchy tree by spawning all children.
    for _ in 0..height {
        let current_hierarchy_level = hierarchy_level.clone();

        hierarchy_level.clear();

        for parent in current_hierarchy_level {
            for _ in 0..children {
                let child_id = world.spawn((B::default(), ChildOf(parent))).id();
                hierarchy_level.push(child_id);
            }
        }
    }

    b.iter(|| {
        cloner.spawn_clone(&mut world, black_box(id));
        world.flush();
    });
}

// Each benchmark runs twice: using either the `Clone` or `Reflect` traits to clone entities. This
// constant represents this as an easy array that can be used in a `for` loop.
const SCENARIOS: [(&str, bool); 2] = [("clone", false), ("reflect", true)];

/// Benchmarks cloning a single entity with 10 components and no children.
fn single(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("single"));

    // We're cloning 1 entity.
    group.throughput(Throughput::Elements(1));

    for (id, clone_via_reflect) in SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone::<ComplexBundle>(b, clone_via_reflect);
        });
    }

    group.finish();
}

/// Benchmarks cloning an an entity and its 50 descendents, each with only 1 component.
fn hierarchy_tall(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("hierarchy_tall"));

    // We're cloning both the root entity and its 50 descendents.
    group.throughput(Throughput::Elements(51));

    for (id, clone_via_reflect) in SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone_hierarchy::<C1>(b, 50, 1, clone_via_reflect);
        });
    }

    group.finish();
}

/// Benchmarks cloning an an entity and its 50 direct children, each with only 1 component.
fn hierarchy_wide(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("hierarchy_wide"));

    // We're cloning both the root entity and its 50 direct children.
    group.throughput(Throughput::Elements(51));

    for (id, clone_via_reflect) in SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone_hierarchy::<C1>(b, 1, 50, clone_via_reflect);
        });
    }

    group.finish();
}

/// Benchmarks cloning a large hierarchy of entities with several children each. Each entity has 10
/// components.
fn hierarchy_many(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("hierarchy_many"));

    // We're cloning 364 entities total. This number was calculated by manually counting the number
    // of entities spawned in `bench_clone_hierarchy()` with a `println!()` statement. :)
    group.throughput(Throughput::Elements(364));

    for (id, clone_via_reflect) in SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone_hierarchy::<ComplexBundle>(b, 5, 3, clone_via_reflect);
        });
    }

    group.finish();
}
