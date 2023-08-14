//! Hierarchy and transform propagation benchmark, derived from the stress test example.
//!
//! For the configurations, see the stress test documentation.

use bevy_transform::prelude::*;
use bevy_app::prelude::*;
use bevy_ecs::{prelude::*, schedule::ScheduleLabel};
use bevy_math::prelude::*;
use bevy_core_pipeline::prelude::Camera2dBundle;
use bevy_time::{Time, TimePlugin};
use bevy_hierarchy::{Children, Parent, BuildWorldChildren};
use bevy_utils::default;

use rand::{Rng, rngs::SmallRng, SeedableRng};

use std::time::{Instant, Duration};

use criterion::black_box;
// Available replacement for criterion::black_box from Rust 1.66
// use std::hint::black_box;

use criterion::*;

/// pre-defined benchmark configurations with name
const CONFIGS: [(&str, Cfg); 9] = [
    (
        "large_tree",
        Cfg {
            test_case: TestCase::NonUniformTree {
                depth: 18,
                branch_width: 8,
            },
            update_filter: UpdateFilter {
                probability: 0.5,
                min_depth: 0,
                max_depth: u32::MAX,
            },
        },
    ),
    (
        "wide_tree",
        Cfg {
            test_case: TestCase::Tree {
                depth: 3,
                branch_width: 500,
            },
            update_filter: UpdateFilter {
                probability: 0.5,
                min_depth: 0,
                max_depth: u32::MAX,
            },
        },
    ),
    (
        "deep_tree",
        Cfg {
            test_case: TestCase::NonUniformTree {
                depth: 25,
                branch_width: 2,
            },
            update_filter: UpdateFilter {
                probability: 0.5,
                min_depth: 0,
                max_depth: u32::MAX,
            },
        },
    ),
    (
        "chain",
        Cfg {
            test_case: TestCase::Tree {
                depth: 2500,
                branch_width: 1,
            },
            update_filter: UpdateFilter {
                probability: 0.5,
                min_depth: 0,
                max_depth: u32::MAX,
            },
        },
    ),
    (
        "update_leaves",
        Cfg {
            test_case: TestCase::Tree {
                depth: 18,
                branch_width: 2,
            },
            update_filter: UpdateFilter {
                probability: 0.5,
                min_depth: 17,
                max_depth: u32::MAX,
            },
        },
    ),
    (
        "update_shallow",
        Cfg {
            test_case: TestCase::Tree {
                depth: 18,
                branch_width: 2,
            },
            update_filter: UpdateFilter {
                probability: 0.5,
                min_depth: 0,
                max_depth: 8,
            },
        },
    ),
    (
        "humanoids_active",
        Cfg {
            test_case: TestCase::Humanoids {
                active: 4000,
                inactive: 0,
            },
            update_filter: UpdateFilter {
                probability: 1.0,
                min_depth: 0,
                max_depth: u32::MAX,
            },
        },
    ),
    (
        "humanoids_inactive",
        Cfg {
            test_case: TestCase::Humanoids {
                active: 10,
                inactive: 3990,
            },
            update_filter: UpdateFilter {
                probability: 1.0,
                min_depth: 0,
                max_depth: u32::MAX,
            },
        },
    ),
    (
        "humanoids_mixed",
        Cfg {
            test_case: TestCase::Humanoids {
                active: 2000,
                inactive: 2000,
            },
            update_filter: UpdateFilter {
                probability: 1.0,
                min_depth: 0,
                max_depth: u32::MAX,
            },
        },
    ),
];

// Random seed for tree creation
// (kept constant to make benchmark results comparable)
const SEED: u64 = 0x94eb0d25004f5f17;

#[derive(PartialEq, Clone, Copy)]
enum TransformUpdates { Enabled, Disabled }

