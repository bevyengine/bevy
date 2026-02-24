use bevy::prelude::*;
use noise::{NoiseFn, OpenSimplex};
use rand::{rngs::SmallRng, RngExt, SeedableRng};

use crate::{assets::CityAssets, Car, Road};

#[derive(Component)]
pub struct CityRoot;

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
pub fn spawn_city(commands: &mut Commands, assets: &CityAssets, seed: u64, size: u32) {
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

                    spawn_roads_and_cars(commands, assets, &mut rng, offset);

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

                    if density < forest {
                        spawn_forest(commands, assets, &mut rng, offset);
                    } else if density < low_density {
                        spawn_low_density(commands, assets, &mut rng, offset);
                    } else if density < medium_density {
                        spawn_medium_density(commands, assets, &mut rng, offset);
                    } else {
                        spawn_high_density(commands, assets, &mut rng, offset);
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
) {
    let x = offset.x;
    let z = offset.z;

    commands.spawn((
        SceneRoot(assets.crossroad.clone()),
        Transform::from_xyz(x, 0.0, z),
    ));

    let max_car_density = 0.4;

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

            for i in 0..car_count {
                let car_pos = Vec3::new(0.0, 0.0, 0.75 + i as f32 * 0.5);

                if rng.random::<f32>() < max_car_density {
                    commands.spawn((
                        SceneRoot(assets.get_random_car(rng)),
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
                    ));
                }

                if rng.random::<f32>() < max_car_density {
                    commands.spawn((
                        SceneRoot(assets.get_random_car(rng)),
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
                    ));
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

            for i in 0..car_count {
                let car_pos = Vec3::new(0.0, 0.0, 0.75 + i as f32 * 0.5);

                if rng.random::<f32>() < max_car_density {
                    commands.spawn((
                        SceneRoot(assets.get_random_car(rng)),
                        Transform::from_translation(car_pos + Vec3::new(0.15, 0.0, 0.0))
                            .with_scale(Vec3::splat(0.15)),
                        Car {
                            distance_traveled: i as f32 * 0.5,
                            dir: 1.0,
                            offset: Vec3::new(-0.15, 0.0, -0.25),
                        },
                    ));
                }

                if rng.random::<f32>() < max_car_density {
                    commands.spawn((
                        SceneRoot(assets.get_random_car(rng)),
                        Transform::from_translation(car_pos + Vec3::new(-0.15, 0.0, 0.0))
                            .with_scale(Vec3::splat(0.15))
                            .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
                        Car {
                            distance_traveled: i as f32 * 0.5,
                            dir: -1.0,
                            offset: Vec3::new(0.15, 0.0, 2.75),
                        },
                    ));
                }
            }
        });
}

fn spawn_low_density<R: RngExt>(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    rng: &mut R,
    offset: Vec3,
) {
    for x in 1..=2 {
        let x_factor = 1.8;
        commands.spawn((
            assets.low_density.get_random_building(rng),
            Transform::from_translation(Vec3::new(x as f32 * x_factor, 0.0, 1.25) + offset),
        ));
        commands.spawn((
            assets.low_density.get_random_building(rng),
            Transform::from_translation(Vec3::new(x as f32 * x_factor, 0.0, 2.75) + offset)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
        ));
    }
    for i in 0..=6 {
        commands.spawn((
            SceneRoot(assets.fence.clone()),
            Transform::from_translation(Vec3::new(2.75, 0.0, 0.75 + i as f32 * 0.4) + offset)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)),
        ));
    }
    for z in 0..=8 {
        commands.spawn((
            SceneRoot(assets.tree_small.clone()),
            Transform::from_translation(Vec3::new(0.75, 0.0, 0.75 + z as f32 * 0.3) + offset),
        ));
        commands.spawn((
            SceneRoot(assets.tree_small.clone()),
            Transform::from_translation(Vec3::new(4.75, 0.0, 0.75 + z as f32 * 0.3) + offset),
        ));
    }
}

fn spawn_medium_density<R: RngExt>(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    rng: &mut R,
    offset: Vec3,
) {
    let x_factor = 0.9;
    for x in 1..=5 {
        commands.spawn((
            assets.medium_density.get_random_building(rng),
            Transform::from_translation(Vec3::new(x as f32 * x_factor, 0.0, 1.0) + offset),
        ));

        for tree_x in 0..=1 {
            let tree_x = tree_x as f32 * 0.5;
            if x == 5 && tree_x == 0.5 {
                break;
            }
            commands.spawn((
                SceneRoot(assets.tree_large.clone()),
                Transform::from_translation(
                    Vec3::new(tree_x + x as f32 * x_factor, 0.0, 1.75) + offset,
                ),
            ));
            commands.spawn((
                SceneRoot(assets.tree_large.clone()),
                Transform::from_translation(
                    Vec3::new(tree_x + x as f32 * x_factor, 0.0, 2.25) + offset,
                ),
            ));
        }

        commands.spawn((
            assets.medium_density.get_random_building(rng),
            Transform::from_translation(Vec3::new(x as f32 * x_factor, 0.0, 3.0) + offset)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
        ));
    }

    for x in 0..=10 {
        commands.spawn((
            SceneRoot(assets.path_stones_long.clone()),
            Transform::from_translation(Vec3::new(0.75 + (x as f32 * 0.4), 0.02, 2.0) + offset)
                .with_scale(Vec3::new(1.0, 2.0, 1.0))
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)),
        ));
        commands.spawn((
            SceneRoot(assets.fence.clone()),
            Transform::from_translation(Vec3::new(0.75 + (x as f32 * 0.4), 0.02, 1.85) + offset),
        ));
        commands.spawn((
            SceneRoot(assets.fence.clone()),
            Transform::from_translation(Vec3::new(0.75 + (x as f32 * 0.4), 0.02, 2.15) + offset),
        ));
    }
}

fn spawn_high_density<R: RngExt>(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    rng: &mut R,
    offset: Vec3,
) {
    for x in 0..3 {
        let x = x as f32;
        commands.spawn((
            assets.high_density.get_random_building(rng),
            Transform::from_translation(Vec3::new(1.25 + x * 1.5, 0.0, 1.25) + offset),
        ));
        commands.spawn((
            assets.high_density.get_random_building(rng),
            Transform::from_translation(Vec3::new(1.25 + x * 1.5, 0.0, 2.75) + offset)
                .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
        ));
    }
}

fn spawn_forest<R: RngExt>(
    commands: &mut ChildSpawnerCommands,
    assets: &CityAssets,
    rng: &mut R,
    offset: Vec3,
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
                    commands.spawn((SceneRoot(assets.tree_small.clone()), transform));
                }
                2 => {
                    commands.spawn((SceneRoot(assets.tree_large.clone()), transform));
                }
                _ => {}
            }
        }
    }
}
