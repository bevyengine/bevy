use bevy::prelude::*;
use noise::{NoiseFn, OpenSimplex};
use rand::{rngs::SmallRng, RngExt, SeedableRng};

use crate::{assets::CityAssets, Car, Road};

#[derive(Component)]
pub struct CityRoot;

#[derive(Default)]
pub struct CityStats {
    pub buildings: u32,
    pub trees: u32,
    pub cars: u32,
    pub fences: u32,
    pub paths: u32,
    pub roads: u32,
    pub ground_tile: u32,
}

/// Spawns a grid of city blocks
///
/// For simplicity we spawn the roads and buildings in this pattern
///
/// X-------
/// | B B B
/// | B B B
///
/// X = crossroad, B = buildings
///
/// This way we can easily tile each city block
/// Each city block is 5.5 units x 4.0 units.
///
/// Every asset gets spawned relative to the crossroad position
pub fn spawn_city(
    commands: &mut Commands,
    assets: &CityAssets,
    seed: u64,
    size: u32,
    car_density: f32,
    stats: &mut CityStats,
) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let noise = OpenSimplex::new(rng.random());
    let noise_scale = 0.025;

    commands
        .spawn((CityRoot, Transform::default(), Visibility::default()))
        .with_children(|commands| {
            let half_size = size as i32 / 2;
            for x in -half_size..half_size {
                for z in -half_size..half_size {
                    // scale the position to match the city block size
                    let x = x as f32 * 5.5;
                    let z = z as f32 * 4.0;
                    let offset = Vec3::new(x, 0.0, z);

                    spawn_roads_and_cars(commands, assets, &mut rng, offset, car_density, stats);

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
                        spawn_forest(commands, assets, &mut rng, offset, stats);
                    } else if density < low_density {
                        spawn_low_density(commands, assets, &mut rng, offset, stats);
                    } else if density < medium_density {
                        spawn_medium_density(commands, assets, &mut rng, offset, stats);
                    } else {
                        spawn_high_density(commands, assets, &mut rng, offset, stats);
                    }
                }
            }
        });
}