fn build_app(cfg: &Cfg, enable_update: TransformUpdates) -> App {
    let mut app = App::new();

    app.add_plugins(TransformPlugin);

    if enable_update == TransformUpdates::Enabled {
        app
            .add_plugins(TimePlugin)
            .add_systems(Update, update);
    }

    // Finish Plugin setup - identical to what the ScheduleRunnerPlugin runner does
    // We can't use the ScheduleRunnerPlugin since we run app.update() ourselves,
    // and app.run() can't be called repeatedly when using RunMode::Once

    // Do any of the plugins we use in the benchmarks require any asynchronous
    // initialization using task pools?
    // Currently, this is never the case, but the code is kept here as a reference
    // in case it becomes necessary in the future.
    const ASYNC_PLUGIN_INIT: bool = false;
    if ASYNC_PLUGIN_INIT {
        while !app.ready() {
            #[cfg(not(target_arch = "wasm32"))]
            bevy_tasks::tick_global_task_pools_on_main_thread();
        }
    }
    assert!(app.ready());

    app.finish();
    app.cleanup();

    // Run setup (what would normally happen in the Startup schedule)
    setup(&mut app.world, cfg);

    app
}

criterion_group!{
    name = transform_hierarchy_benches;
    config = Criterion::default()
        .warm_up_time(std::time::Duration::from_secs(3))
        .measurement_time(std::time::Duration::from_secs(20));
    targets = transform_init, transform_propagation
}

/// This benchmark tries to measure the cost of the initial transform propagation,
/// i.e. the first time transform propagation runs after we just added all our entities.
fn transform_init(c: &mut Criterion) {
    let mut group = c.benchmark_group("transform_init");

    for (name, cfg) in &CONFIGS {
        // Simplified benchmark for the initial propagation
        group.bench_with_input(BenchmarkId::new("reset", name), cfg, |b, cfg| {
            // Building the World (in setup) takes a lot of time, so we shouldn't do that on every
            // iteration.
            // Unfortunately, we can't re-use an App directly in iter() because the World would no
            // longer be in its pristine, just initialized state from the second iteration onwards.
            // Furthermore, it's not possible to clone a pristine World since World doesn't implement
            // Clone.
            // As an alternative, we reuse the same App and reset it to a pseudo-pristine state by
            // simply marking all Parent, Children and Transform components as changed.
            // This should look like a pristine state to the propagation systems.

            let mut app = build_app(cfg, TransformUpdates::Disabled);

            app.add_schedule(ResetSchedule, reset_schedule());

            // Run Main schedule once to ensure initial updates are done
            // This is a little counterintuitive since the initial delay is exactly what we want to
            // measure - however, we have the ResetSchedule in place to hopefully replicate the
            // World in its pristine state on every iteration.
            // We therefore run update here to prevent the first iteration having additional work
            // due to possible incompleteness of the reset mechanism
            app.update();

            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;

                for _i in 0..iters {
                    black_box(app.world.run_schedule(ResetSchedule));

                    let start = Instant::now();
                    black_box(app.world.run_schedule(bevy_app::Main));
                    let elapsed = start.elapsed();

                    total += elapsed;
                }

                total
            });
        });

        // Reference benchmark for the initial propagation - needs to rebuild the App
        // on every iteration, which makes the benchmark quite slow and results
        // in less precise results in the same time compared to the simplified benchmark.
        
        // Reduce sample size and enable flat sampling to make sure this benchmark doesn't
        // take a lot longer than the simplified benchmark.
        group.sample_size(50);
        group.sampling_mode(SamplingMode::Flat);
        group.bench_with_input(BenchmarkId::new("reference", name), cfg, |b, cfg| {
            // Use iter_batched_ref to prevent influence of Drop
            b.iter_batched_ref(
                || {
                    build_app(cfg, TransformUpdates::Disabled)
                },
                App::update,
                BatchSize::PerIteration,
            );
        });
    }
}


fn transform_propagation(c: &mut Criterion) {
    let mut group = c.benchmark_group("transform_propagation");

    let do_bench = |b: &mut Bencher<_>, &(cfg, enable_update): &(&Cfg, TransformUpdates)| {
        let mut app = build_app(cfg, enable_update);

        // Run Main schedule once to ensure initial updates are done
        app.update();

        b.iter(move || { app.update(); });
    };

    for (name, cfg) in &CONFIGS {
        // Measures hierarchy propagation systems when some transforms are updated.
        group.bench_with_input(BenchmarkId::new("transform_updates", name), &(cfg, TransformUpdates::Enabled), do_bench);

        // Measures hierarchy propagation systems when there are no changes
        // during the Update schedule.
        group.bench_with_input(BenchmarkId::new("noop", name), &(cfg, TransformUpdates::Disabled), do_bench);
    }
}

