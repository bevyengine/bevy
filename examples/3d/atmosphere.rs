//! This example showcases pbr atmospheric scattering
#[cfg(feature = "free_camera")]
use bevy::camera_controller::free_camera::{FreeCamera, FreeCameraPlugin};
use std::f32::consts::PI;

use bevy::{
    anti_alias::taa::TemporalAntiAliasing,
    camera::Exposure,
    color::palettes::css::BLACK,
    core_pipeline::tonemapping::Tonemapping,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    image::{
        ImageAddressMode, ImageFilterMode, ImageLoaderSettings, ImageSampler,
        ImageSamplerDescriptor,
    },
    input::keyboard::KeyCode,
    light::{
        atmosphere::{Falloff, PhaseFunction, ScatteringMedium, ScatteringTerm},
        light_consts::lux,
        Atmosphere, AtmosphereEnvironmentMapLight, FogVolume, SunDisk, VolumetricFog,
        VolumetricLight,
    },
    pbr::{
        AtmosphereMode, AtmosphereSettings, CloudLayer, CloudWeather, DefaultOpaqueRendererMethod,
        VolumetricCloudSettings,
        ExtendedMaterial, MaterialExtension, ScreenSpaceReflections,
    },
    post_process::bloom::Bloom,
    prelude::*,
    render::{
        diagnostic::RenderDiagnosticsPlugin,
        render_resource::{AsBindGroup, ShaderType},
        view::screenshot::{save_to_disk, Screenshot},
    },
    shader::ShaderRef,
};

#[derive(Resource)]
struct GameState {
    paused: bool,
    planet_scale_view: bool,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            paused: true,
            planet_scale_view: false,
        }
    }
}

#[derive(Resource)]
struct AtmospherePresets {
    earth: Handle<ScatteringMedium>,
    mars: Handle<ScatteringMedium>,
}

fn main() {
    let mut app = App::new();
    app.insert_resource(DefaultOpaqueRendererMethod::deferred())
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(GameState::default())
        .insert_resource(GlobalAmbientLight::NONE)
        .add_plugins((
            DefaultPlugins,
            #[cfg(feature = "free_camera")]
            FreeCameraPlugin,
        ))
        .add_plugins(MaterialPlugin::<ExtendedMaterial<StandardMaterial, Water>>::default())
        .add_systems(
            Startup,
            (setup_camera_fog, setup_terrain_scene, print_controls),
        )
        .add_systems(
            Update,
            (
                dynamic_scene,
                atmosphere_controls,
                visibility_toggle.after(atmosphere_controls),
            ),
        );

    // Optional benchmarking hooks (used for profiling the atmosphere/cloud passes):
    // - `ATMOSPHERE_DIAGNOSTICS=1` logs frame time and per-pass render timings once per second.
    // - `ATMOSPHERE_SCREENSHOT=<path.png>` saves a screenshot after `ATMOSPHERE_SCREENSHOT_SECS` (default 8 s).
    // - `ATMOSPHERE_EXIT_SECS=<secs>` exits the app after the given number of seconds.
    let bench = BenchSettings::from_env();
    if bench.diagnostics {
        app.add_plugins((
            FrameTimeDiagnosticsPlugin::default(),
            RenderDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
        ));
    }
    // `ATMOSPHERE_FREEZE=1`: pause virtual time (sun motion, wind/noise offsets, animations) so
    // realtime and reference renders see identical clouds.
    if std::env::var("ATMOSPHERE_FREEZE").is_ok() {
        app.add_systems(Startup, |mut time: ResMut<Time<Virtual>>| time.pause());
    }
    if bench.screenshot.is_some() || bench.exit_secs.is_some() {
        app.insert_resource(bench).add_systems(Update, bench_hooks);
    }
    app.run();
}

#[derive(Resource, Default)]
struct BenchSettings {
    diagnostics: bool,
    screenshot: Option<String>,
    screenshot_secs: f32,
    exit_secs: Option<f32>,
    no_clouds: bool,
    sunset: bool,
    screenshot_taken: bool,
    startup_frames: Option<u32>,
    frame: u32,
}

