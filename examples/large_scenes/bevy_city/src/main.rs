//! A procedurally generated city.
//!
//! This scene is intended to be an attractive, fairly realistic stress test of Bevy's capacity
//! to model extremely large scenes.
//! As a result, the complexity is higher than in most examples or benchmarks —
//! we want to use a large number of features so that pathological paths
//! are caught during development, rather than by end users.

use argh::FromArgs;
use assets::{load_assets, CityAssets};
use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::{visibility::NoCpuCulling, Exposure, Hdr},
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    color::palettes::css::WHITE,
    feathers::{dark_theme::create_dark_theme, theme::UiTheme, FeathersPlugins},
    light::{
        atmosphere::{Falloff, PhaseFunction, ScatteringMedium, ScatteringTerm},
        Atmosphere, AtmosphereEnvironmentMapLight,
    },
    pbr::{
        wireframe::{WireframeConfig, WireframePlugin},
        AtmosphereSettings, ContactShadows,
    },
    post_process::bloom::Bloom,
    prelude::*,
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
    world_serialization::WorldInstanceReady,
};

use crate::generate_city::spawn_city;
use crate::{
    assets::{merge_car_meshes, strip_base_url},
    settings::{settings_ui, Settings},
};

mod assets;
mod generate_city;
mod settings;

#[derive(FromArgs, Resource, Clone)]
/// Config
pub struct Args {
    /// seed
    #[argh(option, default = "42")]
    seed: u64,

    /// size
    #[argh(option, default = "30")]
    size: u32,

    /// adds NoCpuCulling to all meshes
    #[argh(switch)]
    no_cpu_culling: bool,
}

fn main() {
    let args: Args = argh::from_env();

    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "bevy_city".into(),
                    resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                    present_mode: PresentMode::AutoNoVsync,
                    position: WindowPosition::Centered(MonitorSelection::Primary),
                    ..default()
                }),
                ..default()
            }),
            FreeCameraPlugin,
            FeathersPlugins,
            WireframePlugin::default(),
        ))
        .insert_resource(args.clone())
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WinitSettings::continuous())
        .init_resource::<Settings>()
        .insert_resource(UiTheme(create_dark_theme()))
        .insert_resource(WireframeConfig {
            global: false,
            default_color: WHITE.into(),
            ..default()
        })
        // Like in many realistic large scenes, many of the objects don't move
        // We can accelerate transform propagation by optimizing for this case
        .insert_resource(StaticTransformOptimizations::Enabled)
        .add_message::<CityAssetsLoaded>()
        .add_message::<CityAssetsReady>()
        .add_message::<CitySpawned>()
        .add_systems(Startup, (scene.spawn(), spawn_atmosphere, load_assets))
        .add_systems(
            Update,
            (
                simulate_cars,
                update_loading_screen,
                process_assets.run_if(on_message::<CityAssetsLoaded>),
                on_city_assets_ready.run_if(on_message::<CityAssetsReady>),
                (add_no_cpu_culling, on_city_spawned, settings_ui.spawn())
                    .run_if(on_message::<CitySpawned>),
            ),
        )
        .add_observer(add_no_cpu_culling_on_scene_ready)
        .run();
}

fn scene() -> impl SceneList {
    bsn_list![camera(), sun(), loading_screen()]
}

fn camera() -> impl Scene {
    bsn! {
        Camera3d
        Hdr
        template_value(Transform::from_xyz(15.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y))
        FreeCamera
        AtmosphereSettings {
            // Reduce the default max distance in the aerial view LUT
            // to 16km to approximately fit the size of the city. This way the aerial perspective
            // gets more detail and has less banding artifacts compared to the 32km default.
            aerial_view_lut_max_distance: 1.6e4,
        }
        // The directional light illuminance used in this scene is
        // quite bright, so raising the exposure compensation helps
        // bring the scene to a nicer brightness range.
        Exposure::OVERCAST
        // Bloom gives the sun a much more natural look.
        Bloom::NATURAL
        // Enables the atmosphere to drive reflections and ambient lighting (IBL) for this view
        AtmosphereEnvironmentMapLight
        Msaa::Off
        TemporalAntiAliasing
        ContactShadows
    }
}