/// test configuration
#[derive(Resource, Debug, Clone)]
struct Cfg {
    /// which test case should be inserted
    test_case: TestCase,
    /// which entities should be updated
    update_filter: UpdateFilter,
}

#[allow(unused)]
#[derive(Debug, Clone)]
enum TestCase {
    /// a uniform tree, exponentially growing with depth
    Tree {
        /// total depth
        depth: u32,
        /// number of children per node
        branch_width: u32,
    },
    /// a non uniform tree (one side is deeper than the other)
    /// creates significantly less nodes than `TestCase::Tree` with the same parameters
    NonUniformTree {
        /// the maximum depth
        depth: u32,
        /// max number of children per node
        branch_width: u32,
    },
    /// one or multiple humanoid rigs
    Humanoids {
        /// number of active instances (uses the specified [`UpdateFilter`])
        active: u32,
        /// number of inactive instances (always inactive)
        inactive: u32,
    },
}

/// a filter to restrict which nodes are updated
#[derive(Debug, Clone)]
struct UpdateFilter {
    /// starting depth (inclusive)
    min_depth: u32,
    /// end depth (inclusive)
    max_depth: u32,
    /// probability of a node to get updated (evaluated at insertion time, not during update)
    /// 0 (never) .. 1 (always)
    probability: f32,
}

/// update component with some per-component value
#[derive(Component)]
struct UpdateValue(f32);

/// update positions system
fn update(time: Res<Time>, mut query: Query<(&mut Transform, &mut UpdateValue)>) {
    for (mut t, mut u) in &mut query {
        u.0 += time.delta_seconds() * 0.1;
        set_translation(&mut t.translation, u.0);
    }
}

/// mark all transforms as changed
fn reset_transforms(mut transform_query: Query<&mut Transform>) {
    for mut transform in transform_query.iter_mut() {
        transform.set_changed();
    }
}

/// mark all parents as changed
fn reset_parents(mut parent_query: Query<&mut Parent>) {
    for mut parent in parent_query.iter_mut() {
        parent.set_changed();
    }
}

/// mark all children as changed
fn reset_children(mut children_query: Query<&mut Children>) {
    for mut children in children_query.iter_mut() {
        children.set_changed();
    }
}

/// create a Schedule that resets all that Components that are tracked
/// by transform propagation, such that the World appears as it had just
/// been created
fn reset_schedule() -> Schedule {
    let mut schedule = Schedule::new();
    schedule.add_systems((reset_transforms, reset_parents, reset_children));
    schedule
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, ScheduleLabel)]
struct ResetSchedule;

/// set translation based on the angle `a`
fn set_translation(translation: &mut Vec3, a: f32) {
    translation.x = a.cos() * 32.0;
    translation.y = a.sin() * 32.0;
}

fn setup(world: &mut World, cfg: &Cfg) -> InsertResult {
    let mut cam = Camera2dBundle::default();

    cam.transform.translation.z = 100.0;
    world.spawn(cam);

    let mut rng = rand::rngs::SmallRng::seed_from_u64(SEED);

    match cfg.test_case {
        TestCase::Tree {
            depth,
            branch_width,
        } => {
            let tree = gen_tree(depth, branch_width);
            spawn_tree(&tree, world, &cfg.update_filter, default(), rng)
        }
        TestCase::NonUniformTree {
            depth,
            branch_width,
        } => {
            let tree = gen_non_uniform_tree(depth, branch_width);
            spawn_tree(&tree, world, &cfg.update_filter, default(), rng)
        }
        TestCase::Humanoids { active, inactive } => {
            let mut result = InsertResult::default();

            for _ in 0..active {
                let mut rng = SmallRng::from_rng(&mut rng).unwrap();

                result.combine(spawn_tree(
                    &HUMANOID_RIG,
                    world,
                    &cfg.update_filter,
                    Transform::from_xyz(
                        rng.gen::<f32>() * 500.0 - 250.0,
                        rng.gen::<f32>() * 500.0 - 250.0,
                        0.0,
                    ),
                    rng,
                ));
            }

            for _ in 0..inactive {
                let mut rng = SmallRng::from_rng(&mut rng).unwrap();

                let transform = Transform::from_xyz(
                        rng.gen::<f32>() * 500.0 - 250.0,
                        rng.gen::<f32>() * 500.0 - 250.0,
                        0.0,
                    );

                result.combine(spawn_tree(
                    &HUMANOID_RIG,
                    world,
                    &UpdateFilter {
                        // force inactive by setting the probability < 0
                        probability: -1.0,
                        ..cfg.update_filter
                    },
                    transform,
                    rng,
                ));
            }

            result
        }
    }
}