impl BenchSettings {
    fn from_env() -> Self {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        Self {
            diagnostics: var("ATMOSPHERE_DIAGNOSTICS").is_some(),
            screenshot: var("ATMOSPHERE_SCREENSHOT"),
            screenshot_secs: var("ATMOSPHERE_SCREENSHOT_SECS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(8.0),
            exit_secs: var("ATMOSPHERE_EXIT_SECS").and_then(|v| v.parse().ok()),
            no_clouds: var("ATMOSPHERE_NO_CLOUDS").is_some(),
            sunset: var("ATMOSPHERE_SUNSET").is_some(),
            screenshot_taken: false,
            startup_frames: var("ATMOSPHERE_SCREENSHOT_FRAMES").and_then(|v| v.parse().ok()),
            frame: 0,
        }
    }
}

fn bench_hooks(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut bench: ResMut<BenchSettings>,
    mut exit: MessageWriter<AppExit>,
    cloud_layers: Query<Entity, With<CloudLayer>>,
) {
    let _sunset = bench.sunset; // sun tilt applied at spawn via ATMOSPHERE_SUNSET
    if bench.no_clouds {
        for entity in &cloud_layers {
            commands.entity(entity).remove::<CloudLayer>();
        }
    }
    let elapsed = time.elapsed_secs();
    if !bench.screenshot_taken
        && elapsed >= bench.screenshot_secs
        && let Some(path) = bench.screenshot.clone()
    {
        bench.screenshot_taken = true;
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    // `ATMOSPHERE_SCREENSHOT_FRAMES=<n>` + `ATMOSPHERE_SCREENSHOT=<dir/prefix>`: also capture each
    // of the first n frames (startup convergence debugging) to `<prefix>_fNN.png`.
    if let (Some(n), Some(prefix)) = (bench.startup_frames, bench.screenshot.clone())
        && bench.frame < n
    {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(format!("{prefix}_f{:02}.png", bench.frame)));
    }
    bench.frame += 1;
    if bench.exit_secs.is_some_and(|secs| elapsed >= secs) {
        exit.write(AppExit::Success);
    }
}

fn print_controls() {
    println!("Atmosphere Example Controls:");
    println!("    1          - Switch to lookup texture rendering method");
    println!("    2          - Switch to raymarched rendering method");
    println!("    3          - Switch to Earth atmosphere");
    println!("    4          - Switch to Mars atmosphere");
    println!("    5          - Switch between normal and planet scale (space) view");
    println!("    C          - Toggle cloud layer");
    println!("    Left/Right - Decrease/Increase cloud density");
    println!("    Enter      - Pause/Resume sun motion");
    println!("    Up/Down    - Increase/Decrease exposure");
}

fn atmosphere_controls(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut planet_atmosphere: Query<(&mut Atmosphere, &mut GlobalTransform)>,
    mut camera_settings: Query<&mut AtmosphereSettings, With<Camera3d>>,
    mut sun_disks: Query<&mut SunDisk, With<DirectionalLight>>,
    mut atmosphere_entities: Query<(Entity, Option<&mut CloudLayer>), With<Atmosphere>>,
    mut commands: Commands,
    atmosphere_presets: Res<AtmospherePresets>,
    mut game_state: ResMut<GameState>,
    mut camera_exposure: Query<&mut Exposure, With<Camera3d>>,
    time: Res<Time>,
) {
    if keyboard_input.just_pressed(KeyCode::Digit3) {
        for (mut atmosphere, mut transform) in &mut planet_atmosphere {
            *atmosphere = Atmosphere::earth(atmosphere_presets.earth.clone());
            *transform = GlobalTransform::from_translation(-Vec3::Y * atmosphere.inner_radius);
        }
        for mut sun_disk in &mut sun_disks {
            sun_disk.angular_size = SunDisk::EARTH.angular_size;
        }
        println!("Switched to Earth atmosphere");
    }

    if keyboard_input.just_pressed(KeyCode::Digit4) {
        for (mut atmosphere, mut transform) in &mut planet_atmosphere {
            *atmosphere = Atmosphere::mars(atmosphere_presets.mars.clone());
            *transform = GlobalTransform::from_translation(-Vec3::Y * atmosphere.inner_radius);
        }
        for mut sun_disk in &mut sun_disks {
            sun_disk.angular_size = SunDisk::MARS.angular_size;
        }
        println!("Switched to Mars atmosphere");
    }

    if keyboard_input.just_pressed(KeyCode::Digit1) {
        for mut settings in &mut camera_settings {
            settings.rendering_method = AtmosphereMode::LookupTexture;
            println!("Switched to lookup texture rendering method");
        }
    }

    if keyboard_input.just_pressed(KeyCode::Digit2) {
        for mut settings in &mut camera_settings {
            settings.rendering_method = AtmosphereMode::Raymarched;
            println!("Switched to raymarched rendering method");
        }
    }

    if keyboard_input.just_pressed(KeyCode::Digit5) {
        game_state.planet_scale_view = !game_state.planet_scale_view;
        if game_state.planet_scale_view {
            for mut settings in &mut camera_settings {
                settings.aerial_view_lut_size = UVec3::new(160, 88, 64);
                settings.aerial_view_lut_max_distance = 6.2e4;
            }
            println!("Switched to planet scale view");
        } else {
            for mut settings in &mut camera_settings {
                settings.aerial_view_lut_size = UVec3::new(32, 32, 32);
                settings.aerial_view_lut_max_distance = 3.2e4;
            }
            println!("Switched to normal view");
        }
    }

    if keyboard_input.just_pressed(KeyCode::KeyC) {
        let has_clouds = atmosphere_entities
            .iter()
            .any(|(_, cloud_layer)| cloud_layer.is_some());
        let entities: Vec<Entity> = atmosphere_entities
            .iter()
            .map(|(entity, _)| entity)
            .collect();
        if has_clouds {
            for entity in entities {
                commands.entity(entity).remove::<CloudLayer>();
            }
            println!("Cloud layer disabled");
        } else {
            for entity in entities {
                commands.entity(entity).insert(CloudLayer::default());
            }
            println!("Cloud layer enabled");
        }
    }

    if keyboard_input.pressed(KeyCode::ArrowLeft) {
        for (_, cloud_layer) in atmosphere_entities.iter_mut() {
            if let Some(mut cloud_layer) = cloud_layer {
                cloud_layer.cloud_density =
                    (cloud_layer.cloud_density - time.delta_secs()).max(0.0);
                println!("Cloud density: {:.2}", cloud_layer.cloud_density);
            }
        }
    }

    if keyboard_input.pressed(KeyCode::ArrowRight) {
        for (_, cloud_layer) in atmosphere_entities.iter_mut() {
            if let Some(mut cloud_layer) = cloud_layer {
                cloud_layer.cloud_density =
                    (cloud_layer.cloud_density + time.delta_secs()).min(100.0);
                println!("Cloud density: {:.2}", cloud_layer.cloud_density);
            }
        }
    }

    if keyboard_input.just_pressed(KeyCode::Enter) {
        game_state.paused = !game_state.paused;
    }

    if keyboard_input.pressed(KeyCode::ArrowUp) {
        for mut exposure in &mut camera_exposure {
            exposure.ev100 -= time.delta_secs() * 2.0;
        }
    }

    if keyboard_input.pressed(KeyCode::ArrowDown) {
        for mut exposure in &mut camera_exposure {
            exposure.ev100 += time.delta_secs() * 2.0;
        }
    }

    // Animate clouds by updating noise offset
    for (_, cloud_layer) in atmosphere_entities.iter_mut() {
        if let Some(_cloud_layer) = cloud_layer {
            // _cloud_layer.noise_offset.x += time.delta_secs() * 100.0;
        }
    }
}

fn visibility_toggle(
    game_state: Res<GameState>,
    mut visibility_query: Query<&mut Visibility, Or<(With<WaterPlane>, With<FogVolume>)>>,
) {
    let visibility = if game_state.planet_scale_view {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    };
    for mut v in &mut visibility_query {
        *v = visibility;
    }
}

fn setup_camera_fog(
    mut commands: Commands,
    mut scattering_mediums: ResMut<Assets<ScatteringMedium>>,
    asset_server: Res<AssetServer>,
) {
    let mut medium = ScatteringMedium::earth(256, 256);
    medium.terms.push(ScatteringTerm {
        absorption: Vec3::splat(2e-5),
        scattering: Vec3::splat(1e-4),
        falloff: Falloff::Exponential { scale: 0.2 / 60.0 },
        phase: PhaseFunction::Mie { asymmetry: 0.76 },
    });
    let earth_medium = scattering_mediums.add(medium);
    let mars_phase = asset_server.load("textures/mars_mie_phase.ktx2");
    let mars_medium = scattering_mediums.add(ScatteringMedium::mars(256, 256, mars_phase));

    commands.insert_resource(AtmospherePresets {
        earth: earth_medium.clone(),
        mars: mars_medium.clone(),
    });

    // Spawn earth atmosphere
    commands.spawn((
        // Ground albedo 0.28 to match the cloud ground bounce (CLOUD_GROUND_ALBEDO) and the reference.
        Atmosphere {
            ground_albedo: Vec3::splat(0.28),
            ..Atmosphere::earth(earth_medium)
        },
        // Add a volumetric cloud layer using 3D FBM noise
        // Physically realistic parameters (units: m^-1 per unit density):
        // - Density is normalized to [0, 1] in the shader
        // - cloud_scattering: 0.0008 m^-1 per unit density (physically correct for water droplets)
        //   At max density (1.0): scattering_coeff = 0.0008
        //   After phase amplification (~6.5x at forward angles): 0.0008 * 6.5 ≈ 0.005
        //   This matches atmospheric scattering magnitudes (~0.001-0.01 range)
        // - cloud_absorption: 0.00005 m^-1 per unit density (realistic for water clouds)
        //   Single-scattering albedo ≈ 0.94 (typical water clouds have albedo 0.95-0.99)
        CloudLayer {
            cloud_layer_start: 6_361_500.0, // cloud base 1.5 km above the surface
            cloud_layer_end: 6_364_000.0,   // tops 4 km (2.5 km thick layer)
            cloud_density: 1.0, // Used for enabling/disabling, actual density comes from noise (normalized [0, 1])
            cloud_absorption: 0.000_02, // single-scattering albedo ~0.999
            cloud_scattering: 0.02, // ~50 m mean free path at full density (cumulus 0.02-0.1 m^-1)
            // Larger scale = larger cloud features (more “big cumulus”, less “small puffs”)
            noise_scale: 32_000.0, // weather tile; shape noise period = 1/8 (4 km)
            noise_offset: Vec3::ZERO,
            detail_noise_scale: 600.0, // Smaller scale = higher-frequency breakup
            detail_strength: 1.0,
            wind: Vec3::new(20.0, 0.0, 8.0),
        },
        CloudWeather::default(),
    ));

    commands.spawn((
        Camera3d::default(),
        // `ATMOSPHERE_VIEW=up|wide` picks an alternative camera for cloud screenshots.
        match std::env::var("ATMOSPHERE_VIEW").as_deref() {
            Ok("up") => Transform::from_xyz(-2.8, 0.045, 0.0)
                .looking_at(Vec3::new(-1.8, 1.2, 0.3), Vec3::Y),
            // Toward the sun (+X), tilted up; use with ATMOSPHERE_SUNSET=1 for a low sun.
            Ok("sun") => Transform::from_xyz(-2.8, 0.045, 0.0)
                .looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
            Ok("wide") => Transform::from_xyz(-2.8, 0.045, 0.0)
                .looking_at(Vec3::new(-2.8, 0.25, -3.0), Vec3::Y),
            _ => Transform::from_xyz(-2.8, 0.045, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        },
        // Can be adjusted to change the scene scale and rendering quality
        AtmosphereSettings {
            // Profiling override: `ATMOSPHERE_LUT_MODE=1` starts in `AtmosphereMode::LookupTexture`.
            rendering_method: if std::env::var("ATMOSPHERE_LUT_MODE").is_ok() {
                AtmosphereMode::LookupTexture
            } else {
                AtmosphereMode::Raymarched
            },
            // Profiling override: `ATMOSPHERE_SHADOW_INTERLEAVE=1` traces the whole cloud shadow
            // map every frame instead of time-slicing it.
            cloud_shadow_map_update_interleave: std::env::var("ATMOSPHERE_SHADOW_INTERLEAVE")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(AtmosphereSettings::default().cloud_shadow_map_update_interleave),
            cloud_shadow_temporal_enabled: std::env::var("ATMOSPHERE_SHADOW_TEMPORAL_ALPHA").is_ok(),
            cloud_shadow_temporal_alpha: std::env::var("ATMOSPHERE_SHADOW_TEMPORAL_ALPHA")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(AtmosphereSettings::default().cloud_shadow_temporal_alpha),
            ..default()
        },
        // The directional light illuminance used in this scene
        // (the one recommended for use with this feature) is
        // quite bright, so raising the exposure compensation helps
        // bring the scene to a nicer brightness range.
        Exposure { ev100: 13.0 },
        // Tonemapper chosen just because it looked good with the scene, any
        // tonemapper would be fine :)
        Tonemapping::AcesFitted,
        // Bloom gives the sun a much more natural look.
        Bloom::NATURAL,
        // Enables the atmosphere to drive reflections and ambient lighting (IBL) for this view
        AtmosphereEnvironmentMapLight::default(),
        #[cfg(feature = "free_camera")]
        FreeCamera::default(),
        VolumetricFog {
            ambient_intensity: 0.0,
            ..default()
        },
        Msaa::Off,
        VolumetricCloudSettings::MEDIUM,
        TemporalAntiAliasing::default(),
        ScreenSpaceReflections {
            min_perceptual_roughness: 0.0..0.0,
            ..default()
        },
    ));
}

#[derive(Component)]
struct Terrain;

#[derive(Component)]
struct WaterPlane;

/// A custom [`ExtendedMaterial`] that creates animated water ripples.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
struct Water {
    /// The normal map image.
    ///
    /// Note that, like all normal maps, this must not be loaded as sRGB.
    #[texture(100)]
    #[sampler(101)]
    normals: Handle<Image>,

    // Parameters to the water shader.
    #[uniform(102)]
    settings: WaterSettings,
}

/// Parameters to the water shader.
#[derive(ShaderType, Debug, Clone)]
struct WaterSettings {
    /// How much to displace each octave each frame, in the u and v directions.
    /// Two octaves are packed into each `vec4`.
    octave_vectors: [Vec4; 2],
    /// How wide the waves are in each octave.
    octave_scales: Vec4,
    /// How high the waves are in each octave.
    octave_strengths: Vec4,
}

impl MaterialExtension for Water {
    fn deferred_fragment_shader() -> ShaderRef {
        "shaders/water_material.wesl".into()
    }
}

fn setup_terrain_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut water_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, Water>>>,
    asset_server: Res<AssetServer>,
) {
    // Sun
    commands.spawn((
        DirectionalLight {
            shadow_maps_enabled: true,
            // lux::RAW_SUNLIGHT is recommended for use with this feature, since
            // other values approximate sunlight *post-scattering* in various
            // conditions. RAW_SUNLIGHT in comparison is the illuminance of the
            // sun unfiltered by the atmosphere, so it is the proper input for
            // sunlight to be filtered by the atmosphere.
            illuminance: lux::RAW_SUNLIGHT,
            ..default()
        },
        // `ATMOSPHERE_SUN_ELEV_DEG=<deg>` fixes the sun elevation (overrides ATMOSPHERE_SUNSET).
        match std::env::var("ATMOSPHERE_SUN_ELEV_DEG")
            .ok()
            .and_then(|v| v.parse::<f32>().ok())
        {
            Some(deg) => Transform::from_xyz(deg.to_radians().cos(), deg.to_radians().sin(), 0.0),
            None => Transform::from_xyz(
                1.0,
                if std::env::var("ATMOSPHERE_SUNSET").is_ok() { 0.06 } else { 0.4 },
                0.0,
            ),
        }
        .looking_at(Vec3::ZERO, Vec3::Y),
        VolumetricLight,
        SunDisk::EARTH,
    ));

    // // spawn the fog volume
    // commands.spawn((
    //     FogVolume::default(),
    //     Transform::from_scale(Vec3::new(10.0, 1.0, 10.0)).with_translation(Vec3::Y * 0.5),
    // ));

    // Terrain: temporarily off by default while tuning clouds + atmosphere (the planet ground
    // sphere still renders); `ATMOSPHERE_TERRAIN=1` brings it back.
    if std::env::var("ATMOSPHERE_TERRAIN").is_err() {
        return;
    }
    commands.spawn((
        Terrain,
        WorldAssetRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/terrain/terrain.glb")),
        ),
        Transform::from_xyz(-1.0, 0.0, -0.5)
            .with_scale(Vec3::splat(0.5))
            .with_rotation(Quat::from_rotation_y(PI / 2.0)),
    ));