fn loading_screen() -> impl Scene {
    bsn! {
        LoadingScreen
        Node {
            position_type: PositionType::Absolute,
            width: percent(100),
            height: percent(100),
        }
        BackgroundColor(Color::BLACK)
        Children [
            Node {
                position_type: PositionType::Absolute,
                top: percent(50),
                left: percent(20),
                right: percent(20),
                height: vh(40),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                overflow: Overflow::scroll_y(),
            }
            Children [
                (
                    LoadingText
                    Text("Loading...")
                    TextFont {
                        font_size: FontSize::Px(24.0),
                    }
                ),
                (
                    LoadingPaths
                    Text
                    TextFont {
                        font_size: FontSize::Px(14.0),
                    }
                ),
            ]
        ]
    }
}

fn sun() -> impl Scene {
    bsn! {
        DirectionalLight {
            shadow_maps_enabled: {Settings::default().shadow_maps_enabled},
            contact_shadows_enabled: {Settings::default().contact_shadows_enabled},
            illuminance: light_consts::lux::RAW_SUNLIGHT,
        }
        template_value(Transform::from_xyz(1.0, 0.15, 1.0).looking_at(Vec3::ZERO, Vec3::Y))
    }
}

/// Spawns the earth atmosphere plus an extra near-ground fog term.
fn spawn_atmosphere(
    mut commands: Commands,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
) {
    let mut earth_medium = ScatteringMedium::default();

    // Same 60 km atmosphere height as `ScatteringMedium::earth`
    const ATMOSPHERE_REF_HEIGHT_KM: f32 = 60.0;

    // The scale height of haze is set to 100 meters providing a low-lying dense fog layer.
    const HAZE_SCALE_HEIGHT_KM: f32 = 0.1;

    // Fog has high albedo and very low absorption resulting in a white color.
    const HAZE_SINGLE_SCATTER_ALBEDO: f32 = 0.99;

    // Distance at which contrast falls low enough to be indistinguishable from the sky.
    // known as Meteorological Optical Range
    const HAZE_VISIBILITY_KM: f32 = 12.0;

    // Koschmieder relation to calculate the extinction coefficient for the medium in m^-1 units.
    let beta_ext = (3.912 / HAZE_VISIBILITY_KM) * 1e-3;

    // Add the fog to the earth medium as an additional scattering term.
    earth_medium.terms.push(ScatteringTerm {
        absorption: Vec3::splat(beta_ext * (1.0 - HAZE_SINGLE_SCATTER_ALBEDO)),
        scattering: Vec3::splat(beta_ext * HAZE_SINGLE_SCATTER_ALBEDO),
        falloff: Falloff::Exponential {
            scale: HAZE_SCALE_HEIGHT_KM / ATMOSPHERE_REF_HEIGHT_KM,
        },
        // Fog is approximated as a mie scatterer with this asymmetry factor
        phase: PhaseFunction::Mie { asymmetry: 0.76 },
    });
    let earth_atmosphere = Atmosphere::earth(scattering_mediums.add(earth_medium));

    // This scale means that 1 city block in this scene will be roughly 100 meters relative to the atmosphere.
    let scale = 1.0 / 20.0;
    commands.spawn((
        earth_atmosphere.clone(),
        Transform::from_scale(Vec3::splat(scale))
            .with_translation(-Vec3::Y * earth_atmosphere.inner_radius * scale),
    ));
}

#[derive(Component, Default, Clone)]
struct LoadingScreen;
#[derive(Component, Default, Clone)]
struct LoadingText;
#[derive(Component, Default, Clone)]
struct LoadingPaths;

/// Triggers when all the assets managed in [`CityAssets`] are loaded
#[derive(Message)]
struct CityAssetsLoaded;
/// Triggers when all the assets are done loading and have been processed
#[derive(Message)]
struct CityAssetsReady;
/// Triggers once all the city blocks have been spawned
#[derive(Message)]
struct CitySpawned;

#[allow(clippy::type_complexity)]
fn update_loading_screen(
    mut commands: Commands,
    assets: Res<CityAssets>,
    asset_server: Res<AssetServer>,
    mut loading_text: Query<&mut Text, With<LoadingText>>,
    mut loading_paths: Query<(Entity, &mut Text), (With<LoadingPaths>, Without<LoadingText>)>,
) {
    let Ok(mut text) = loading_text.single_mut() else {
        return;
    };
    let Ok((paths_entity, mut paths_text)) = loading_paths.single_mut() else {
        return;
    };
    let mut paths = vec![];
    for untyped in &assets.untyped_assets {
        if let Some(path) = asset_server.get_path(untyped) {
            let state = asset_server.is_loaded_with_dependencies(untyped);
            if !state {
                paths.push(strip_base_url(path.to_string()));
            }
        }
    }
    if paths.is_empty() {
        commands.entity(paths_entity).despawn();
        text.0 = "Processing assets...".into();
        // Use a Message instead of an Event so asset processing only starts on the next frame
        commands.write_message(CityAssetsLoaded);
    } else {
        text.0 = format!(
            "Loading assets: {}/{}",
            assets.untyped_assets.len() - paths.len(),
            assets.untyped_assets.len(),
        );
        paths.reverse();
        paths_text.0 = paths.join("\n");
    }
}

