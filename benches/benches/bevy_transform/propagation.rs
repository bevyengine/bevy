use bevy_ecs::prelude::*;
use bevy_math::{ops, prelude::*};
use bevy_tasks::{ComputeTaskPool, TaskPool};
use bevy_transform::{
    prelude::*,
    systems::{mark_dirty_trees, propagate_parent_transforms, sync_simple_transforms},
};
use criterion::{criterion_group, Criterion};
use rand::RngExt;

#[derive(Debug, Clone)]
struct Cfg {
    test_case: TestCase,
    update_filter: UpdateFilter,
}

#[derive(Debug, Clone)]
enum TestCase {
    Tree { depth: u32, branch_width: u32 },
    NonUniformTree { depth: u32, branch_width: u32 },
    Humanoids { active: u32, inactive: u32 },
}

#[derive(Debug, Clone)]
struct UpdateFilter {
    min_depth: u32,
    max_depth: u32,
    probability: f32,
}

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

#[derive(Component)]
struct UpdateValue(f32);

fn set_translation(translation: &mut Vec3, a: f32) {
    translation.x = ops::cos(a) * 32.0;
    translation.y = ops::sin(a) * 32.0;
}

fn update_transforms(mut query: Query<(&mut Transform, &mut UpdateValue)>) {
    for (mut t, mut u) in &mut query {
        u.0 += 0.016 * 0.1;
        set_translation(&mut t.translation, u.0);
    }
}

fn gen_tree(depth: u32, branch_width: u32) -> Vec<usize> {
    let mut count: usize = 0;
    for i in 0..(depth - 1) {
        count += TryInto::<usize>::try_into(branch_width.pow(i)).unwrap();
    }
    (0..count)
        .flat_map(|i| core::iter::repeat_n(i, branch_width.try_into().unwrap()))
        .collect()
}

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

fn gen_non_uniform_tree(max_depth: u32, max_branch_width: u32) -> Vec<usize> {
    let mut tree = Vec::new();
    add_children_non_uniform(&mut tree, 0, max_depth, max_branch_width);
    tree
}

const HUMANOID_RIG: [usize; 67] = [
    0, 1, 2, 3, 4, 5, 6, 6, 6, 4, 10, 11, 12, 13, 14, 15, 16, 13, 18, 19, 20, 13, 22, 23, 24,
    13, 26, 27, 28, 13, 30, 31, 32, 4, 34, 35, 36, 37, 38, 39, 40, 37, 42, 43, 44, 37, 46, 47,
    48, 37, 50, 51, 52, 37, 54, 55, 56, 1, 58, 59, 60, 61, 1, 63, 64, 65, 66,
];

fn spawn_tree(
    parent_map: &[usize],
    world: &mut World,
    update_filter: &UpdateFilter,
    root_transform: Transform,
) {
    let count = parent_map.len() + 1;

    #[derive(Default, Clone, Copy)]
    struct NodeInfo {
        child_count: u32,
        depth: u32,
    }

    let mut ents: Vec<Entity> = Vec::with_capacity(count);
    let mut node_info: Vec<NodeInfo> = vec![NodeInfo::default(); count];
    for (i, &parent_idx) in parent_map.iter().enumerate() {
        assert!(parent_idx <= i, "invalid spawn order");
        node_info[parent_idx].child_count += 1;
    }

    ents.push(world.spawn(root_transform).id());

    let mut rng = rand::rng();
    let mut child_idx: Vec<u16> = vec![0; count];

    for (current_idx, &parent_idx) in parent_map.iter().enumerate() {
        let current_idx = current_idx + 1;

        let sep = child_idx[parent_idx] as f32 / node_info[parent_idx].child_count as f32;
        child_idx[parent_idx] += 1;

        let depth = node_info[parent_idx].depth + 1;
        node_info[current_idx].depth = depth;

        let update = (rng.random::<f32>() <= update_filter.probability)
            && (depth >= update_filter.min_depth && depth <= update_filter.max_depth);

        let transform = {
            let mut translation = Vec3::ZERO;
            set_translation(&mut translation, sep);
            Transform::from_translation(translation)
        };

        let child_entity = if update {
            world.spawn((transform, UpdateValue(sep))).id()
        } else {
            world.spawn(transform).id()
        };

        world.entity_mut(ents[parent_idx]).add_child(child_entity);
        ents.push(child_entity);
    }
}

fn setup_world(cfg: &Cfg) -> World {
    ComputeTaskPool::get_or_init(TaskPool::default);

    let mut world = World::new();
    world.init_resource::<StaticTransformOptimizations>();

    match &cfg.test_case {
        TestCase::Tree {
            depth,
            branch_width,
        } => {
            let tree = gen_tree(*depth, *branch_width);
            spawn_tree(&tree, &mut world, &cfg.update_filter, Transform::default());
        }
        TestCase::NonUniformTree {
            depth,
            branch_width,
        } => {
            let tree = gen_non_uniform_tree(*depth, *branch_width);
            spawn_tree(&tree, &mut world, &cfg.update_filter, Transform::default());
        }
        TestCase::Humanoids { active, inactive } => {
            let mut rng = rand::rng();
            for _ in 0..*active {
                spawn_tree(
                    &HUMANOID_RIG,
                    &mut world,
                    &cfg.update_filter,
                    Transform::from_xyz(
                        rng.random::<f32>() * 500.0 - 250.0,
                        rng.random::<f32>() * 500.0 - 250.0,
                        0.0,
                    ),
                );
            }
            let inactive_filter = UpdateFilter {
                probability: -1.0,
                ..cfg.update_filter.clone()
            };
            for _ in 0..*inactive {
                spawn_tree(
                    &HUMANOID_RIG,
                    &mut world,
                    &inactive_filter,
                    Transform::from_xyz(
                        rng.random::<f32>() * 500.0 - 250.0,
                        rng.random::<f32>() * 500.0 - 250.0,
                        0.0,
                    ),
                );
            }
        }
    }

    world
}

fn create_schedule() -> Schedule {
    let mut schedule = Schedule::default();
    schedule.add_systems(
        (
            update_transforms,
            mark_dirty_trees,
            propagate_parent_transforms,
            sync_simple_transforms,
        )
            .chain(),
    );
    schedule
}

pub fn transform_propagation(criterion: &mut Criterion) {
    let mut group = criterion.benchmark_group("transform_propagation");
    group.warm_up_time(core::time::Duration::from_millis(500));
    group.measurement_time(core::time::Duration::from_secs(5));

    for (name, cfg) in &CONFIGS {
        let cfg = cfg.clone();
        group.bench_function(*name, |bencher| {
            bencher.iter_batched(
                || {
                    // Fresh world + schedule per sample so thread pool layout variance
                    // becomes within-run variance that criterion can model.
                    let mut world = setup_world(&cfg);
                    let mut schedule = create_schedule();
                    schedule.run(&mut world); // initialize systems
                    (world, schedule)
                },
                |(mut world, mut schedule)| {
                    schedule.run(&mut world);
                },
                criterion::BatchSize::LargeInput,
            );
        });
    }

    group.finish();
}

criterion_group!(benches, transform_propagation);
