use core::hint::black_box;

use benches::bench;
use bevy_ecs::bundle::{Bundle, InsertMode};
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
    filter
);

#[derive(Component, Reflect, Default, Clone)]
struct C<const N: usize>(Mat4);

type ComplexBundle = (C<1>, C<2>, C<3>, C<4>, C<5>, C<6>, C<7>, C<8>, C<9>, C<10>);

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

    let mut builder = EntityCloner::build_opt_out(world);

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

/// A helper function that benchmarks running [`EntityCloner::spawn_clone`] with a bundle `B`.
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
        let mut builder = EntityCloner::build_opt_out(&mut world);
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
const CLONE_SCENARIOS: [(&str, bool); 2] = [("clone", false), ("reflect", true)];

/// Benchmarks cloning a single entity with 10 components and no children.
fn single(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("single"));

    // We're cloning 1 entity.
    group.throughput(Throughput::Elements(1));

    for (id, clone_via_reflect) in CLONE_SCENARIOS {
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

    for (id, clone_via_reflect) in CLONE_SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone_hierarchy::<C<1>>(b, 50, 1, clone_via_reflect);
        });
    }

    group.finish();
}

/// Benchmarks cloning an an entity and its 50 direct children, each with only 1 component.
fn hierarchy_wide(c: &mut Criterion) {
    let mut group = c.benchmark_group(bench!("hierarchy_wide"));

    // We're cloning both the root entity and its 50 direct children.
    group.throughput(Throughput::Elements(51));

    for (id, clone_via_reflect) in CLONE_SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone_hierarchy::<C<1>>(b, 1, 50, clone_via_reflect);
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

    for (id, clone_via_reflect) in CLONE_SCENARIOS {
        group.bench_function(id, |b| {
            bench_clone_hierarchy::<ComplexBundle>(b, 5, 3, clone_via_reflect);
        });
    }

    group.finish();
}

/// Filter scenario variant for bot opt-in and opt-out filters
#[derive(Clone, Copy)]
#[expect(
    clippy::enum_variant_names,
    reason = "'Opt' is not understood as an prefix but `OptOut'/'OptIn' are"
)]
enum FilterScenario {
    OptOutNone,
    OptOutNoneKeep(bool),
    OptOutAll,
    OptInNone,
    OptInAll,
    OptInAllWithoutRequired,
    OptInAllKeep(bool),
    OptInAllKeepWithoutRequired(bool),
}

impl From<FilterScenario> for String {
    fn from(value: FilterScenario) -> Self {
        match value {
            FilterScenario::OptOutNone => "opt_out_none",
            FilterScenario::OptOutNoneKeep(true) => "opt_out_none_keep_none",
            FilterScenario::OptOutNoneKeep(false) => "opt_out_none_keep_all",
            FilterScenario::OptOutAll => "opt_out_all",
            FilterScenario::OptInNone => "opt_in_none",
            FilterScenario::OptInAll => "opt_in_all",
            FilterScenario::OptInAllWithoutRequired => "opt_in_all_without_required",
            FilterScenario::OptInAllKeep(true) => "opt_in_all_keep_none",
            FilterScenario::OptInAllKeep(false) => "opt_in_all_keep_all",
            FilterScenario::OptInAllKeepWithoutRequired(true) => {
                "opt_in_all_keep_none_without_required"
            }
            FilterScenario::OptInAllKeepWithoutRequired(false) => {
                "opt_in_all_keep_all_without_required"
            }
        }
        .into()
    }
}

/// Common scenarios for different filter to be benchmarked.
const FILTER_SCENARIOS: [FilterScenario; 11] = [
    FilterScenario::OptOutNone,
    FilterScenario::OptOutNoneKeep(true),
    FilterScenario::OptOutNoneKeep(false),
    FilterScenario::OptOutAll,
    FilterScenario::OptInNone,
    FilterScenario::OptInAll,
    FilterScenario::OptInAllWithoutRequired,
    FilterScenario::OptInAllKeep(true),
    FilterScenario::OptInAllKeep(false),
    FilterScenario::OptInAllKeepWithoutRequired(true),
    FilterScenario::OptInAllKeepWithoutRequired(false),
];

/// A helper function that benchmarks running [`EntityCloner::clone_entity`] with a bundle `B`.
///
/// The bundle must implement [`Default`], which is used to create the first entity that gets its components cloned
/// in the benchmark. It may also be used to populate the target entity depending on the scenario.
fn bench_filter<B: Bundle + Default>(b: &mut Bencher, scenario: FilterScenario) {
    let mut world = World::default();
    let mut spawn = |empty| match empty {
        false => world.spawn(B::default()).id(),
        true => world.spawn_empty().id(),
    };
    let source = spawn(false);
    let (target, mut cloner);

    match scenario {
        FilterScenario::OptOutNone => {
            target = spawn(true);
            cloner = EntityCloner::default();
        }
        FilterScenario::OptOutNoneKeep(is_new) => {
            target = spawn(is_new);
            let mut builder = EntityCloner::build_opt_out(&mut world);
            builder.insert_mode(InsertMode::Keep);
            cloner = builder.finish();
        }
        FilterScenario::OptOutAll => {
            target = spawn(true);
            let mut builder = EntityCloner::build_opt_out(&mut world);
            builder.deny::<B>();
            cloner = builder.finish();
        }
        FilterScenario::OptInNone => {
            target = spawn(true);
            let builder = EntityCloner::build_opt_in(&mut world);
            cloner = builder.finish();
        }
        FilterScenario::OptInAll => {
            target = spawn(true);
            let mut builder = EntityCloner::build_opt_in(&mut world);
            builder.allow::<B>();
            cloner = builder.finish();
        }
        FilterScenario::OptInAllWithoutRequired => {
            target = spawn(true);
            let mut builder = EntityCloner::build_opt_in(&mut world);
            builder.without_required_components(|builder| {
                builder.allow::<B>();
            });
            cloner = builder.finish();
        }
        FilterScenario::OptInAllKeep(is_new) => {
            target = spawn(is_new);
            let mut builder = EntityCloner::build_opt_in(&mut world);
            builder.allow_if_new::<B>();
            cloner = builder.finish();
        }
        FilterScenario::OptInAllKeepWithoutRequired(is_new) => {
            target = spawn(is_new);
            let mut builder = EntityCloner::build_opt_in(&mut world);
            builder.without_required_components(|builder| {
                builder.allow_if_new::<B>();
            });
            cloner = builder.finish();
        }
    }

    b.iter(|| {
        // clones the given entity into the target
        cloner.clone_entity(&mut world, black_box(source), black_box(target));
        world.flush();
    });
}

/// Benchmarks filtering of cloning a single entity with 5 unclonable components (each requiring 1 unclonable component) into a target.
fn filter(c: &mut Criterion) {
    #[derive(Component, Default)]
    #[component(clone_behavior = Ignore)]
    struct C<const N: usize>;

    #[derive(Component, Default)]
    #[component(clone_behavior = Ignore)]
    #[require(C::<N>)]
    struct R<const N: usize>;

    type RequiringBundle = (R<1>, R<2>, R<3>, R<4>, R<5>);

    let mut group = c.benchmark_group(bench!("filter"));

    // We're cloning 1 entity into a target.
    group.throughput(Throughput::Elements(1));

    for scenario in FILTER_SCENARIOS {
        group.bench_function(scenario, |b| {
            bench_filter::<RequiringBundle>(b, scenario);
        });
    }

    group.finish();
}
