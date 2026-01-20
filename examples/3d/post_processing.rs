//! Demonstrates Bevy's built-in postprocessing features.
//!
//! Includes:
//!
//! - Chromatic Aberration
//! - Vignette

use std::f32::consts::PI;

use bevy::{
    light::CascadeShadowConfigBuilder,
    post_process::effect_stack::{ChromaticAberration, Vignette},
    prelude::*,
    render::view::Hdr,
};

/// The number of units per frame to add to or subtract from intensity when the
/// arrow keys are held.
const ADJUSTMENT_SPEED: f32 = 0.005;

/// The maximum supported chromatic aberration intensity level.
const MAX_CHROMATIC_ABERRATION_INTENSITY: f32 = 0.4;

/// The settings that the user can control.
#[derive(Resource)]
struct AppSettings {
    /// The index of the currently selected UI item.
    selected: usize,
    /// The intensity of the chromatic aberration effect.
    chromatic_aberration_intensity: f32,
    /// The intensity of the vignette effect.
    vignette_intensity: f32,
    /// The radius of the vignette.
    vignette_radius: f32,
    /// The smoothness of the vignette.
    vignette_smoothness: f32,
    /// The roundness of the vignette.
    vignette_roundness: f32,
    vignette_edge_compensation: f32,
}

/// The entry point.
fn main() {
    App::new()
        .init_resource::<AppSettings>()
        .add_plugins(DefaultPlugins)
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
fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Spawn the camera.
    spawn_camera(&mut commands, &asset_server);

    // Create the scene.
    spawn_scene(&mut commands, &asset_server);

    // Spawn the help text.
    spawn_text(&mut commands);
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
        // Include the `Vignette` component.
        Vignette::default(),
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
            shadow_maps_enabled: true,
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

/// Spawns the help text.
fn spawn_text(commands: &mut Commands) {
    commands.spawn((
        Text::default(),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

impl Default for AppSettings {
    fn default() -> Self {
        let vignette_default = Vignette::default();
        Self {
            selected: 0,
            chromatic_aberration_intensity: ChromaticAberration::default().intensity,
            vignette_intensity: vignette_default.intensity,
            vignette_radius: vignette_default.radius,
            vignette_smoothness: vignette_default.smoothness,
            vignette_roundness: vignette_default.roundness,
            vignette_edge_compensation: vignette_default.edge_compensation,
        }
    }
}

/// Handles requests from the user to change the chromatic aberration intensity.
fn handle_keyboard_input(mut app_settings: ResMut<AppSettings>, input: Res<ButtonInput<KeyCode>>) {
    if input.just_pressed(KeyCode::ArrowUp) && app_settings.selected > 0 {
        app_settings.selected -= 1;
    } else if input.just_pressed(KeyCode::ArrowDown) && app_settings.selected < 6 {
        app_settings.selected += 1;
    }

    let mut delta = 0.0;
    if input.pressed(KeyCode::ArrowLeft) {
        delta -= ADJUSTMENT_SPEED;
    } else if input.pressed(KeyCode::ArrowRight) {
        delta += ADJUSTMENT_SPEED;
    }

    // If no arrow key was pressed, just bail out.
    if delta == 0.0 {
        return;
    }

    match app_settings.selected {
        0 => {
            app_settings.chromatic_aberration_intensity =
                (app_settings.chromatic_aberration_intensity + delta)
                    .clamp(0.0, MAX_CHROMATIC_ABERRATION_INTENSITY);
        }
        1 => {
            app_settings.vignette_intensity =
                (app_settings.vignette_intensity + delta).clamp(0.0, 1.0);
        }
        2 => app_settings.vignette_radius = (app_settings.vignette_radius + delta).clamp(0.0, 2.0),
        3 => {
            app_settings.vignette_smoothness = (app_settings.vignette_smoothness + delta).max(0.01)
        }
        4 => app_settings.vignette_roundness = (app_settings.vignette_roundness + delta).max(0.01),
        5 => {
            app_settings.vignette_edge_compensation =
                (app_settings.vignette_edge_compensation + delta).clamp(0.0, 1.0)
        }
        _ => {}
    }
}

/// Updates the [`ChromaticAberration`] settings per the [`AppSettings`].
fn update_chromatic_aberration_settings(
    mut chromatic_aberration: Query<&mut ChromaticAberration>,
    mut vignette: Query<&mut Vignette>,
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

    for mut vignette in &mut vignette {
        vignette.intensity = app_settings.vignette_intensity;
        vignette.radius = app_settings.vignette_radius;
        vignette.smoothness = app_settings.vignette_smoothness;
        vignette.roundness = app_settings.vignette_roundness;
        vignette.edge_compensation = app_settings.vignette_edge_compensation;
    }
}

/// Updates the help text at the bottom of the screen to reflect the current
/// [`AppSettings`].
fn update_help_text(mut text: Single<&mut Text>, app_settings: Res<AppSettings>) {
    text.clear();
    //let vignette_mode_list = ["Cosine Fourth Law", "Higher-order Powers", "Smoothstep"];
    let text_list = [
        format!(
            "Chromatic aberration intensity: {:.2}\n",
            app_settings.chromatic_aberration_intensity
        ),
        format!(
            "Vignette intensity: {:.2}\n",
            app_settings.vignette_intensity
        ),
        format!("Vignette radius: {:.2}\n", app_settings.vignette_radius),
        format!(
            "Vignette smoothness: {:.2}\n",
            app_settings.vignette_smoothness
        ),
        format!(
            "Vignette roundness: {:.2}\n",
            app_settings.vignette_roundness
        ),
        format!(
            "Vignette edge_compensation: {:.2}\n",
            app_settings.vignette_edge_compensation
        ),
    ];
    for (i, val) in text_list.iter().enumerate() {
        if i == app_settings.selected {
            text.push_str("> ");
        }
        text.push_str(val);
    }
    text.push_str("\n(Press Up or Down to select)\n(Press Left or Right to change)");
}
