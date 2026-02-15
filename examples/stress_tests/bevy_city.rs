//! A procedurally generated city

// TODO force reload failed assets

use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::Exposure,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    color::palettes::css::WHITE,
    light::{atmosphere::ScatteringMedium, Atmosphere, AtmosphereEnvironmentMapLight},
    pbr::AtmosphereSettings,
    post_process::bloom::Bloom,
    prelude::*,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};

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

#[derive(Resource)]
struct CityAssets {
    cars: Vec<Handle<Scene>>,
    crossroad: Handle<Scene>,
    road_straight: Handle<Scene>,
    high_density: Buildings,
    medium_density: Buildings,
    low_density: Buildings,
    ground_tile: (
        Handle<Mesh>,
        Handle<StandardMaterial>,
        Handle<StandardMaterial>,
    ),
    tree_small: Handle<Scene>,
    tree_large: Handle<Scene>,
}

impl CityAssets {
    fn get_random_car<R: Rng>(&self, rng: &mut R) -> Handle<Scene> {
        self.cars[rng.random_range(0..self.cars.len())].clone()
    }
}

struct Buildings {
    meshes: Vec<Handle<Mesh>>,
    materials: Vec<Handle<StandardMaterial>>,
}

impl Buildings {
    fn get_random_building<R: Rng>(
        &self,
        rng: &mut R,
    ) -> (Mesh3d, MeshMaterial3d<StandardMaterial>) {
        let mesh = self.meshes[rng.random_range(0..self.meshes.len())].clone();
        let material = self.materials[rng.random_range(0..self.materials.len())].clone();
        (Mesh3d(mesh), MeshMaterial3d(material))
    }
}

fn load_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let base_url = "https://github.com/bevyengine/bevy_asset_files/raw/main/kenney";

    let cars = {
        // We need to trigger a load of the texture even if we never use it directly
        let _car_texture: Handle<Image> =
            asset_server.load(format!("{base_url}/car-kit/Textures/colormap.png"));

        // TODO generate variations
        [
            "hatchback-sports",
            "suv",
            "suv-luxury",
            "sedan",
            "sedan-sports",
            "truck",
            "truck-flat",
            "van",
        ]
        .iter()
        .map(|t| {
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/car-kit/{t}.glb")))
        })
        .collect::<Vec<_>>()
    };

    // We need to trigger a load of the texture even if we never use it directly
    let _road_texture: Handle<Image> =
        asset_server.load(format!("{base_url}/city-kit-roads/Textures/colormap.png"));

    let crossroad = asset_server.load(
        GltfAssetLabel::Scene(0)
            .from_asset(format!("{base_url}/city-kit-roads/road-crossroad-path.glb")),
    );
    let road_straight = asset_server.load(
        GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/city-kit-roads/road-straight.glb")),
    );

    let high_density = {
        let materials = ["colormap", "variation-a", "variation-b"]
            .iter()
            .map(|variation| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(asset_server.load(format!(
                        "{base_url}/city-kit-commercial/Textures/{variation}.png"
                    ))),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();

        let mut meshes = ["a", "b", "c", "d", "e"]
            .iter()
            .map(|t| {
                asset_server.load(
                    GltfAssetLabel::Primitive {
                        mesh: 0,
                        primitive: 0,
                    }
                    .from_asset(format!(
                        "{base_url}/city-kit-commercial/building-skyscraper-{t}.glb"
                    )),
                )
            })
            .collect::<Vec<_>>();
        meshes.extend(["m", "l"].iter().map(|t| {
            asset_server.load(
                GltfAssetLabel::Primitive {
                    mesh: 0,
                    primitive: 0,
                }
                .from_asset(format!("{base_url}/city-kit-commercial/building-{t}.glb")),
            )
        }));

        Buildings { meshes, materials }
    };

    let medium_density = {
        let materials = ["colormap", "variation-a", "variation-b"]
            .iter()
            .map(|variation| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(asset_server.load(format!(
                        "{base_url}/city-kit-commercial/Textures/{variation}.png"
                    ))),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        let meshes = ["a", "b", "c", "d", "f", "g", "h"]
            .iter()
            .map(|t| {
                asset_server.load(
                    GltfAssetLabel::Primitive {
                        mesh: 0,
                        primitive: 0,
                    }
                    .from_asset(format!("{base_url}/city-kit-commercial/building-{t}.glb")),
                )
            })
            .collect::<Vec<_>>();

        Buildings { meshes, materials }
    };
    let low_density = {
        let materials = ["colormap", "variation-a", "variation-b", "variation-c"]
            .iter()
            .map(|variation| {
                materials.add(StandardMaterial {
                    base_color_texture: Some(asset_server.load(format!(
                        "{base_url}/city-kit-suburban/Textures/{variation}.png"
                    ))),
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        let meshes = ["b", "c", "d", "e", "f", "g", "h", "i", "k", "l", "o", "u"]
            .iter()
            .map(|t| {
                asset_server.load(
                    GltfAssetLabel::Primitive {
                        mesh: 0,
                        primitive: 0,
                    }
                    .from_asset(format!(
                        "{base_url}/city-kit-suburban/building-type-{t}.glb"
                    )),
                )
            })
            .collect::<Vec<_>>();

        Buildings { meshes, materials }
    };

    let ground_tile = {
        let mesh = asset_server.load(
            GltfAssetLabel::Primitive {
                mesh: 0,
                primitive: 0,
            }
            .from_asset(format!("{base_url}/city-kit-roads/tile-low.glb")),
        );
        // TODO use this once https://github.com/bevyengine/bevy/pull/22943 is merged
        // let default_material: Handle<StandardMaterial> = asset_server.load(format!(
        //     "ground_tile/tile-low.glb#{}/std",
        //     GltfAssetLabel::DefaultMaterial
        // ));
        let white_material = materials.add(StandardMaterial::from_color(WHITE));
        let grass_material =
            materials.add(StandardMaterial::from_color(Color::srgb_u8(97, 203, 139)));

        (mesh, white_material, grass_material)
    };

    let tree_small: Handle<Scene> = asset_server.load(
        GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/city-kit-suburban/tree-small.glb")),
    );
    let tree_large: Handle<Scene> = asset_server.load(
        GltfAssetLabel::Scene(0).from_asset(format!("{base_url}/city-kit-suburban/tree-large.glb")),
    );

    commands.insert_resource(CityAssets {
        cars,
        crossroad,
        road_straight,
        high_density,
        medium_density,
        low_density,
        ground_tile,
        tree_small,
        tree_large,
    });
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

            // TODO use noise
            let density = rng.random::<f32>();
            let low_density = 0.6;
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

            if density < low_density {
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