/// Runs after the assets are loaded. For now, this will merge all the meshes for each car gltf into
/// a single mesh. This is necessary because the tires are separate meshes and this increases the
/// amount of meshes bevy has to process every frame for no benefits.
///
/// Eventually, this will also be used for things like generating LODs
fn process_assets(
    mut commands: Commands,
    mut city_assets: ResMut<CityAssets>,
    mut world_assets: ResMut<Assets<WorldAsset>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    merge_car_meshes(&mut city_assets, &mut world_assets, &mut meshes);

    // Use a Message instead of an Event so spawning the city happens in the next frame
    commands.write_message(CityAssetsReady);
}

fn on_city_assets_ready(
    mut commands: Commands,
    city_assets: Res<CityAssets>,
    args: Res<Args>,
    mut loading_text: Query<&mut Text, With<LoadingText>>,
) {
    let Ok(mut text) = loading_text.single_mut() else {
        return;
    };
    text.0 = "Spawning city...".into();

    spawn_city(&mut commands, &city_assets, args.seed, args.size);
    commands.write_message(CitySpawned);
}

fn on_city_spawned(
    mut commands: Commands,
    loading_screen: Option<Single<Entity, With<LoadingScreen>>>,
) {
    let Some(loading_screen) = loading_screen else {
        return;
    };
    commands.entity(*loading_screen).despawn();
}

#[derive(Component)]
struct Road {
    start: Vec3,
    end: Vec3,
}

#[derive(Component)]
struct Car {
    offset: Vec3,
    distance_traveled: f32,
    dir: f32,
}

/// Do a very naive traffic simulation. This will only move the car to the end of the road then
/// spawn it back at the start.
///
/// Eventually this will be a more complex traffic simulation that should stress the ECS
fn simulate_cars(
    settings: Res<Settings>,
    roads: Query<(&Road, &Transform, &Children), Without<Car>>,
    mut cars: Query<(&mut Car, &mut Transform), Without<Road>>,
    time: Res<Time>,
) {
    if !settings.simulate_cars {
        return;
    }
    let speed = 1.5;

    for (road, _, children) in &roads {
        for child in children {
            let Ok((mut car, mut car_transform)) = cars.get_mut(*child) else {
                continue;
            };

            car.distance_traveled += speed * time.delta_secs();
            let road_len = (road.end - road.start).length();
            if car.distance_traveled > road_len {
                car.distance_traveled = 0.0;
            }
            let direction = (road.end - road.start).normalize() * car.dir;

            let progress = car.distance_traveled / road_len;
            car_transform.translation = (road.start + car.offset) + direction * road_len * progress;
        }
    }
}

/// Adds [`NoCpuCulling`] to all meshes in the scene after the city is done spawning
fn add_no_cpu_culling(
    mut commands: Commands,
    meshes: Query<Entity, (With<Mesh3d>, Without<NoCpuCulling>)>,
    args: Res<Args>,
) {
    if args.no_cpu_culling {
        for entity in meshes.iter() {
            commands.entity(entity).insert(NoCpuCulling);
        }
    }
}

/// Adds [`NoCpuCulling`] to all meshes in all scenes after the city is done spawning
///
/// This is required because a few assets are spawned using a [`WorldAssetRoot`] instead of directly
/// spawning a [`Mesh`]
fn add_no_cpu_culling_on_scene_ready(
    scene_ready: On<WorldInstanceReady>,
    mut commands: Commands,
    children: Query<&Children>,
    meshes: Query<(), (With<Mesh3d>, Without<NoCpuCulling>)>,
    args: Res<Args>,
) {
    if args.no_cpu_culling {
        for descendant in children.iter_descendants(scene_ready.entity) {
            if meshes.get(descendant).is_ok() {
                commands.entity(descendant).insert(NoCpuCulling);
            }
        }
    }
}