fn spawn_roads_and_cars<R: RngExt>(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    rng: &mut R,
    offset: Vec3,
    car_density: f32,
    stats: &mut CityStats,
) {
    let x = offset.x;
    let z = offset.z;

    commands.spawn((
        SceneRoot(assets.crossroad.clone()),
        Transform::from_xyz(x, 0.0, z),
    ));
    stats.roads += 1;

    let max_car_density = car_density;

    // When spawning roads we rotate and stretch a single road asset instead of spawning multiple
    // road segments

    // NOTE most of the magic numbers were hand tweaked for something that looks visually nice

    // horizontal road
    let car_count = 9;
    commands
        .spawn((
            Transform::from_translation(offset),
            Visibility::default(),
            Road {
                start: Vec3::new(0.75, 0.0, 0.0),
                end: Vec3::new(0.75 + (0.5 * car_count as f32), 0.0, 0.0),
            },
        ))
        .with_children(|commands| {
            commands.spawn((
                SceneRoot(assets.road_straight.clone()),
                Transform::from_translation(Vec3::new(2.75, 0.0, 0.0))
                    .with_scale(Vec3::new(4.5, 1.0, 1.0)),
            ));
            stats.roads += 1;

            for i in 0..car_count {
                let car_pos = Vec3::new(0.0, 0.0, 0.75 + i as f32 * 0.5);

                if rng.random::<f32>() < max_car_density {
                    assets.spawn_car(
                        commands,
                        rng,
                        Transform::from_translation(car_pos + Vec3::new(0.0, 0.0, -0.15))
                            .with_scale(Vec3::splat(0.15))
                            .with_rotation(Quat::from_axis_angle(
                                Vec3::Y,
                                3.0 * std::f32::consts::FRAC_PI_2,
                            )),
                        Car {
                            distance_traveled: i as f32 * 0.5,
                            dir: -1.0,
                            offset: Vec3::new(4.25, 0.0, -0.15),
                        },
                    );
                    stats.cars += 1;
                }

                if rng.random::<f32>() < max_car_density {
                    assets.spawn_car(
                        commands,
                        rng,
                        Transform::from_translation(car_pos + Vec3::new(0.0, 0.0, 0.15))
                            .with_scale(Vec3::splat(0.15))
                            .with_rotation(Quat::from_axis_angle(
                                Vec3::Y,
                                std::f32::consts::FRAC_PI_2,
                            )),
                        Car {
                            distance_traveled: i as f32 * 0.5,
                            dir: 1.0,
                            offset: Vec3::new(-0.25, 0.0, 0.15),
                        },
                    );
                    stats.cars += 1;
                }
            }
        });

    // vertical road
    let car_count = 6;
    commands
        .spawn((
            Transform::from_translation(offset),
            Visibility::default(),
            Road {
                start: Vec3::new(0.0, 0.0, 0.75),
                end: Vec3::new(0.0, 0.0, 0.75 + (0.5 * car_count as f32)),
            },
        ))
        .with_children(|commands| {
            commands.spawn((
                SceneRoot(assets.road_straight.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.0, 2.0))
                    .with_scale(Vec3::new(3.0, 1.0, 1.0))
                    .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)),
            ));
            stats.roads += 1;

            for i in 0..car_count {
                let car_pos = Vec3::new(0.0, 0.0, 0.75 + i as f32 * 0.5);

                if rng.random::<f32>() < max_car_density {
                    assets.spawn_car(
                        commands,
                        rng,
                        Transform::from_translation(car_pos + Vec3::new(0.15, 0.0, 0.0))
                            .with_scale(Vec3::splat(0.15)),
                        Car {
                            distance_traveled: i as f32 * 0.5,
                            dir: 1.0,
                            offset: Vec3::new(-0.15, 0.0, -0.25),
                        },
                    );
                    stats.cars += 1;
                }

                if rng.random::<f32>() < max_car_density {
                    assets.spawn_car(
                        commands,
                        rng,
                        Transform::from_translation(car_pos + Vec3::new(-0.15, 0.0, 0.0))
                            .with_scale(Vec3::splat(0.15))
                            .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
                        Car {
                            distance_traveled: i as f32 * 0.5,
                            dir: -1.0,
                            offset: Vec3::new(0.15, 0.0, 2.75),
                        },
                    );
                    stats.cars += 1;
                }
            }
        });
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
            SceneRoot(assets.fence.clone()),
            Transform::from_translation(Vec3::new(2.75, 0.0, 0.75 + i as f32 * 0.4) + offset)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)),
            assets.visibility_ranges[0].clone(),
        ));
        stats.fences += 6;
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
            assets.spawn_tree_large(
                commands,
                Transform::from_translation(
                    Vec3::new(tree_x + x as f32 * x_factor, 0.0, 1.75) + offset,
                ),
            );
            assets.spawn_tree_large(
                commands,
                Transform::from_translation(
                    Vec3::new(tree_x + x as f32 * x_factor, 0.0, 2.25) + offset,
                ),
            );
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
            SceneRoot(assets.path_stones_long.clone()),
            Transform::from_translation(Vec3::new(0.75 + (x as f32 * 0.4), 0.02, 2.0) + offset)
                .with_scale(Vec3::new(1.0, 2.0, 1.0))
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)),
            assets.visibility_ranges[0].clone(),
        ));
        stats.paths += 1;
        commands.spawn((
            SceneRoot(assets.fence.clone()),
            Transform::from_translation(Vec3::new(0.75 + (x as f32 * 0.4), 0.02, 1.85) + offset),
            assets.visibility_ranges[0].clone(),
        ));
        commands.spawn((
            SceneRoot(assets.fence.clone()),
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
