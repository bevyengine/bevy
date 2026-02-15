//! A procedurally generated city

// TODO force reload failed assets

use assets::{load_assets, CityAssets};
use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::Exposure,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    light::{atmosphere::ScatteringMedium, Atmosphere, AtmosphereEnvironmentMapLight},
    pbr::AtmosphereSettings,
    post_process::bloom::Bloom,
    prelude::*,
};
use rand::{rngs::SmallRng, seq::SliceRandom, Rng, SeedableRng};

#[path = "bevy_city/assets.rs"]
mod assets;

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "bevy_city".into(),
                    resolution: (1920, 1080).into(),
                    present_mode: bevy::window::PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FreeCameraPlugin,
        ))
        .add_systems(Startup, (setup, load_assets, setup_city.after(load_assets)))
        .add_systems(Update, simulate_cars)
        .run();
}

fn setup(mut commands: Commands, mut scattering_mediums: ResMut<Assets<ScatteringMedium>>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        FreeCamera::default(),
        Atmosphere::earthlike(scattering_mediums.add(ScatteringMedium::default())),
        AtmosphereSettings::default(),
        // The directional light illuminance used in this scene is
        // quite bright, so raising the exposure compensation helps
        // bring the scene to a nicer brightness range.
        Exposure { ev100: 13.0 },
        // Bloom gives the sun a much more natural look.
        Bloom::NATURAL,
        // Enables the atmosphere to drive reflections and ambient lighting (IBL) for this view
        AtmosphereEnvironmentMapLight::default(),
        Msaa::Off,
        TemporalAntiAliasing::default(),
    ));
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            illuminance: light_consts::lux::RAW_SUNLIGHT,
            ..default()
        },
        Transform::from_xyz(1.0, 0.15, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[derive(Component)]
struct Car {
    start: Vec3,
    end: Vec3,
    distance_traveled: f32,
}

fn simulate_cars(mut cars: Query<(&mut Car, &mut Transform)>, time: Res<Time>) {
    let speed = 2.0;
    for (mut car, mut transform) in &mut cars {
        car.distance_traveled += speed * time.delta_secs();

        let road_len = (car.end - car.start).length();

        if car.distance_traveled > road_len {
            car.distance_traveled = 0.0;
        }
        let direction = (car.end - car.start).normalize();

        let progress = car.distance_traveled / road_len;
        transform.translation = car.start + direction * road_len * progress;
    }
}

fn setup_city(mut commands: Commands, assets: Res<CityAssets>) {
    let mut rng = SmallRng::seed_from_u64(42);
    let size = 32;
    let half_size = size / 2;
    for x in -half_size..half_size {
        for z in -half_size..half_size {
            let x = x as f32 * 5.5;
            let z = z as f32 * 4.0;
            let offset = Vec3::new(x, 0.0, z);

            // spawn roads
            {
                commands.spawn((
                    SceneRoot(assets.crossroad.clone()),
                    Transform::from_xyz(x, 0.0, z),
                ));

                // horizontal road
                commands.spawn((
                    SceneRoot(assets.road_straight.clone()),
                    Transform::from_translation(Vec3::new(2.75, 0.0, 0.0) + offset)
                        .with_scale(Vec3::new(4.5, 1.0, 1.0)),
                ));

                let car_density = 0.75;
                for i in 0..9 {
                    if rng.random::<f32>() > car_density {
                        commands.spawn((
                            SceneRoot(assets.get_random_car(&mut rng)),
                            Transform::from_translation(
                                Vec3::new(0.75 + i as f32 * 0.5, 0.0, 0.15) + offset,
                            )
                            .with_scale(Vec3::splat(0.15))
                            .with_rotation(Quat::from_axis_angle(
                                Vec3::Y,
                                3.0 * -std::f32::consts::FRAC_PI_2,
                            )),
                            Car {
                                start: Vec3::new(0.3, 0.0, 0.15) + offset,
                                end: Vec3::new(5.2, 0.0, 0.15) + offset,
                                distance_traveled: i as f32 * 0.55,
                            },
                        ));
                    }
                    if rng.random::<f32>() > car_density {
                        commands.spawn((
                            SceneRoot(assets.get_random_car(&mut rng)),
                            Transform::from_translation(
                                Vec3::new(0.75 + i as f32 * 0.5, 0.0, -0.15) + offset,
                            )
                            .with_scale(Vec3::splat(0.15))
                            .with_rotation(Quat::from_axis_angle(
                                Vec3::Y,
                                -std::f32::consts::FRAC_PI_2,
                            )),
                            Car {
                                start: Vec3::new(5.2, 0.0, -0.15) + offset,
                                end: Vec3::new(0.3, 0.0, -0.15) + offset,
                                distance_traveled: i as f32 * 0.55,
                            },
                        ));
                    }
                }

                // vertical road
                commands.spawn((
                    SceneRoot(assets.road_straight.clone()),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 2.0) + offset)
                        .with_scale(Vec3::new(3.0, 1.0, 1.0))
                        .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::FRAC_PI_2)),
                ));
                for i in 0..6 {
                    if rng.random::<f32>() > car_density {
                        commands.spawn((
                            SceneRoot(assets.get_random_car(&mut rng)),
                            Transform::from_translation(
                                Vec3::new(-0.15, 0.0, 0.75 + i as f32 * 0.5) + offset,
                            )
                            .with_scale(Vec3::splat(0.15)),
                            Car {
                                start: Vec3::new(-0.15, 0.0, 0.75) + offset,
                                end: Vec3::new(-0.15, 0.0, 3.25) + offset,
                                distance_traveled: i as f32 * 0.5,
                            },
                        ));
                    }
                    if rng.random::<f32>() > car_density {
                        commands.spawn((
                            SceneRoot(assets.get_random_car(&mut rng)),
                            Transform::from_translation(
                                Vec3::new(0.15, 0.0, 0.75 + i as f32 * 0.5) + offset,
                            )
                            .with_scale(Vec3::splat(0.15))
                            .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
                            Car {
                                start: Vec3::new(0.15, 0.0, 3.25) + offset,
                                end: Vec3::new(0.15, 0.0, 0.75) + offset,
                                distance_traveled: i as f32 * 0.5,
                            },
                        ));
                    }
                }
            }

            let noise = ValueNoise::new(42);
            let density = noise.sample(Vec2::new(x, z) / 20.0) * 0.5 + 0.5;
            let low_density = 0.65;
            let medium_density = 0.9;

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

            if density < 0.35 {
                // forest
            } else if density < low_density {
                for x in 1..=2 {
                    let x_factor = 1.8;
                    commands.spawn((
                        assets.low_density.get_random_building(&mut rng),
                        Transform::from_translation(
                            Vec3::new(x as f32 * x_factor, 0.0, 1.25) + offset,
                        ),
                    ));
                    commands.spawn((
                        assets.low_density.get_random_building(&mut rng),
                        Transform::from_translation(
                            Vec3::new(x as f32 * x_factor, 0.0, 2.75) + offset,
                        )
                        .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
                    ));
                }
                for z in 0..=8 {
                    commands.spawn((
                        SceneRoot(assets.tree_small.clone()),
                        Transform::from_translation(
                            Vec3::new(0.75, 0.0, 0.75 + z as f32 * 0.3) + offset,
                        ),
                    ));
                    commands.spawn((
                        SceneRoot(assets.tree_small.clone()),
                        Transform::from_translation(
                            Vec3::new(4.75, 0.0, 0.75 + z as f32 * 0.3) + offset,
                        ),
                    ));
                }
            } else if density < medium_density {
                let x_factor = 0.9;
                for x in 1..=5 {
                    commands.spawn((
                        assets.medium_density.get_random_building(&mut rng),
                        Transform::from_translation(
                            Vec3::new(x as f32 * x_factor, 0.0, 1.0) + offset,
                        ),
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
                        assets.medium_density.get_random_building(&mut rng),
                        Transform::from_translation(
                            Vec3::new(x as f32 * x_factor, 0.0, 3.0) + offset,
                        )
                        .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
                    ));
                }
            } else {
                for x in 0..3 {
                    let x = x as f32;
                    commands.spawn((
                        assets.high_density.get_random_building(&mut rng),
                        Transform::from_translation(Vec3::new(1.25 + x * 1.5, 0.0, 1.25) + offset),
                    ));
                    commands.spawn((
                        assets.high_density.get_random_building(&mut rng),
                        Transform::from_translation(Vec3::new(1.25 + x * 1.5, 0.0, 2.75) + offset)
                            .with_rotation(Quat::from_axis_angle(Vec3::Y, std::f32::consts::PI)),
                    ));
                }
            }
        }
    }
}

