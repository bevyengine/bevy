//! Demonstrates Bevy's built-in postprocessing features.
//!
//! Currently, this simply consists of chromatic aberration.

use std::f32::consts::PI;

use bevy::{
    core_pipeline::post_process::ChromaticAberration, light::CascadeShadowConfigBuilder,
    prelude::*, render::view::Hdr,
};

/// The number of units per frame to add to or subtract from intensity when the
/// arrow keys are held.
const CHROMATIC_ABERRATION_INTENSITY_ADJUSTMENT_SPEED: f32 = 0.002;

/// The maximum supported chromatic aberration intensity level.
const MAX_CHROMATIC_ABERRATION_INTENSITY: f32 = 0.4;

/// The settings that the user can control.
#[derive(Resource)]
struct AppSettings {
    /// The intensity of the chromatic aberration effect.
    chromatic_aberration_intensity: f32,
}

/// The entry point.
fn main() {
    App::new()
        .init_resource::<AppSettings>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Chromatic Aberration Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, handle_keyboard_input)
        .add_systems(
            Update,
            (update_chromatic_aberration_settings, update_help_text)
                .run_if(resource_changed::<AppSettings>)
                .after(handle_keyboard_input),
        )
        .run();
}

/// Creates the example scene and spawns the UI.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, app_settings: Res<AppSettings>) {
    // Spawn the camera.
    spawn_camera(&mut commands, &asset_server);

    // Create the scene.
    spawn_scene(&mut commands, &asset_server);

    // Spawn the help text.
    spawn_text(&mut commands, &app_settings);
}

/// Spawns the camera, including the [`ChromaticAberration`] component.
fn spawn_camera(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn((
        Camera3d::default(),
        Hdr,
        Transform::from_xyz(0.7, 0.7, 1.0).looking_at(Vec3::new(0.0, 0.3, 0.0), Vec3::Y),
        DistanceFog {
            color: Color::srgb_u8(43, 44, 47),
            falloff: FogFalloff::Linear {
                start: 1.0,
                end: 8.0,
            },
            ..default()
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 2000.0,
            ..default()
        },
        // Include the `ChromaticAberration` component.
        ChromaticAberration::default(),
    ));
}

/// Spawns the scene.
///
/// This is just the tonemapping test scene, chosen for the fact that it uses a
/// variety of colors.
fn spawn_scene(commands: &mut Commands, asset_server: &AssetServer) {
    // Spawn the main scene.
    commands.spawn(SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("models/TonemappingTest/TonemappingTest.gltf"),
    )));

    // Spawn the flight helmet.
    commands.spawn((
        SceneRoot(
            asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf")),
        ),
        Transform::from_xyz(0.5, 0.0, -0.5).with_rotation(Quat::from_rotation_y(-0.15 * PI)),
    ));

    // Spawn the light.
    commands.spawn((
        DirectionalLight {
            illuminance: 15000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::ZYX, 0.0, PI * -0.15, PI * -0.15)),
        CascadeShadowConfigBuilder {
            maximum_distance: 3.0,
            first_cascade_far_bound: 0.9,
            ..default()
        }
        .build(),
    ));
}

/// Spawns the help text at the bottom of the screen.
fn spawn_text(commands: &mut Commands, app_settings: &AppSettings) {
    commands.spawn((
        create_help_text(app_settings),
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            chromatic_aberration_intensity: ChromaticAberration::default().intensity,
        }
    }
}

/// Creates help text at the bottom of the screen.
fn create_help_text(app_settings: &AppSettings) -> Text {
    format!(
        "Chromatic aberration intensity: {} (Press Left or Right to change)",
        app_settings.chromatic_aberration_intensity
    )
    .into()
}

/// Handles requests from the user to change the chromatic aberration intensity.
fn handle_keyboard_input(mut app_settings: ResMut<AppSettings>, input: Res<ButtonInput<KeyCode>>) {
    let mut delta = 0.0;
    if input.pressed(KeyCode::ArrowLeft) {
        delta -= CHROMATIC_ABERRATION_INTENSITY_ADJUSTMENT_SPEED;
    } else if input.pressed(KeyCode::ArrowRight) {
        delta += CHROMATIC_ABERRATION_INTENSITY_ADJUSTMENT_SPEED;
    }

    // If no arrow key was pressed, just bail out.
    if delta == 0.0 {
        return;
    }

    app_settings.chromatic_aberration_intensity = (app_settings.chromatic_aberration_intensity
        + delta)
        .clamp(0.0, MAX_CHROMATIC_ABERRATION_INTENSITY);
}

/// Updates the [`ChromaticAberration`] settings per the [`AppSettings`].
fn update_chromatic_aberration_settings(
    mut chromatic_aberration: Query<&mut ChromaticAberration>,
    app_settings: Res<AppSettings>,
) {
    let intensity = app_settings.chromatic_aberration_intensity;

    // Pick a reasonable maximum sample size for the intensity to avoid an
    // artifact whereby the individual samples appear instead of producing
    // smooth streaks of color.
    //
    // Don't take this formula too seriously; it hasn't been heavily tuned.
    let max_samples = ((intensity - 0.02) / (0.20 - 0.02) * 56.0 + 8.0)
        .clamp(8.0, 64.0)
        .round() as u32;

    for mut chromatic_aberration in &mut chromatic_aberration {
        chromatic_aberration.intensity = intensity;
        chromatic_aberration.max_samples = max_samples;
    }
}

/// Updates the help text at the bottom of the screen to reflect the current
/// [`AppSettings`].
fn update_help_text(mut text: Query<&mut Text>, app_settings: Res<AppSettings>) {
    for mut text in text.iter_mut() {
        *text = create_help_text(&app_settings);
    }
}
