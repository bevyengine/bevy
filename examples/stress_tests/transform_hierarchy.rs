//! Hierarchy and transform propagation stress test.
//!
//! Running this example:
//!
//! ```
//! cargo r --release --example transform_hierarchy <configuration name>
//! ```
//!
//! | Configuration        | Description                                                       |
//! | -------------------- | ----------------------------------------------------------------- |
//! | `large_tree`         | A fairly wide and deep tree.                                      |
//! | `wide_tree`          | A shallow but very wide tree.                                     |
//! | `deep_tree`          | A deep but not very wide tree.                                    |
//! | `chain`              | A chain. 2500 levels deep.                                        |
//! | `update_leaves`      | Same as `large_tree`, but only leaves are updated.                |
//! | `update_shallow`     | Same as `large_tree`, but only the first few levels are updated.  |
//! | `humanoids_active`   | 4000 active humanoid rigs.                                        |
//! | `humanoids_inactive` | 4000 humanoid rigs. Only 10 are active.                           |
//! | `humanoids_mixed`    | 2000 active and 2000 inactive humanoid rigs.                      |

use bevy::prelude::*;
use rand::Rng;

/// pre-defined test configurations with name
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

fn print_available_configs() {
    println!("available configurations:");
    for (name, _) in CONFIGS {
        println!("  {name}");
    }
}

fn main() {
    // parse cli argument and find the selected test configuration
    let cfg: Cfg = match std::env::args().nth(1) {
        Some(arg) => match CONFIGS.iter().find(|(name, _)| *name == arg) {
            Some((name, cfg)) => {
                println!("test configuration: {name}");
                cfg.clone()
            }
            None => {
                println!("test configuration \"{arg}\" not found.\n");
                print_available_configs();
                return;
            }
        },
        None => {
            println!("missing argument: <test configuration>\n");
            print_available_configs();
            return;
        }
    };

    println!("\n{cfg:#?}");

    App::new()
        .insert_resource(cfg)
        .add_plugins((MinimalPlugins, TransformPlugin))
        .add_systems(Startup, setup)
        // Updating transforms *must* be done before `PostUpdate`
        // or the hierarchy will momentarily be in an invalid state.
        .add_systems(Update, update)
        .run();
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

/// set translation based on the angle `a`
fn set_translation(translation: &mut Vec3, a: f32) {
    translation.x = a.cos() * 32.0;
    translation.y = a.sin() * 32.0;
}

fn setup(mut commands: Commands, cfg: Res<Cfg>) {
    warn!(include_str!("warning_string.txt"));

    let mut cam = Camera2dBundle::default();

    cam.transform.translation.z = 100.0;
    commands.spawn(cam);

    let result = match cfg.test_case {
        TestCase::Tree {
            depth,
            branch_width,
        } => {
            let tree = gen_tree(depth, branch_width);
            spawn_tree(&tree, &mut commands, &cfg.update_filter, default())
        }
        TestCase::NonUniformTree {
            depth,
            branch_width,
        } => {
            let tree = gen_non_uniform_tree(depth, branch_width);
            spawn_tree(&tree, &mut commands, &cfg.update_filter, default())
        }
        TestCase::Humanoids { active, inactive } => {
            let mut result = InsertResult::default();
            let mut rng = rand::thread_rng();

            for _ in 0..active {
                result.combine(spawn_tree(
                    &HUMANOID_RIG,
                    &mut commands,
                    &cfg.update_filter,
                    Transform::from_xyz(
                        rng.gen::<f32>() * 500.0 - 250.0,
                        rng.gen::<f32>() * 500.0 - 250.0,
                        0.0,
                    ),
                ));
            }

            for _ in 0..inactive {
                result.combine(spawn_tree(
                    &HUMANOID_RIG,
                    &mut commands,
                    &UpdateFilter {
                        // force inactive by setting the probability < 0
                        probability: -1.0,
                        ..cfg.update_filter
                    },
                    Transform::from_xyz(
                        rng.gen::<f32>() * 500.0 - 250.0,
                        rng.gen::<f32>() * 500.0 - 250.0,
                        0.0,
                    ),
                ));
            }

            result
        }
    };

    println!("\n{result:#?}");
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
    commands: &mut Commands,
    update_filter: &UpdateFilter,
    root_transform: Transform,
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
    ents.push(commands.spawn(TransformBundle::from(root_transform)).id());

    let mut result = InsertResult::default();
    let mut rng = rand::thread_rng();
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
            let mut cmd = commands.spawn_empty();

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

        commands
            .get_or_spawn(ents[parent_idx])
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