pub(crate) struct ValueNoise {
    values: [f32; 256],
    perm: [u8; 256],
}

impl ValueNoise {
    pub(crate) fn new(seed: u64) -> Self {
        let mut rng = SmallRng::seed_from_u64(seed);
        let mut values = [0.0f32; 256];
        let mut perm = [0u8; 256];

        for v in &mut values {
            *v = rng.random_range(-1.0..=1.0);
        }
        for (i, p) in perm.iter_mut().enumerate() {
            *p = i as u8;
        }
        perm.shuffle(&mut rng);

        ValueNoise { values, perm }
    }

    /// Sample 2-D noise at `pos`.
    /// Range: -1..1
    pub(crate) fn sample(&self, pos: Vec2) -> f32 {
        let cell = pos.floor();
        let frac = pos - cell;

        let ux = frac.x * frac.x * (3.0 - 2.0 * frac.x);
        let uy = frac.y * frac.y * (3.0 - 2.0 * frac.y);

        let g00 = self.grad(cell);
        let g10 = self.grad(cell + Vec2::new(1.0, 0.0));
        let g01 = self.grad(cell + Vec2::new(0.0, 1.0));
        let g11 = self.grad(cell + Vec2::new(1.0, 1.0));

        let lerp = |a, b, t| a + t * (b - a);
        lerp(lerp(g00, g10, ux), lerp(g01, g11, ux), uy)
    }

    fn grad(&self, cell: Vec2) -> f32 {
        let x = cell.x as i32;
        let y = cell.y as i32;
        let idx = self.hash(x, y) as usize;
        self.values[idx & 255]
    }

    fn hash(&self, x: i32, y: i32) -> u8 {
        let h = (x.wrapping_mul(1836311903) ^ y.wrapping_mul(297121507)) as u32;
        self.perm[h as usize & 255]
    }
}
