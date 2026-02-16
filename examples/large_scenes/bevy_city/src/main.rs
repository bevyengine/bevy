//! A procedurally generated city

// TODO force reload failed assets

use assets::{load_assets, CityAssets};
use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::Exposure,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin, FreeCameraState},
    dev_tools::fps_overlay::{FpsOverlayConfig, FpsOverlayPlugin, FrameTimeGraphConfig},
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
    window::{PresentMode, WindowResolution},
    winit::WinitSettings,
};
use rand::RngExt;

use crate::generate_city::{spawn_city, CityRoot};

mod assets;
mod generate_city;

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
                    resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                    present_mode: PresentMode::Immediate,
                    ..default()
                }),
                ..default()
            }),
            FreeCameraPlugin,
            FeathersPlugins,
        ))
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(WinitSettings::continuous())
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
                        Spawn((Text::new("Regenerate City"), ThemedText))
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

fn setup_city(mut commands: Commands, assets: Res<CityAssets>) {
    spawn_city(&mut commands, &assets, 42, 30);
}

#[derive(Component)]
struct Road {
    start: Vec3,
    end: Vec3,
}

#[derive(Component)]
struct Car {
    offset: Vec3,
    distance_travelled: f32,
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

            car.distance_travelled += speed * time.delta_secs();
            let road_len = (road.end - road.start).length();
            if car.distance_travelled > road_len {
                car.distance_travelled = 0.0;
            }
            let direction = (road.end - road.start).normalize() * car.dir;

            let progress = car.distance_travelled / road_len;
            car_transform.translation = (road.start + car.offset) + direction * road_len * progress;
        }
    }
}
