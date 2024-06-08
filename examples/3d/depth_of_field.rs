//! Demonstrates depth of field (DOF).
//!
//! The depth of field effect simulates the blur that a real camera produces on
//! objects that are out of focus.
//!
//! The test scene is inspired by [a blog post on depth of field in Unity].
//! However, the technique used in Bevy has little to do with that blog post,
//! and all the assets are original.
//!
//! [a blog post on depth of field in Unity]: https://catlikecoding.com/unity/tutorials/advanced-rendering/depth-of-field/

use bevy::{
    core_pipeline::{
        bloom::BloomSettings,
        dof::{self, DepthOfFieldMode, DepthOfFieldSettings},
        tonemapping::Tonemapping,
    },
    pbr::Lightmap,
    prelude::*,
    render::camera::PhysicalCameraParameters,
};

/// The increments in which the user can adjust the focal distance, in meters
/// per frame.
const FOCAL_DISTANCE_SPEED: f32 = 0.05;
/// The increments in which the user can adjust the f-number, in units per frame.
const APERTURE_F_STOP_SPEED: f32 = 0.01;

/// The minimum distance that we allow the user to focus on.
const MIN_FOCAL_DISTANCE: f32 = 0.01;
/// The minimum f-number that we allow the user to set.
const MIN_APERTURE_F_STOPS: f32 = 0.05;

/// A resource that stores the settings that the user can change.
#[derive(Clone, Copy, Resource)]
struct AppSettings {
    /// The distance from the camera to the area in the most focus.
    focal_distance: f32,

    /// The [f-number]. Lower numbers cause objects outside the focal distance
    /// to be blurred more.
    ///
    /// [f-number]: https://en.wikipedia.org/wiki/F-number
    aperture_f_stops: f32,

    /// Whether depth of field is on, and, if so, whether we're in Gaussian or
    /// bokeh mode.
    mode: Option<DepthOfFieldMode>,
}

fn main() {
    App::new()
        .init_resource::<AppSettings>()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Depth of Field Example".to_string(),
                ..default()
            }),
            ..default()
        }))
        .add_systems(Startup, setup)
        .add_systems(Update, tweak_scene)
        .add_systems(
            Update,
            (adjust_focus, change_mode, update_dof_settings, update_text).chain(),
        )
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, app_settings: Res<AppSettings>) {
    // Spawn the camera. Enable HDR and bloom, as that highlights the depth of
    // field effect.
    let mut camera = commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 4.5, 8.25).looking_at(Vec3::ZERO, Vec3::Y),
        camera: Camera {
            hdr: true,
            ..default()
        },
        tonemapping: Tonemapping::TonyMcMapface,
        ..default()
    });
    camera.insert(BloomSettings::NATURAL);

    // Insert the depth of field settings.
    if let Some(dof_settings) = Option::<DepthOfFieldSettings>::from(*app_settings) {
        camera.insert(dof_settings);
    }

    // Spawn the scene.
    commands.spawn(SceneBundle {
        scene: asset_server.load(
            GltfAssetLabel::Scene(0)
                .from_asset("models/DepthOfFieldExample/DepthOfFieldExample.glb"),
        ),
        ..default()
    });

    // Spawn the help text.
    commands.spawn(
        TextBundle {
            text: create_text(&app_settings),
            ..default()
        }
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        }),
    );
}

/// Adjusts the focal distance and f-number per user inputs.
fn adjust_focus(input: Res<ButtonInput<KeyCode>>, mut app_settings: ResMut<AppSettings>) {
    // Change the focal distance if the user requested.
    let distance_delta = if input.pressed(KeyCode::ArrowDown) {
        -FOCAL_DISTANCE_SPEED
    } else if input.pressed(KeyCode::ArrowUp) {
        FOCAL_DISTANCE_SPEED
    } else {
        0.0
    };

    // Change the f-number if the user requested.
    let f_stop_delta = if input.pressed(KeyCode::ArrowLeft) {
        -APERTURE_F_STOP_SPEED
    } else if input.pressed(KeyCode::ArrowRight) {
        APERTURE_F_STOP_SPEED
    } else {
        0.0
    };

    app_settings.focal_distance =
        (app_settings.focal_distance + distance_delta).max(MIN_FOCAL_DISTANCE);
    app_settings.aperture_f_stops =
        (app_settings.aperture_f_stops + f_stop_delta).max(MIN_APERTURE_F_STOPS);
}

