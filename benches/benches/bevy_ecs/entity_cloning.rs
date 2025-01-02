use core::hint::black_box;

use benches::bench;
use bevy_ecs::bundle::Bundle;
use bevy_ecs::component::ComponentCloneHandler;
use bevy_ecs::reflect::AppTypeRegistry;
use bevy_ecs::{component::Component, reflect::ReflectComponent, world::World};
use bevy_hierarchy::{BuildChildren, CloneEntityHierarchyExt};
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
#[reflect(Component)]
struct Foo(Mat4);

/// Sets the [`ComponentCloneHandler`] for all explicit and required components in a bundle `B` to
/// use the [`Reflect`] trait instead of [`Clone`].
fn set_reflect_clone_handler<B: Bundle + GetTypeRegistration>(world: &mut World) {
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

    let clone_handlers = world.get_component_clone_handlers_mut();

    // Overwrite the clone handler for all components in the bundle to use `Reflect`, not `Clone`.
    for component in component_ids {
        clone_handlers.set_component_handler(component, ComponentCloneHandler::reflect_handler());
    }
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

    if clone_via_reflect {
        set_reflect_clone_handler::<B>(&mut world);
    }

    // Spawn the first entity, which will be cloned in the benchmark routine.
    let id = world.spawn(B::default()).id();

    b.iter(|| {
        // Queue the command to clone the entity.
        world.commands().entity(black_box(id)).clone_and_spawn();

        // Run the command.
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

    if clone_via_reflect {
        set_reflect_clone_handler::<B>(&mut world);
    }

    // Spawn the first entity, which will be cloned in the benchmark routine.
    let id = world.spawn(B::default()).id();

    let mut hierarchy_level = vec![id];

    // Set up the hierarchy tree by spawning all children.
    for _ in 0..height {
        let current_hierarchy_level = hierarchy_level.clone();

        hierarchy_level.clear();

        for parent_id in current_hierarchy_level {
            for _ in 0..children {
                let child_id = world.spawn(B::default()).set_parent(parent_id).id();

                hierarchy_level.push(child_id);
            }
        }
    }

    // Flush all `set_parent()` commands.
    world.flush();

    b.iter(|| {
        world
            .commands()
            .entity(black_box(id))
            .clone_and_spawn_with(|builder| {
                // Make the clone command recursive, so children are cloned as well.
                builder.recursive(true);
            });

        world.flush();
    });
}

// Each benchmark runs twice: using either the `Clone` or `Reflect` traits to clone entities. This
// constant represents this as an easy array that can be used in a `for` loop.
const SCENARIOS: [(&'static str, bool); 2] = [("clone", false), ("reflect", true)];

fn single(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("single"));

    // We're cloning 1 entity.
    group.throughput(Throughput::Elements(1));

    for (id, clone_via_reflect) in SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone::<Foo>(b, clone_via_reflect);
        });
    }

    group.finish();
}

fn hierarchy_tall(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("hierarchy_tall"));

    // We're cloning both the root entity and its 50 descendents.
    group.throughput(Throughput::Elements(51));

    for (id, clone_via_reflect) in SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone_hierarchy::<Foo>(b, 50, 1, clone_via_reflect);
        });
    }

    group.finish();
}

fn hierarchy_wide(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("hierarchy_wide"));

    // We're cloning both the root entity and its 50 direct children.
    group.throughput(Throughput::Elements(51));

    for (id, clone_via_reflect) in SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone_hierarchy::<Foo>(b, 1, 50, clone_via_reflect);
        });
    }

    group.finish();
}

fn hierarchy_many(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("hierarchy_many"));

    // We're cloning 3,906 entities total. This number was calculated by manually counting the
    // number of entities spawned in `bench_clone_hierarchy()` with a `println!()` statement. :)
    group.throughput(Throughput::Elements(3906));

    // The default 5 seconds are not enough here.
    group.measurement_time(std::time::Duration::from_secs(8));

    for (id, clone_via_reflect) in SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone_hierarchy::<Foo>(b, 5, 5, clone_via_reflect);
        });
    }

    group.finish();
}
