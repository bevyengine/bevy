use bevy::{platform::collections::HashMap, prelude::*};
use noise::{NoiseFn, OpenSimplex};
use rand::{rngs::SmallRng, RngExt, SeedableRng};

use crate::{
    assets::CityAssets,
    simulation::{Car, Intersection, Lane, Road, LANE_WIDTH},
};

#[derive(Component)]
pub struct CityRoot;

#[derive(Component)]
pub struct StaticRoot;

#[derive(Component)]
pub struct CarsRoot;

#[derive(Default)]
pub struct CityStats {
    pub buildings: u32,
    pub trees: u32,
    pub fences: u32,
    pub paths: u32,
    pub roads: u32,
    pub ground_tile: u32,
}

/// Spawns a grid of city blocks and builds a road graph
pub fn spawn_buildings_and_roads(
    commands: &mut Commands,
    assets: &CityAssets,
    seed: u64,
    size: u32,
    stats: &mut CityStats,
) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let noise = OpenSimplex::new(rng.random());
    let half_size = size as i32 / 2;

    commands
        .spawn((CityRoot, Transform::default(), Visibility::default()))
        .with_children(|commands| {
            commands
                .spawn((StaticRoot, Transform::default(), Visibility::default()))
                .with_children(|commands| {
                    spawn_buildings(commands, assets, &mut rng, &noise, half_size, stats);
                    spawn_roads_and_intersections(commands, assets, half_size, stats);
                });
            commands.spawn((CarsRoot, Transform::default(), Visibility::default()));
        });
}

/// Spawns the ground tile and buildings or forest for every city block
fn spawn_buildings(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    rng: &mut SmallRng,
    noise: &OpenSimplex,
    half_size: i32,
    stats: &mut CityStats,
) {
    let noise_scale = 0.025;

    for x in -half_size..half_size {
        for z in -half_size..half_size {
            commands
                .spawn((Transform::default(), Visibility::default()))
                .with_children(|commands| {
                    let offset = Vec3::new(x as f32 * 5.5, 0.0, z as f32 * 4.0);

                    let density = noise.get([
                        offset.x as f64 * noise_scale,
                        offset.z as f64 * noise_scale,
                        0.0,
                    ]) * 0.5
                        + 0.5;

                    let forest = 0.45;
                    let low_density = 0.6;
                    let medium_density = 0.7;
                    let ground_tile_scale = Vec3::new(4.5, 1.0, 3.0);
                    commands.spawn((
                        Mesh3d(assets.ground_tile.0.clone()),
                        if density < low_density {
                            MeshMaterial3d(assets.ground_tile.2.clone())
                        } else {
                            MeshMaterial3d(assets.ground_tile.1.clone())
                        },
                        Transform::from_translation(
                            Vec3::new(0.5, -0.5005, 0.5) + ground_tile_scale / 2.0 + offset,
                        )
                        .with_scale(ground_tile_scale),
                    ));
                    stats.ground_tile += 1;

                    if density < forest {
                        spawn_forest(commands, assets, rng, offset, stats);
                    } else if density < low_density {
                        spawn_low_density(commands, assets, rng, offset, stats);
                    } else if density < medium_density {
                        spawn_medium_density(commands, assets, rng, offset, stats);
                    } else {
                        spawn_high_density(commands, assets, rng, offset, stats);
                    }
                });
        }
    }
}

// Represents an intersection in the road graph
struct Node {
    entity: Entity,
    /// Lanes leaving this intersection
    lanes: Vec<Entity>,
}

fn spawn_roads_and_intersections(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    half_size: i32,
    stats: &mut CityStats,
) {
    // Each node contains a vec of outgoing lanes
    let mut nodes = HashMap::new();
    for x in -half_size..=half_size {
        for z in -half_size..=half_size {
            let entity = commands.spawn_empty().id();
            nodes.insert(
                IVec2::new(x, z),
                Node {
                    entity,
                    lanes: Vec::new(),
                },
            );
        }
    }

    for x in -half_size..=half_size {
        for z in -half_size..=half_size {
            let grid_pos = IVec2::new(x, z);
            let offset = Vec3::new(x as f32 * 5.5, 0.0, z as f32 * 4.0);
            let start_node = nodes[&grid_pos].entity;

            if x < half_size {
                let end_pos = grid_pos + IVec2::new(1, 0);
                let (lane_to_end, lane_to_start) = spawn_horizontal_road(
                    commands,
                    assets,
                    offset,
                    start_node,
                    nodes[&end_pos].entity,
                    stats,
                );
                nodes.get_mut(&grid_pos).unwrap().lanes.push(lane_to_end);
                nodes.get_mut(&end_pos).unwrap().lanes.push(lane_to_start);
            }
            if z < half_size {
                let end_pos = grid_pos + IVec2::new(0, 1);
                let (lane_to_end, lane_to_start) = spawn_vertical_road(
                    commands,
                    assets,
                    offset,
                    start_node,
                    nodes[&end_pos].entity,
                    stats,
                );
                nodes.get_mut(&grid_pos).unwrap().lanes.push(lane_to_end);
                nodes.get_mut(&end_pos).unwrap().lanes.push(lane_to_start);
            }
        }
    }

    for (grid_pos, node) in nodes {
        let offset = Vec3::new(grid_pos.x as f32 * 5.5, 0.0, grid_pos.y as f32 * 4.0);
        commands.commands().entity(node.entity).insert((
            WorldAssetRoot(assets.crossroad.clone()),
            Transform::from_translation(offset),
            Intersection { lanes: node.lanes },
        ));
    }
}

