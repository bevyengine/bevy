//! Demonstrates visibility ranges, also known as HLODs.

use std::f32::consts::PI;

use bevy::{
    input::mouse::MouseWheel,
    math::vec3,
    pbr::{light_consts::lux::FULL_DAYLIGHT, CascadeShadowConfigBuilder},
    prelude::*,
    render::view::VisibilityRange,
};

// Where the camera is focused.
const CAMERA_FOCAL_POINT: Vec3 = vec3(0.0, 0.3, 0.0);
// Speed in units per frame.
const CAMERA_KEYBOARD_ZOOM_SPEED: f32 = 0.05;
// Speed in radians per frame.
const CAMERA_KEYBOARD_PAN_SPEED: f32 = 0.01;
// Speed in units per frame.
const CAMERA_MOUSE_MOVEMENT_SPEED: f32 = 0.25;
// The minimum distance that the camera is allowed to be from the model.
const MIN_ZOOM_DISTANCE: f32 = 0.5;

// The visibility ranges for high-poly and low-poly models respectively, when
// both models are being shown.
static NORMAL_VISIBILITY_RANGE_HIGH_POLY: VisibilityRange = VisibilityRange {
    start_margin: 0.0..0.0,
    end_margin: 3.0..4.0,
};
static NORMAL_VISIBILITY_RANGE_LOW_POLY: VisibilityRange = VisibilityRange {
    start_margin: 3.0..4.0,
    end_margin: 8.0..9.0,
};

// A visibility model that we use to always show a model (until the camera is so
// far zoomed out that it's culled entirely).
static SINGLE_MODEL_VISIBILITY_RANGE: VisibilityRange = VisibilityRange {
    start_margin: 0.0..0.0,
    end_margin: 8.0..9.0,
};

// A visibility range that we use to completely hide a model.
static INVISIBLE_VISIBILITY_RANGE: VisibilityRange = VisibilityRange {
    start_margin: 0.0..0.0,
    end_margin: 0.0..0.0,
};

// Allows us to identify the main model.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
enum MainModel {
    // The high-poly version.
    HighPoly,
    // The low-poly version.
    LowPoly,
}

// The current mode.
#[derive(Default, Resource)]
struct AppStatus {
    // Whether to show only one model.
    show_one_model_only: Option<MainModel>,
}

// Sets up the app.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Visibility Range Example".into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<AppStatus>()
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                move_camera,
                set_visibility_ranges,
                update_help_text,
                update_mode,
            ),
        )
        .run();
}

