//! A procedurally generated city

use argh::FromArgs;
use assets::{load_assets, CityAssets};
use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::{visibility::NoCpuCulling, Exposure, Hdr},
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    color::palettes::css::WHITE,
    feathers::{dark_theme::create_dark_theme, theme::UiTheme, FeathersPlugins},
    light::{atmosphere::ScatteringMedium, Atmosphere, AtmosphereEnvironmentMapLight},
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

use crate::{assets::strip_base_url, settings::Settings};
use crate::{generate_city::spawn_city, settings::setup_settings_ui};

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
        .add_systems(Startup, (setup, load_assets))
        .add_systems(Update, (simulate_cars, loading_screen))
        .add_observer(add_no_cpu_culling)
        .add_observer(add_no_cpu_culling_on_scene_ready)
        .add_observer(on_city_assets_ready)
        .add_observer(setup_settings_ui)
        .run();
}

fn setup(mut commands: Commands, mut scattering_mediums: ResMut<Assets<ScatteringMedium>>) {
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Transform::from_xyz(15.0, 10.0, 20.0).looking_at(Vec3::ZERO, Vec3::Y),
        FreeCamera::default(),
        Atmosphere::earth(scattering_mediums.add(ScatteringMedium::default())),
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
        ContactShadows::default(),
    ));

    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: Settings::default().shadow_maps_enabled,
            contact_shadows_enabled: Settings::default().contact_shadows_enabled,
            illuminance: light_consts::lux::RAW_SUNLIGHT,
            ..default()
        },
        Transform::from_xyz(1.0, 0.15, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    commands.spawn((
        LoadingScreen,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        children![(
            Node {
                position_type: PositionType::Absolute,
                top: Val::Percent(50.0),
                left: Val::Percent(20.0),
                right: Val::Percent(20.0),
                height: Val::Vh(40.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::FlexStart,
                overflow: Overflow::scroll_y(),
                ..default()
            },
            children![
                (
                    LoadingText,
                    Text::new("Loading..."),
                    TextFont {
                        font_size: FontSize::Px(24.0),
                        ..default()
                    },
                ),
                (
                    LoadingPaths,
                    Text::new(""),
                    TextFont {
                        font_size: FontSize::Px(14.0),
                        ..default()
                    },
                ),
            ]
        )],
    ));
}

#[derive(Component)]
struct LoadingScreen;
#[derive(Component)]
struct LoadingText;
#[derive(Component)]
struct LoadingPaths;

#[derive(Event)]
struct CityAssetsReady;

#[derive(Event)]
struct CitySpawned;

fn loading_screen(
    mut commands: Commands,
    assets: Res<CityAssets>,
    asset_server: Res<AssetServer>,
    mut loading_text: Query<&mut Text, With<LoadingText>>,
    mut loading_paths: Query<&mut Text, (With<LoadingPaths>, Without<LoadingText>)>,
    loading_screen: Query<Entity, With<LoadingScreen>>,
) {
    let Ok(loading_screen) = loading_screen.single() else {
        return;
    };
    let Ok(mut text) = loading_text.single_mut() else {
        return;
    };
    let Ok(mut paths_text) = loading_paths.single_mut() else {
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
        commands.entity(loading_screen).despawn();
        commands.trigger(CityAssetsReady);
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

fn on_city_assets_ready(
    _: On<CityAssetsReady>,
    mut commands: Commands,
    assets: Res<CityAssets>,
    args: Res<Args>,
) {
    spawn_city(&mut commands, &assets, args.seed, args.size);
    commands.trigger(CitySpawned);
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

fn add_no_cpu_culling(
    _: On<CitySpawned>,
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