/// overview of the inserted hierarchy
#[derive(Default, Debug)]
struct InsertResult {
    /// total number of nodes inserted
    inserted_nodes: usize,
    /// number of nodes that get updated each frame
    active_nodes: usize,
    /// maximum depth of the hierarchy tree
    maximum_depth: usize,
}

impl InsertResult {
    fn combine(&mut self, rhs: Self) -> &mut Self {
        self.inserted_nodes += rhs.inserted_nodes;
        self.active_nodes += rhs.active_nodes;
        self.maximum_depth = self.maximum_depth.max(rhs.maximum_depth);
        self
    }
}

/// spawns a tree defined by a parent map (excluding root)
/// the parent map must be ordered (parent must exist before child)
fn spawn_tree(
    parent_map: &[usize],
    world: &mut World,
    update_filter: &UpdateFilter,
    root_transform: Transform,
    mut rng: SmallRng,
) -> InsertResult {
    // total count (# of nodes + root)
    let count = parent_map.len() + 1;

    #[derive(Default, Clone, Copy)]
    struct NodeInfo {
        child_count: u32,
        depth: u32,
    }

    // node index -> entity lookup list
    let mut ents: Vec<Entity> = Vec::with_capacity(count);
    let mut node_info: Vec<NodeInfo> = vec![default(); count];
    for (i, &parent_idx) in parent_map.iter().enumerate() {
        // assert spawn order (parent must be processed before child)
        assert!(parent_idx <= i, "invalid spawn order");
        node_info[parent_idx].child_count += 1;
    }

    // insert root
    ents.push(world.spawn(TransformBundle::from(root_transform)).id());

    let mut result = InsertResult::default();
    // used to count through the number of children (used only for visual layout)
    let mut child_idx: Vec<u16> = vec![0; count];

    // insert children
    for (current_idx, &parent_idx) in parent_map.iter().enumerate() {
        let current_idx = current_idx + 1;

        // separation factor to visually separate children (0..1)
        let sep = child_idx[parent_idx] as f32 / node_info[parent_idx].child_count as f32;
        child_idx[parent_idx] += 1;

        // calculate and set depth
        // this works because it's guaranteed that we have already iterated over the parent
        let depth = node_info[parent_idx].depth + 1;
        let info = &mut node_info[current_idx];
        info.depth = depth;

        // update max depth of tree
        result.maximum_depth = result.maximum_depth.max(depth.try_into().unwrap());

        // insert child
        let child_entity = {
            let mut cmd = world.spawn_empty();

            // check whether or not to update this node
            let update = (rng.gen::<f32>() <= update_filter.probability)
                && (depth >= update_filter.min_depth && depth <= update_filter.max_depth);

            if update {
                cmd.insert(UpdateValue(sep));
                result.active_nodes += 1;
            }

            let transform = {
                let mut translation = Vec3::ZERO;
                // use the same placement fn as the `update` system
                // this way the entities won't be all at (0, 0, 0) when they don't have an `Update` component
                set_translation(&mut translation, sep);
                Transform::from_translation(translation)
            };

            // only insert the components necessary for the transform propagation
            cmd.insert(TransformBundle::from(transform));

            cmd.id()
        };

        world
            .get_or_spawn(ents[parent_idx])
            .expect("error spawning parent entity")
            .add_child(child_entity);

        ents.push(child_entity);
    }

    result.inserted_nodes = ents.len();
    result
}