/// Changes the depth of field mode (Gaussian, bokeh, off) per user inputs.
fn change_mode(input: Res<ButtonInput<KeyCode>>, mut app_settings: ResMut<AppSettings>) {
    if !input.just_pressed(KeyCode::Space) {
        return;
    }

    app_settings.mode = match app_settings.mode {
        Some(DepthOfFieldMode::Bokeh) => Some(DepthOfFieldMode::Gaussian),
        Some(DepthOfFieldMode::Gaussian) => None,
        None => Some(DepthOfFieldMode::Bokeh),
    }
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            // Objects 7 meters away will be in full focus.
            focal_distance: 7.0,

            // Set a nice blur level.
            //
            // This is a really low F-number, but we want to demonstrate the
            // effect, even if it's kind of unrealistic.
            aperture_f_stops: 1.0 / 8.0,

            // Turn on bokeh by default, as it's the nicest-looking technique.
            mode: Some(DepthOfFieldMode::Bokeh),
        }
    }
}

/// Writes the depth of field settings into the camera.
fn update_dof_settings(
    mut commands: Commands,
    view_targets: Query<Entity, With<Camera>>,
    app_settings: Res<AppSettings>,
) {
    let dof_settings: Option<DepthOfFieldSettings> = (*app_settings).into();
    for view in view_targets.iter() {
        match dof_settings {
            None => {
                commands.entity(view).remove::<DepthOfFieldSettings>();
            }
            Some(dof_settings) => {
                commands.entity(view).insert(dof_settings);
            }
        }
    }
}

/// Makes one-time adjustments to the scene that can't be encoded in glTF.
fn tweak_scene(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut lights: Query<&mut DirectionalLight, Changed<DirectionalLight>>,
    mut named_entities: Query<
        (Entity, &Name, &Handle<StandardMaterial>),
        (With<Handle<Mesh>>, Without<Lightmap>),
    >,
) {
    // Turn on shadows.
    for mut light in lights.iter_mut() {
        light.shadows_enabled = true;
    }

    // Add a nice lightmap to the circuit board.
    for (entity, name, material) in named_entities.iter_mut() {
        if &**name == "CircuitBoard" {
            materials.get_mut(material).unwrap().lightmap_exposure = 10000.0;
            commands.entity(entity).insert(Lightmap {
                image: asset_server.load("models/DepthOfFieldExample/CircuitBoardLightmap.hdr"),
                ..default()
            });
        }
    }
}

/// Update the help text entity per the current app settings.
fn update_text(mut texts: Query<&mut Text>, app_settings: Res<AppSettings>) {
    for mut text in texts.iter_mut() {
        *text = create_text(&app_settings);
    }
}

/// Regenerates the app text component per the current app settings.
fn create_text(app_settings: &AppSettings) -> Text {
    Text::from_section(app_settings.help_text(), TextStyle::default())
}

impl From<AppSettings> for Option<DepthOfFieldSettings> {
    fn from(app_settings: AppSettings) -> Self {
        app_settings.mode.map(|mode| DepthOfFieldSettings {
            mode,
            focal_distance: app_settings.focal_distance,
            aperture_f_stops: app_settings.aperture_f_stops,
            max_depth: 14.0,
            ..default()
        })
    }
}

impl AppSettings {
    /// Builds the help text.
    fn help_text(&self) -> String {
        let Some(mode) = self.mode else {
            return "Mode: Off (Press Space to change)".to_owned();
        };

        // We leave these as their defaults, so we don't need to store them in
        // the app settings and can just fetch them from the default camera
        // parameters.
        let sensor_height = PhysicalCameraParameters::default().sensor_height;
        let fov = PerspectiveProjection::default().fov;

        format!(
            "Focal distance: {} m (Press Up/Down to change)
Aperture F-stops: f/{} (Press Left/Right to change)
Sensor height: {}mm
Focal length: {}mm
Mode: {} (Press Space to change)",
            self.focal_distance,
            self.aperture_f_stops,
            sensor_height * 1000.0,
            dof::calculate_focal_length(sensor_height, fov) * 1000.0,
            match mode {
                DepthOfFieldMode::Bokeh => "Bokeh",
                DepthOfFieldMode::Gaussian => "Gaussian",
            }
        )
    }
}