// Set up a simple 3D scene. Load the two meshes.
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    app_status: Res<AppStatus>,
) {
    // Spawn a plane.
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(50.0, 50.0)),
        material: materials.add(Color::srgb(0.1, 0.2, 0.1)),
        ..default()
    });

    // Spawn the two HLODs.

    commands
        .spawn(SceneBundle {
            scene: asset_server
                .load(GltfAssetLabel::Scene(0).from_asset("models/FlightHelmet/FlightHelmet.gltf")),
            ..default()
        })
        .insert(MainModel::HighPoly);

    commands
        .spawn(SceneBundle {
            scene: asset_server.load(
                GltfAssetLabel::Scene(0)
                    .from_asset("models/FlightHelmetLowPoly/FlightHelmetLowPoly.gltf"),
            ),
            ..default()
        })
        .insert(MainModel::LowPoly);

    // Spawn a light.
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            illuminance: FULL_DAYLIGHT,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_rotation(Quat::from_euler(
            EulerRot::ZYX,
            0.0,
            PI * -0.15,
            PI * -0.15,
        )),
        cascade_shadow_config: CascadeShadowConfigBuilder {
            maximum_distance: 30.0,
            first_cascade_far_bound: 0.9,
            ..default()
        }
        .into(),
        ..default()
    });

    // Spawn a camera.
    commands
        .spawn(Camera3dBundle {
            transform: Transform::from_xyz(0.7, 0.7, 1.0).looking_at(CAMERA_FOCAL_POINT, Vec3::Y),
            ..default()
        })
        .insert(EnvironmentMapLight {
            diffuse_map: asset_server.load("environment_maps/pisa_diffuse_rgb9e5_zstd.ktx2"),
            specular_map: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            intensity: 150.0,
        });

    // Create the text.
    commands.spawn(
        TextBundle {
            text: app_status.create_text(),
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

// We need to add the `VisibilityRange` components manually, as glTF currently
// has no way to specify visibility ranges. This system watches for new meshes,
// determines which `Scene` they're under, and adds the `VisibilityRange`
// component as appropriate.
fn set_visibility_ranges(
    mut commands: Commands,
    mut new_meshes: Query<Entity, Added<Handle<Mesh>>>,
    parents: Query<(Option<&Parent>, Option<&MainModel>)>,
) {
    // Loop over each newly-added mesh.
    for new_mesh in new_meshes.iter_mut() {
        // Search for the nearest ancestor `MainModel` component.
        let (mut current, mut main_model) = (new_mesh, None);
        while let Ok((parent, maybe_main_model)) = parents.get(current) {
            if let Some(model) = maybe_main_model {
                main_model = Some(model);
                break;
            }
            match parent {
                Some(parent) => current = **parent,
                None => break,
            }
        }

        // Add the `VisibilityRange` component.
        match main_model {
            Some(MainModel::HighPoly) => {
                commands
                    .entity(new_mesh)
                    .insert(NORMAL_VISIBILITY_RANGE_HIGH_POLY.clone())
                    .insert(MainModel::HighPoly);
            }
            Some(MainModel::LowPoly) => {
                commands
                    .entity(new_mesh)
                    .insert(NORMAL_VISIBILITY_RANGE_LOW_POLY.clone())
                    .insert(MainModel::LowPoly);
            }
            None => {}
        }
    }
}

// Process the movement controls.
fn move_camera(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut mouse_wheel_events: EventReader<MouseWheel>,
    mut cameras: Query<&mut Transform, With<Camera3d>>,
) {
    let (mut zoom_delta, mut theta_delta) = (0.0, 0.0);

    // Process zoom in and out via the keyboard.
    if keyboard_input.pressed(KeyCode::KeyW) || keyboard_input.pressed(KeyCode::ArrowUp) {
        zoom_delta -= CAMERA_KEYBOARD_ZOOM_SPEED;
    } else if keyboard_input.pressed(KeyCode::KeyS) || keyboard_input.pressed(KeyCode::ArrowDown) {
        zoom_delta += CAMERA_KEYBOARD_ZOOM_SPEED;
    }

    // Process left and right pan via the keyboard.
    if keyboard_input.pressed(KeyCode::KeyA) || keyboard_input.pressed(KeyCode::ArrowLeft) {
        theta_delta -= CAMERA_KEYBOARD_PAN_SPEED;
    } else if keyboard_input.pressed(KeyCode::KeyD) || keyboard_input.pressed(KeyCode::ArrowRight) {
        theta_delta += CAMERA_KEYBOARD_PAN_SPEED;
    }

    // Process zoom in and out via the mouse wheel.
    for event in mouse_wheel_events.read() {
        zoom_delta -= event.y * CAMERA_MOUSE_MOVEMENT_SPEED;
    }

    // Update the camera transform.
    for transform in cameras.iter_mut() {
        let transform = transform.into_inner();

        let direction = transform.translation.normalize_or_zero();
        let magnitude = transform.translation.length();

        let new_direction = Mat3::from_rotation_y(theta_delta) * direction;
        let new_magnitude = (magnitude + zoom_delta).max(MIN_ZOOM_DISTANCE);

        transform.translation = new_direction * new_magnitude;
        transform.look_at(CAMERA_FOCAL_POINT, Vec3::Y);
    }
}

// Toggles modes if the user requests.
fn update_mode(
    mut meshes: Query<(&mut VisibilityRange, &MainModel)>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut app_status: ResMut<AppStatus>,
) {
    // Toggle the mode as requested.
    if keyboard_input.just_pressed(KeyCode::Digit1) || keyboard_input.just_pressed(KeyCode::Numpad1)
    {
        app_status.show_one_model_only = None;
    } else if keyboard_input.just_pressed(KeyCode::Digit2)
        || keyboard_input.just_pressed(KeyCode::Numpad2)
    {
        app_status.show_one_model_only = Some(MainModel::HighPoly);
    } else if keyboard_input.just_pressed(KeyCode::Digit3)
        || keyboard_input.just_pressed(KeyCode::Numpad3)
    {
        app_status.show_one_model_only = Some(MainModel::LowPoly);
    } else {
        return;
    }

    // Update the visibility ranges as appropriate.
    for (mut visibility_range, main_model) in meshes.iter_mut() {
        *visibility_range = match (main_model, app_status.show_one_model_only) {
            (&MainModel::HighPoly, Some(MainModel::LowPoly))
            | (&MainModel::LowPoly, Some(MainModel::HighPoly)) => {
                INVISIBLE_VISIBILITY_RANGE.clone()
            }
            (&MainModel::HighPoly, Some(MainModel::HighPoly))
            | (&MainModel::LowPoly, Some(MainModel::LowPoly)) => {
                SINGLE_MODEL_VISIBILITY_RANGE.clone()
            }
            (&MainModel::HighPoly, None) => NORMAL_VISIBILITY_RANGE_HIGH_POLY.clone(),
            (&MainModel::LowPoly, None) => NORMAL_VISIBILITY_RANGE_LOW_POLY.clone(),
        }
    }
}

// A system that updates the help text.
fn update_help_text(mut text_query: Query<&mut Text>, app_status: Res<AppStatus>) {
    for mut text in text_query.iter_mut() {
        *text = app_status.create_text();
    }
}

impl AppStatus {
    // Creates and returns help text reflecting the app status.
    fn create_text(&self) -> Text {
        Text::from_section(
            format!(
                "\
{} (1) Switch from high-poly to low-poly based on camera distance
{} (2) Show only the high-poly model
{} (3) Show only the low-poly model
Press 1, 2, or 3 to switch which model is shown
Press WASD or use the mouse wheel to move the camera",
                if self.show_one_model_only.is_none() {
                    '>'
                } else {
                    ' '
                },
                if self.show_one_model_only == Some(MainModel::HighPoly) {
                    '>'
                } else {
                    ' '
                },
                if self.show_one_model_only == Some(MainModel::LowPoly) {
                    '>'
                } else {
                    ' '
                },
            ),
            TextStyle::default(),
        )
    }
}