fn spawn_horizontal_road(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    offset: Vec3,
    start_node: Entity,
    end_node: Entity,
    stats: &mut CityStats,
) -> (Entity, Entity) {
    let car_count = 9;
    let start = offset + Vec3::new(0.25, 0.0, 0.0);
    let end = offset + Vec3::new(0.75 + (0.5 * car_count as f32), 0.0, 0.0);

    let mut lanes = None;
    commands
        .spawn((
            Transform::from_translation(offset),
            Visibility::default(),
            Road,
        ))
        .with_children(|commands| {
            commands.spawn((
                WorldAssetRoot(assets.road_straight.clone()),
                Transform::from_translation(Vec3::new(2.75, 0.0, 0.0))
                    .with_scale(Vec3::new(4.5, 1.0, 1.0)),
            ));
            stats.roads += 1;

            let lane_to_end = commands
                .spawn(Lane {
                    start,
                    end,
                    target_node: end_node,
                })
                .id();
            let lane_to_start = commands
                .spawn(Lane {
                    start: end,
                    end: start,
                    target_node: start_node,
                })
                .id();
            lanes = Some((lane_to_end, lane_to_start));
        });
    lanes.unwrap()
}

fn spawn_vertical_road(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    offset: Vec3,
    start_node: Entity,
    end_node: Entity,
    stats: &mut CityStats,
) -> (Entity, Entity) {
    let car_count = 6;
    let start = offset + Vec3::new(0.0, 0.0, 0.25);
    let end = offset + Vec3::new(0.0, 0.0, 0.75 + (0.5 * car_count as f32));

    let mut lanes = None;
    commands
        .spawn((
            Transform::from_translation(offset),
            Visibility::default(),
            Road,
        ))
        .with_children(|commands| {
            commands.spawn((
                WorldAssetRoot(assets.road_straight.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.0, 2.0))
                    .with_scale(Vec3::new(3.0, 1.0, 1.0))
                    .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)),
            ));
            stats.roads += 1;

            let lane_to_end = commands
                .spawn(Lane {
                    start,
                    end,
                    target_node: end_node,
                })
                .id();
            let lane_to_start = commands
                .spawn(Lane {
                    start: end,
                    end: start,
                    target_node: start_node,
                })
                .id();
            lanes = Some((lane_to_end, lane_to_start));
        });
    lanes.unwrap()
}

pub fn spawn_cars(
    commands: &mut Commands,
    assets: &CityAssets,
    cars_root: Entity,
    seed: u64,
    car_count: u32,
    lanes: &Query<(Entity, &Lane)>,
) {
    let lanes = lanes.iter().collect::<Vec<_>>();
    let Some(max_index) = lanes.len().checked_sub(1) else {
        return;
    };

    let mut rng = SmallRng::seed_from_u64(seed);
    for _ in 0..car_count {
        let (lane_entity, lane) = lanes[rng.random_range(0..=max_index)];
        let lane_len = (lane.end - lane.start).length();
        let progress = rng.random_range(0.0..lane_len);

        let direction = (lane.end - lane.start).normalize();
        let lane_offset = direction.cross(Vec3::Y) * LANE_WIDTH;

        assets.spawn_car(
            commands,
            &mut rng,
            Transform::from_translation(
                lane.start.lerp(lane.end, progress / lane_len) + lane_offset,
            )
            .with_scale(Vec3::splat(0.15))
            .looking_at(-direction, Vec3::Y),
            Car {
                lane: lane_entity,
                progress,
            },
            cars_root,
        );
    }
}

