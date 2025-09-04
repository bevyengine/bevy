//! This example showcases pbr atmospheric scattering
#[path = "../helpers/camera_controller.rs"]
mod camera_controller;

use camera_controller::{CameraController, CameraControllerPlugin};
use std::f32::consts::PI;

use bevy::{
    anti_aliasing::fxaa::Fxaa,
    camera::Exposure,
    core_pipeline::tonemapping::Tonemapping,
    diagnostic::LogDiagnosticsPlugin,
    input::keyboard::KeyCode,
    light::{
        light_consts::lux, AtmosphereEnvironmentMapLight, CascadeShadowConfigBuilder, FogVolume,
        VolumetricFog, VolumetricLight,
    },
    pbr::{
        Atmosphere, AtmosphereMode, AtmosphereSettings, DefaultOpaqueRendererMethod,
        ScreenSpaceReflections,
    },
    post_process::bloom::Bloom,
    prelude::*,
};

#[derive(Resource, Default)]
struct GameState {
    paused: bool,
}

fn main() {
    App::new()
        .insert_resource(DefaultOpaqueRendererMethod::deferred())
        .insert_resource(ClearColor(Color::BLACK))
        .insert_resource(GameState::default())
        .add_plugins((
            DefaultPlugins,
            CameraControllerPlugin,
            LogDiagnosticsPlugin::default(),
        ))
        .add_systems(
            Startup,
            (setup_camera_fog, setup_terrain_scene, print_controls),
        )
        .add_systems(Update, (dynamic_scene, atmosphere_controls))
        .run();
}

fn print_controls() {
    println!("Atmosphere Example Controls:");
    println!("    1          - Switch to default rendering method");
    println!("    2          - Switch to raymarched rendering method");
    println!("    Enter      - Pause/Resume sun motion");
    println!("    Up/Down    - Increase/Decrease exposure");
}

fn atmosphere_controls(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut atmosphere_settings: Query<&mut AtmosphereSettings>,
    mut game_state: ResMut<GameState>,
    mut camera_exposure: Query<&mut Exposure, With<Camera3d>>,
    time: Res<Time>,
) {
    if keyboard_input.just_pressed(KeyCode::Digit1) {
        for mut settings in &mut atmosphere_settings {
            settings.rendering_method = AtmosphereMode::LookupTexture;
            println!("Switched to default rendering method");
        }
    }

    if keyboard_input.just_pressed(KeyCode::Digit2) {
        for mut settings in &mut atmosphere_settings {
            settings.rendering_method = AtmosphereMode::Raymarched;
            println!("Switched to raymarched rendering method");
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
}

fn setup_camera_fog(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-1.2, 0.15, 0.0).looking_at(Vec3::Y * 0.1, Vec3::Y),
        // This is the component that enables atmospheric scattering for a camera
        Atmosphere::EARTH,
        // Can be adjusted to change the scene scale and rendering quality
        AtmosphereSettings::default(),
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
        CameraController::default(),
        VolumetricFog::default(),
        Msaa::Off,
        Fxaa::default(),
        ScreenSpaceReflections::default(),
    ));
}

#[derive(Component)]
struct Terrain;

fn setup_terrain_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Configure a properly scaled cascade shadow map for this scene (defaults are too large, mesh units are in km)
    let cascade_shadow_config = CascadeShadowConfigBuilder {
        first_cascade_far_bound: 0.3,
        maximum_distance: 15.0,
        ..default()
    }
    .build();

    // Sun
    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            // lux::RAW_SUNLIGHT is recommended for use with this feature, since
            // other values approximate sunlight *post-scattering* in various
            // conditions. RAW_SUNLIGHT in comparison is the illuminance of the
            // sun unfiltered by the atmosphere, so it is the proper input for
            // sunlight to be filtered by the atmosphere.
            illuminance: lux::RAW_SUNLIGHT,
            ..default()
        },
        Transform::from_xyz(1.0, 0.4, 0.0).looking_at(Vec3::ZERO, Vec3::Y),
        VolumetricLight,
        cascade_shadow_config,
    ));

    // spawn the fog volume
    commands.spawn((
        FogVolume::default(),
        Transform::from_scale(Vec3::splat(10.0)),
    ));

    let sphere_mesh = meshes.add(Mesh::from(Sphere { radius: 1.0 }));

    // light probe spheres
    commands.spawn((
        Mesh3d(sphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            metallic: 1.0,
            perceptual_roughness: 0.0,
            ..default()
        })),
        Transform::from_xyz(-0.3, 0.1, -0.1).with_scale(Vec3::splat(0.05)),
    ));

    commands.spawn((
        Mesh3d(sphere_mesh.clone()),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::WHITE,
            metallic: 0.0,
            perceptual_roughness: 1.0,
            ..default()
        })),
        Transform::from_xyz(-0.3, 0.1, 0.1).with_scale(Vec3::splat(0.05)),
    ));

    // Terrain
    commands.spawn((
        Terrain,
        SceneRoot(
            asset_server.load(GltfAssetLabel::Scene(0).from_asset("models/terrain/terrain.glb")),
        ),
        Transform::from_xyz(-1.0, 0.0, -0.5)
            .with_scale(Vec3::splat(0.5))
            .with_rotation(Quat::from_rotation_y(PI / 2.0)),
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
            .for_each(|mut tf| tf.rotate_x(-time.delta_secs() * PI / 10.0));
    }
}