    // spawn_water(
    //     &mut commands,
    //     &asset_server,
    //     &mut meshes,
    //     &mut water_materials,
    // );
}

// Spawns the water plane.
fn spawn_water(
    commands: &mut Commands,
    asset_server: &AssetServer,
    meshes: &mut Assets<Mesh>,
    water_materials: &mut Assets<ExtendedMaterial<StandardMaterial, Water>>,
) {
    commands.spawn((
        WaterPlane,
        Visibility::default(),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(1.0)))),
        MeshMaterial3d(
            water_materials.add(ExtendedMaterial {
                base: StandardMaterial {
                    base_color: BLACK.into(),
                    perceptual_roughness: 0.0,
                    ..default()
                },
                extension: Water {
                    normals: asset_server
                        .load_builder()
                        .with_settings(|settings: &mut ImageLoaderSettings| {
                            settings.is_srgb = false;
                            settings.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                                address_mode_u: ImageAddressMode::Repeat,
                                address_mode_v: ImageAddressMode::Repeat,
                                mag_filter: ImageFilterMode::Linear,
                                min_filter: ImageFilterMode::Linear,
                                ..default()
                            });
                        })
                        .load("textures/water_normals.png"),
                    // These water settings are just random values to create some
                    // variety.
                    settings: WaterSettings {
                        octave_vectors: [
                            vec4(0.080, 0.059, 0.073, -0.062),
                            vec4(0.153, 0.138, -0.149, -0.195),
                        ],
                        octave_scales: vec4(1.0, 2.1, 7.9, 14.9) * 500.0,
                        octave_strengths: vec4(0.16, 0.18, 0.093, 0.044) * 0.2,
                    },
                },
            }),
        ),
        Transform::from_scale(Vec3::splat(100.0)),
    ));
}

fn dynamic_scene(
    mut suns: Query<&mut Transform, With<DirectionalLight>>,
    time: Res<Time>,
    sun_motion_state: Res<GameState>,
) {
    // Only rotate the sun if motion is not paused
    if !sun_motion_state.paused {
        suns.iter_mut()
            .for_each(|mut tf| tf.rotate_x(-time.delta_secs() * PI / 40.0));
    }
}