fn spawn_low_density<R: RngExt>(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    rng: &mut R,
    offset: Vec3,
    stats: &mut CityStats,
) {
    for x in 1..=2 {
        let x_factor = 1.8;
        assets.spawn_low_density_building(
            commands,
            rng,
            Transform::from_translation(Vec3::new(x as f32 * x_factor, 0.0, 1.25) + offset),
        );
        assets.spawn_low_density_building(
            commands,
            rng,
            Transform::from_translation(Vec3::new(x as f32 * x_factor, 0.0, 2.75) + offset)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
        );
        stats.buildings += 2;
    }
    for i in 0..=6 {
        commands.spawn((
            WorldAssetRoot(assets.fence.clone()),
            Transform::from_translation(Vec3::new(2.75, 0.0, 0.75 + i as f32 * 0.4) + offset)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)),
            assets.visibility_ranges[0].clone(),
        ));
        stats.fences += 1;
    }
    for z in 0..=8 {
        assets.spawn_tree_small(
            commands,
            Transform::from_translation(Vec3::new(0.75, 0.0, 0.75 + z as f32 * 0.3) + offset),
        );
        assets.spawn_tree_small(
            commands,
            Transform::from_translation(Vec3::new(4.75, 0.0, 0.75 + z as f32 * 0.3) + offset),
        );
        stats.trees += 2;
    }
}

fn spawn_medium_density<R: RngExt>(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    rng: &mut R,
    offset: Vec3,
    stats: &mut CityStats,
) {
    let x_factor = 0.9;
    for x in 1..=5 {
        assets.spawn_medium_density_building(
            commands,
            rng,
            Transform::from_translation(Vec3::new(x as f32 * x_factor, 0.0, 1.0) + offset),
        );
        stats.buildings += 1;

        for tree_x in 0..=1 {
            let tree_x = tree_x as f32 * 0.5;
            if x == 5 && tree_x == 0.5 {
                break;
            }
            commands.spawn((
                WorldAssetRoot(assets.tree_large.clone()),
                Transform::from_translation(
                    Vec3::new(tree_x + x as f32 * x_factor, 0.0, 1.75) + offset,
                ),
            ));
            commands.spawn((
                WorldAssetRoot(assets.tree_large.clone()),
                Transform::from_translation(
                    Vec3::new(tree_x + x as f32 * x_factor, 0.0, 2.25) + offset,
                ),
            ));
            stats.trees += 2;
        }

        assets.spawn_medium_density_building(
            commands,
            rng,
            Transform::from_translation(Vec3::new(x as f32 * x_factor, 0.0, 3.0) + offset)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
        );
        stats.buildings += 1;
    }

    for x in 0..=10 {
        commands.spawn((
            WorldAssetRoot(assets.path_stones_long.clone()),
            Transform::from_translation(Vec3::new(0.75 + (x as f32 * 0.4), 0.02, 2.0) + offset)
                .with_scale(Vec3::new(1.0, 2.0, 1.0))
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)),
            assets.visibility_ranges[0].clone(),
        ));
        stats.paths += 1;
        commands.spawn((
            WorldAssetRoot(assets.fence.clone()),
            Transform::from_translation(Vec3::new(0.75 + (x as f32 * 0.4), 0.02, 1.85) + offset),
            assets.visibility_ranges[0].clone(),
        ));
        commands.spawn((
            WorldAssetRoot(assets.fence.clone()),
            Transform::from_translation(Vec3::new(0.75 + (x as f32 * 0.4), 0.02, 2.15) + offset),
            assets.visibility_ranges[0].clone(),
        ));
        stats.fences += 2;
    }
}

fn spawn_high_density<R: RngExt>(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    rng: &mut R,
    offset: Vec3,
    stats: &mut CityStats,
) {
    for x in 0..3 {
        let x = x as f32;
        assets.spawn_high_density_building(
            commands,
            rng,
            Transform::from_translation(Vec3::new(1.25 + x * 1.5, 0.0, 1.25) + offset),
        );
        assets.spawn_high_density_building(
            commands,
            rng,
            Transform::from_translation(Vec3::new(1.25 + x * 1.5, 0.0, 2.75) + offset)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
        );
        stats.buildings += 2;
    }
}

fn spawn_forest<R: RngExt>(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    rng: &mut R,
    offset: Vec3,
    stats: &mut CityStats,
) {
    for x in 0..=12 {
        for z in 0..=8 {
            let transform = Transform::from_translation(
                Vec3::new(x as f32, 0.0, z as f32) * Vec3::new(0.325, 0.0, 0.3)
                    + Vec3::new(0.75, 0.0, 0.85)
                    + offset,
            );

            match rng.random_range(0..3) {
                0 => {}
                1 => {
                    assets.spawn_tree_small(commands, transform);
                    stats.trees += 1;
                }
                2 => {
                    assets.spawn_tree_large(commands, transform);
                    stats.trees += 1;
                }
                _ => {}
            }
        }
    }
}