/// generate a tree `depth` levels deep, where each node has `branch_width` children
fn gen_tree(depth: u32, branch_width: u32) -> Vec<usize> {
    // calculate the total count of branches
    let mut count: usize = 0;
    for i in 0..(depth - 1) {
        count += TryInto::<usize>::try_into(branch_width.pow(i)).unwrap();
    }

    // the tree is built using this pattern:
    // 0, 0, 0, ... 1, 1, 1, ... 2, 2, 2, ... (count - 1)
    (0..count)
        .flat_map(|i| std::iter::repeat(i).take(branch_width.try_into().unwrap()))
        .collect()
}

/// recursive part of [`gen_non_uniform_tree`]
fn add_children_non_uniform(
    tree: &mut Vec<usize>,
    parent: usize,
    mut curr_depth: u32,
    max_branch_width: u32,
) {
    for _ in 0..max_branch_width {
        tree.push(parent);

        curr_depth = curr_depth.checked_sub(1).unwrap();
        if curr_depth == 0 {
            return;
        }
        add_children_non_uniform(tree, tree.len(), curr_depth, max_branch_width);
    }
}

/// generate a tree that has more nodes on one side that the other
/// the deepest hierarchy path is `max_depth` and the widest branches have `max_branch_width` children
fn gen_non_uniform_tree(max_depth: u32, max_branch_width: u32) -> Vec<usize> {
    let mut tree = Vec::new();
    add_children_non_uniform(&mut tree, 0, max_depth, max_branch_width);
    tree
}

/// parent map for a decently complex humanoid rig (based on mixamo rig)
const HUMANOID_RIG: [usize; 67] = [
    // (0: root)
    0,  // 1: hips
    1,  // 2: spine
    2,  // 3: spine 1
    3,  // 4: spine 2
    4,  // 5: neck
    5,  // 6: head
    6,  // 7: head top
    6,  // 8: left eye
    6,  // 9: right eye
    4,  // 10: left shoulder
    10, // 11: left arm
    11, // 12: left forearm
    12, // 13: left hand
    13, // 14: left hand thumb 1
    14, // 15: left hand thumb 2
    15, // 16: left hand thumb 3
    16, // 17: left hand thumb 4
    13, // 18: left hand index 1
    18, // 19: left hand index 2
    19, // 20: left hand index 3
    20, // 21: left hand index 4
    13, // 22: left hand middle 1
    22, // 23: left hand middle 2
    23, // 24: left hand middle 3
    24, // 25: left hand middle 4
    13, // 26: left hand ring 1
    26, // 27: left hand ring 2
    27, // 28: left hand ring 3
    28, // 29: left hand ring 4
    13, // 30: left hand pinky 1
    30, // 31: left hand pinky 2
    31, // 32: left hand pinky 3
    32, // 33: left hand pinky 4
    4,  // 34: right shoulder
    34, // 35: right arm
    35, // 36: right forearm
    36, // 37: right hand
    37, // 38: right hand thumb 1
    38, // 39: right hand thumb 2
    39, // 40: right hand thumb 3
    40, // 41: right hand thumb 4
    37, // 42: right hand index 1
    42, // 43: right hand index 2
    43, // 44: right hand index 3
    44, // 45: right hand index 4
    37, // 46: right hand middle 1
    46, // 47: right hand middle 2
    47, // 48: right hand middle 3
    48, // 49: right hand middle 4
    37, // 50: right hand ring 1
    50, // 51: right hand ring 2
    51, // 52: right hand ring 3
    52, // 53: right hand ring 4
    37, // 54: right hand pinky 1
    54, // 55: right hand pinky 2
    55, // 56: right hand pinky 3
    56, // 57: right hand pinky 4
    1,  // 58: left upper leg
    58, // 59: left leg
    59, // 60: left foot
    60, // 61: left toe base
    61, // 62: left toe end
    1,  // 63: right upper leg
    63, // 64: right leg
    64, // 65: right foot
    65, // 66: right toe base
    66, // 67: right toe end
];
