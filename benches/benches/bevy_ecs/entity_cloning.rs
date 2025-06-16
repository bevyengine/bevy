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
    single_unfiltered_all,
    hierarchy_tall,
    hierarchy_wide,
    hierarchy_many,
);

#[derive(Component, Default, Clone)]
#[require(C1)]
struct E1(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C1(Mat4);

#[derive(Component, Default, Clone)]
#[require(C2)]
struct E2(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C2(Mat4);

#[derive(Component, Default, Clone)]
#[require(C3)]
struct E3(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C3(Mat4);

#[derive(Component, Default, Clone)]
#[require(C4)]
struct E4(Mat4);

#[derive(Component, Reflect, Default, Clone)]
struct C4(Mat4);

#[derive(Component, Default, Clone)]
#[require(C5)]
struct E5(Mat4);

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

/// Sets the [`ComponentCloneBehavior`] for all explicit and required components in a bundle `B` to
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

    let mut builder = EntityCloner::build_allow_all(world);

    // Overwrite the clone handler for all components in the bundle to use `Reflect`, not `Clone`.
    for component in component_ids {
        builder.override_clone_behavior_with_id(component, ComponentCloneBehavior::reflect());
    }
    builder.linked_cloning(linked_cloning);

    builder.finish()
}

/// A helper function that benchmarks running [`EntityCloner::spawn_clone`] with a bundle `B`.
///
/// The bundle must implement [`Default`], which is used to create the first entity that gets cloned
/// in the benchmark.
///
/// If `clone_via_reflect` is false, this will use the default [`ComponentCloneBehavior`] for all
/// components (which is usually [`ComponentCloneBehavior::clone()`]). If `clone_via_reflect`
/// is true, it will overwrite the handler for all components in the bundle to be
/// [`ComponentCloneBehavior::reflect()`].
fn bench_clone_single_unfiltered_all<B: Bundle + Default + GetTypeRegistration>(
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

/// A helper function that benchmarks running [`EntityCloner::clone_entity`] with[`bevy_ecs::entity::DenyAll`]
/// filter and allowing 5 components which each have one required component.
fn bench_single_filtered_complex_into_target(b: &mut Bencher, scenario: &str) {
    let mut world = World::default();

    let (source, target);

    type Explicit = (E1, E2, E3, E4, E5);
    type Required = (C1, C2, C3, C4, C5);

    let cloner = match scenario {
        deny_all_scenarios::ALL_PASS => {
            source = world.spawn(Explicit::default()).id();
            target = world.spawn_empty().id();
            let mut cloner = EntityCloner::build_deny_all(&mut world);
            cloner.allow::<Explicit>();
            cloner
        }
        deny_all_scenarios::ALL_PASS_AS_NEW => {
            source = world.spawn(Explicit::default()).id();
            target = world.spawn_empty().id();
            let mut cloner = EntityCloner::build_deny_all(&mut world);
            cloner.allow_if_new::<Explicit>();
            cloner
        }
        deny_all_scenarios::EXPLICIT_MISSING => {
            source = world.spawn_empty().id();
            target = world.spawn_empty().id();
            let mut cloner = EntityCloner::build_deny_all(&mut world);
            cloner.allow::<Explicit>();
            cloner
        }
        deny_all_scenarios::EXPLICIT_NOT_NEW => {
            source = world.spawn(Explicit::default()).id();
            target = world.spawn(Explicit::default()).id();
            let mut cloner = EntityCloner::build_deny_all(&mut world);
            cloner.allow_if_new::<Explicit>();
            cloner
        }
        deny_all_scenarios::REQUIRED_NOT_NEW => {
            source = world.spawn(Explicit::default()).id();
            target = world.spawn(Required::default()).id();
            let mut cloner = EntityCloner::build_deny_all(&mut world);
            cloner.allow::<Explicit>();
            cloner
        }
        _ => unimplemented!(),
    };

    let mut cloner = cloner.finish();

    b.iter(|| {
        // clones the given entity's components to the target
        cloner.clone_entity(&mut world, black_box(source), black_box(target));
        world.flush();
    });
}

/// A helper function that benchmarks running [`EntityCloner::spawn_clone`] with a bundle `B`.
///
/// As compared to [`bench_clone_single_unfiltered_all()`], this benchmarks recursively cloning an
/// entity with several children. It does so by setting up an entity tree with a given `height`
/// where each entity has a specified number of `children`.
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
        let mut builder = EntityCloner::build_allow_all(&mut world);
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
const SINGLE_UNFILTERED_ALL_SCENARIOS: [(&str, bool); 2] = [("clone", false), ("reflect", true)];

/// Benchmarks cloning a single entity with 10 components and no children.
fn single_unfiltered_all(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("single_unfiltered_all"));

    // We're cloning 1 entity.
    group.throughput(Throughput::Elements(1));

    for (id, clone_via_reflect) in SINGLE_UNFILTERED_ALL_SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone_single_unfiltered_all::<ComplexBundle>(b, clone_via_reflect);
        });
    }

    group.finish();
}

mod deny_all_scenarios {
    pub const ALL_PASS: &str = "all_pass";
    pub const ALL_PASS_AS_NEW: &str = "all_pass_as_new";
    pub const EXPLICIT_MISSING: &str = "explicit_missing";
    pub const EXPLICIT_NOT_NEW: &str = "explicit_not_new";
    pub const REQUIRED_NOT_NEW: &str = "required_not_new";
}

const SINGLE_FILTERED_INTO_TARGET_SCENARIOS: [&str; 5] = [
    deny_all_scenarios::ALL_PASS,
    deny_all_scenarios::ALL_PASS_AS_NEW,
    deny_all_scenarios::EXPLICIT_MISSING,
    deny_all_scenarios::EXPLICIT_NOT_NEW,
    deny_all_scenarios::REQUIRED_NOT_NEW,
];

/// Benchmarks cloning a single entity with 10 components where each needs to be evaluated
fn single_filtered_into_target(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("single_filtered_into_target"));

    // We're cloning 1 entity into another.
    group.throughput(Throughput::Elements(1));

    for scenario in SINGLE_FILTERED_INTO_TARGET_SCENARIOS {
        group.bench_function(scenario, |b| {
            bench_single_filtered_complex_into_target(b, scenario);
        });
    }

    group.finish();
}

/// Benchmarks cloning an an entity and its 50 descendents, each with only 1 component.
fn hierarchy_tall(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("hierarchy_tall"));

    // We're cloning both the root entity and its 50 descendents.
    group.throughput(Throughput::Elements(51));

    for (id, clone_via_reflect) in SINGLE_UNFILTERED_ALL_SCENARIOS {
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

    for (id, clone_via_reflect) in SINGLE_UNFILTERED_ALL_SCENARIOS {
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

    for (id, clone_via_reflect) in SINGLE_UNFILTERED_ALL_SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone_hierarchy::<ComplexBundle>(b, 5, 3, clone_via_reflect);
        });
    }

    group.finish();
}
