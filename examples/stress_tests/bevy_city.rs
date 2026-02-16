//! A procedurally generated city

// TODO force reload failed assets

use assets::{load_assets, CityAssets};
use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::Exposure,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin, FreeCameraState},
    feathers::{
        self,
        controls::{button, checkbox, ButtonProps},
        dark_theme::create_dark_theme,
        theme::{ThemeBackgroundColor, ThemedText, UiTheme},
        FeathersPlugins,
    },
    light::{atmosphere::ScatteringMedium, Atmosphere, AtmosphereEnvironmentMapLight},
    pbr::AtmosphereSettings,
    post_process::bloom::Bloom,
    prelude::*,
    ui::Checked,
    ui_widgets::{checkbox_self_update, observe, Activate, ValueChange},
};
use noise::{NoiseFn, OpenSimplex};
use rand::{rngs::SmallRng, seq::SliceRandom, Rng, SeedableRng};

#[path = "bevy_city/assets.rs"]
mod assets;

#[derive(Resource)]
struct Settings {
    simulate_cars: bool,
    shadow_maps_enabled: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for Settings {
    fn default() -> Self {
        Self {
            simulate_cars: true,
            shadow_maps_enabled: true,
        }
    }
}

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
            FeathersPlugins,
        ))
        .init_resource::<Settings>()
        .insert_resource(UiTheme(create_dark_theme()))
        .add_systems(Startup, (setup, load_assets, setup_city.after(load_assets)))
        .add_systems(Update, simulate_cars)
        .run();
}

fn setup(mut commands: Commands, mut scattering_mediums: ResMut<Assets<ScatteringMedium>>) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(15.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
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
            shadow_maps_enabled: Settings::default().shadow_maps_enabled,
            illuminance: light_consts::lux::RAW_SUNLIGHT,
            ..default()
        },
        Transform::from_xyz(1.0, 0.15, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(10.0),
            right: Val::Px(10.0),
            padding: UiRect::all(Val::Px(8.0)),
            ..default()
        },
        ThemeBackgroundColor(feathers::tokens::WINDOW_BG),
        observe(
            |_: On<Pointer<Over>>, mut free_camera_state: Single<&mut FreeCameraState>| {
                free_camera_state.enabled = false;
            },
        ),
        observe(
            |_: On<Pointer<Out>>, mut free_camera_state: Single<&mut FreeCameraState>| {
                free_camera_state.enabled = true;
            },
        ),
        children![(
            Node {
                display: Display::Flex,
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Stretch,
                justify_content: JustifyContent::Start,
                row_gap: px(8),
                ..default()
            },
            children![
                (Text("Settings".to_owned())),
                (
                    checkbox(Checked, Spawn((Text::new("Simulate Cars"), ThemedText))),
                    observe(checkbox_self_update),
                    observe(
                        |change: On<ValueChange<bool>>, mut settings: ResMut<Settings>| {
                            settings.simulate_cars = change.value;
                        }
                    )
                ),
                (
                    checkbox(
                        Checked,
                        Spawn((Text::new("Shadow maps enabled"), ThemedText))
                    ),
                    observe(checkbox_self_update),
                    observe(
                        |change: On<ValueChange<bool>>,
                         mut settings: ResMut<Settings>,
                         mut directional_lights: Query<&mut DirectionalLight>| {
                            settings.shadow_maps_enabled = change.value;
                            for mut light in &mut directional_lights {
                                light.shadow_maps_enabled = change.value;

                            }
                        }
                    )
                ),
                (
                    button(
                        ButtonProps::default(),
                        (),
                        Spawn((Text::new("Reload"), ThemedText))
                    ),
                    observe(
                        |_activate: On<Activate>,
                         mut commands: Commands,
                         city_root: Single<Entity, With<CityRoot>>,
                         assets: Res<CityAssets>| {
                            commands.entity(*city_root).despawn();

                            let mut rng = rand::rng();
                            let seed = rng.random::<u64>();
                            println!("new seed: {seed}");
                            spawn_city(&mut commands, &assets, seed, 32);
                        }
                    )
                ),
            ]
        )],
    ));
}

#[derive(Component)]
struct Car {
    start: Vec3,
    end: Vec3,
    distance_traveled: f32,
}

fn simulate_cars(
    settings: Res<Settings>,
    mut cars: Query<(&mut Car, &mut Transform)>,
    time: Res<Time>,
) {
    if !settings.simulate_cars {
        return;
    }

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

#[derive(Component)]
struct CityRoot;

fn setup_city(mut commands: Commands, assets: Res<CityAssets>) {
    spawn_city(&mut commands, &assets, 42, 32);
}

fn spawn_city(commands: &mut Commands, assets: &CityAssets, seed: u64, size: u32) {
    let mut rng = SmallRng::seed_from_u64(seed);
    let noise = OpenSimplex::new(rng.random());
    let noise_scale = 0.025;

    commands
        .spawn((CityRoot, Transform::default(), Visibility::default()))
        .with_children(|commands| {
            let half_size = size as i32 / 2;
            for x in -half_size..half_size {
                for z in -half_size..half_size {
                    let x = x as f32 * 5.5;
                    let z = z as f32 * 4.0;
                    let offset = Vec3::new(x, 0.0, z);

                    spawn_roads_and_cars(commands, &assets, &mut rng, offset);

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
                        // TODO spawn a bunch of trees and rocks
                    } else if density < low_density {
                        spawn_low_density(commands, &assets, &mut rng, offset);
                    } else if density < medium_density {
                        spawn_medium_density(commands, &assets, &mut rng, offset);
                    } else {
                        spawn_high_density(commands, &assets, &mut rng, offset);
                    }
                }
            }
        });
}

fn spawn_roads_and_cars<R: Rng>(
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
                SceneRoot(assets.get_random_car(rng)),
                Transform::from_translation(Vec3::new(0.75 + i as f32 * 0.5, 0.0, 0.15) + offset)
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
                SceneRoot(assets.get_random_car(rng)),
                Transform::from_translation(Vec3::new(0.75 + i as f32 * 0.5, 0.0, -0.15) + offset)
                    .with_scale(Vec3::splat(0.15))
                    .with_rotation(Quat::from_axis_angle(Vec3::Y, -std::f32::consts::FRAC_PI_2)),
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
                SceneRoot(assets.get_random_car(rng)),
                Transform::from_translation(Vec3::new(-0.15, 0.0, 0.75 + i as f32 * 0.5) + offset)
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
                SceneRoot(assets.get_random_car(rng)),
                Transform::from_translation(Vec3::new(0.15, 0.0, 0.75 + i as f32 * 0.5) + offset)
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

fn spawn_low_density<R: Rng>(
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

fn spawn_medium_density<R: Rng>(
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
}

fn spawn_high_density<R: Rng>(
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
