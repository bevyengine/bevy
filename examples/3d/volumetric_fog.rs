//! Demonstrates volumetric fog and lighting (light shafts or god rays).

use bevy::{
    color::palettes::css::RED,
    core_pipeline::{bloom::Bloom, tonemapping::Tonemapping, Skybox},
    input::mouse::{MouseMotion, MouseWheel},
    math::{vec3, VectorSpace},
    pbr::{FogVolume, VolumetricFog, VolumetricLight},
    prelude::*,
};

const DIRECTIONAL_LIGHT_MOVEMENT_SPEED: f32 = 0.02;

/// The current settings that the user has chosen.
#[derive(Resource)]
struct AppSettings {
    /// Whether volumetric spot light is on.
    volumetric_spotlight: bool,
    /// Whether volumetric point light is on.
    volumetric_pointlight: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            volumetric_spotlight: true,
            volumetric_pointlight: true,
        }
    }
}

// Define a struct to store parameters for the point light's movement.
#[derive(Component)]
struct MoveBackAndForthHorizontally {
    min_x: f32,
    max_x: f32,
    speed: f32,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(ClearColor(Color::Srgba(Srgba {
            red: 0.02,
            green: 0.02,
            blue: 0.02,
            alpha: 1.0,
        })))
        .insert_resource(AmbientLight::NONE)
        .init_resource::<AppSettings>()
        .add_systems(Startup, setup)
        .add_systems(Update, tweak_scene)
        .add_systems(Update, (move_directional_light, move_point_light))
        .add_systems(Update, adjust_app_settings)
        .add_systems(
            Update,
            (pan_camera, smooth_camera_movement.after(pan_camera)),
        )
        .run();
}

#[derive(Component)]
struct CameraOrbit {
    target_transform: Transform,
    distance: f32,
    target: Vec3,
}

/// Initializes the scene.
fn setup(mut commands: Commands, asset_server: Res<AssetServer>, app_settings: Res<AppSettings>) {
    // Spawn the glTF scene.
    commands.spawn(SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset("models/VolumetricFogExample/VolumetricFogExample.glb"),
    )));

    let target = Vec3::new(-1.5, 1.7, 3.5);
    let initial_transform = Transform::from_xyz(-1.7, 1.5, 4.5).looking_at(target, Vec3::Y);
    let initial_distance = 7.0;

    // Spawn the camera.
    commands
        .spawn((
            Camera3d::default(),
            Camera {
                hdr: true,
                ..default()
            },
            initial_transform,
            CameraOrbit {
                target_transform: initial_transform,
                distance: initial_distance,
                target,
            },
            Tonemapping::TonyMcMapface,
            Bloom::default(),
        ))
        .insert(Skybox {
            image: asset_server.load("environment_maps/pisa_specular_rgb9e5_zstd.ktx2"),
            brightness: 1000.0,
            ..default()
        })
        .insert(VolumetricFog {
            // This value is explicitly set to 0 since we have no environment map light
            ambient_intensity: 0.0,
            ..default()
        });

    // Add the point light
    commands.spawn((
        Transform::from_xyz(-0.4, 1.9, 1.0),
        PointLight {
            shadows_enabled: true,
            range: 150.0,
            color: RED.into(),
            intensity: 1000.0,
            ..default()
        },
        VolumetricLight,
        MoveBackAndForthHorizontally {
            min_x: -1.93,
            max_x: -0.4,
            speed: -0.2,
        },
    ));

    // Add the spot light
    commands.spawn((
        Transform::from_xyz(-1.8, 3.9, -2.7).looking_at(Vec3::ZERO, Vec3::Y),
        SpotLight {
            intensity: 5000.0, // lumens
            color: Color::WHITE,
            shadows_enabled: true,
            inner_angle: 0.76,
            outer_angle: 0.94,
            ..default()
        },
        VolumetricLight,
    ));

    // Add the fog volume.
    commands.spawn((
        FogVolume::default(),
        Transform::from_scale(Vec3::splat(35.0)),
    ));

    // Add the help text.
    commands.spawn((
        create_text(&app_settings),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}

fn create_text(app_settings: &AppSettings) -> Text {
    format!(
        "{}\n{}\n{}",
        "Press WASD or the arrow keys to change the direction of the directional light",
        if app_settings.volumetric_pointlight {
            "Press P to turn volumetric point light off"
        } else {
            "Press P to turn volumetric point light on"
        },
        if app_settings.volumetric_spotlight {
            "Press L to turn volumetric spot light off"
        } else {
            "Press L to turn volumetric spot light on"
        }
    )
    .into()
}

/// A system that makes directional lights in the glTF scene into volumetric
/// lights with shadows.
fn tweak_scene(
    mut commands: Commands,
    mut lights: Query<(Entity, &mut DirectionalLight), Changed<DirectionalLight>>,
) {
    for (light, mut directional_light) in lights.iter_mut() {
        // Shadows are needed for volumetric lights to work.
        directional_light.shadows_enabled = true;
        commands.entity(light).insert(VolumetricLight);
    }
}

/// Processes user requests to move the directional light.
fn move_directional_light(
    input: Res<ButtonInput<KeyCode>>,
    mut directional_lights: Query<&mut Transform, With<DirectionalLight>>,
) {
    let mut delta_theta = Vec2::ZERO;
    if input.pressed(KeyCode::KeyW) || input.pressed(KeyCode::ArrowUp) {
        delta_theta.y += DIRECTIONAL_LIGHT_MOVEMENT_SPEED;
    }
    if input.pressed(KeyCode::KeyS) || input.pressed(KeyCode::ArrowDown) {
        delta_theta.y -= DIRECTIONAL_LIGHT_MOVEMENT_SPEED;
    }
    if input.pressed(KeyCode::KeyA) || input.pressed(KeyCode::ArrowLeft) {
        delta_theta.x += DIRECTIONAL_LIGHT_MOVEMENT_SPEED;
    }
    if input.pressed(KeyCode::KeyD) || input.pressed(KeyCode::ArrowRight) {
        delta_theta.x -= DIRECTIONAL_LIGHT_MOVEMENT_SPEED;
    }

    if delta_theta == Vec2::ZERO {
        return;
    }

    let delta_quat = Quat::from_euler(EulerRot::XZY, delta_theta.y, 0.0, delta_theta.x);
    for mut transform in directional_lights.iter_mut() {
        transform.rotate(delta_quat);
    }
}

// Toggle point light movement between left and right.
fn move_point_light(
    timer: Res<Time>,
    mut objects: Query<(&mut Transform, &mut MoveBackAndForthHorizontally)>,
) {
    for (mut transform, mut move_data) in objects.iter_mut() {
        let mut translation = transform.translation;
        let mut need_toggle = false;
        translation.x += move_data.speed * timer.delta_secs();
        if translation.x > move_data.max_x {
            translation.x = move_data.max_x;
            need_toggle = true;
        } else if translation.x < move_data.min_x {
            translation.x = move_data.min_x;
            need_toggle = true;
        }
        if need_toggle {
            move_data.speed = -move_data.speed;
        }
        transform.translation = translation;
    }
}

// Adjusts app settings per user input.
fn adjust_app_settings(
    mut commands: Commands,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut app_settings: ResMut<AppSettings>,
    mut point_lights: Query<Entity, With<PointLight>>,
    mut spot_lights: Query<Entity, With<SpotLight>>,
    mut text: Query<&mut Text>,
) {
    // If there are no changes, we're going to bail for efficiency. Record that
    // here.
    let mut any_changes = false;

    // If the user pressed P, toggle volumetric state of the point light.
    if keyboard_input.just_pressed(KeyCode::KeyP) {
        app_settings.volumetric_pointlight = !app_settings.volumetric_pointlight;
        any_changes = true;
    }
    // If the user pressed L, toggle volumetric state of the spot light.
    if keyboard_input.just_pressed(KeyCode::KeyL) {
        app_settings.volumetric_spotlight = !app_settings.volumetric_spotlight;
        any_changes = true;
    }

    // If there were no changes, bail out.
    if !any_changes {
        return;
    }

    // Update volumetric settings.
    for point_light in point_lights.iter_mut() {
        if app_settings.volumetric_pointlight {
            commands.entity(point_light).insert(VolumetricLight);
        } else {
            commands.entity(point_light).remove::<VolumetricLight>();
        }
    }
    for spot_light in spot_lights.iter_mut() {
        if app_settings.volumetric_spotlight {
            commands.entity(spot_light).insert(VolumetricLight);
        } else {
            commands.entity(spot_light).remove::<VolumetricLight>();
        }
    }

    // Update the help text.
    for mut text in text.iter_mut() {
        *text = create_text(&app_settings);
    }
}

fn pan_camera(
    mut motion_evr: EventReader<MouseMotion>,
    mut scroll_evr: EventReader<MouseWheel>,
    mut camera_query: Query<(&Transform, &mut CameraOrbit), With<Camera3d>>,
    mouse_button: Res<ButtonInput<MouseButton>>,
    camera_query_view: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window>,
) {
    let (camera_transform, mut camera_orbit) = camera_query.single_mut();
    for ev in scroll_evr.read() {
        let zoom_factor = camera_orbit.distance * 0.01;
        camera_orbit.distance = (camera_orbit.distance - ev.y * zoom_factor).clamp(0.5, 400.0);
        let direction = camera_orbit.target_transform.translation.normalize();
        camera_orbit.target_transform.translation = direction * camera_orbit.distance;
    }

    for ev in motion_evr.read() {
        if mouse_button.pressed(MouseButton::Left) {
            let orbit_speed = 0.005;
            let yaw_rotation = Quat::from_axis_angle(Vec3::Y, -ev.delta.x * orbit_speed);
            let pitch_rotation =
                Quat::from_axis_angle(camera_transform.local_x().into(), -ev.delta.y * orbit_speed);
            let current_pos = camera_orbit.target_transform.translation;
            let rotated_pos = yaw_rotation * pitch_rotation * current_pos;
            camera_orbit.target_transform.translation = rotated_pos;
            camera_orbit.target_transform.look_at(Vec3::ZERO, Vec3::Y);
        }
    }
}

fn smooth_camera_movement(
    time: Res<Time>,
    mut camera_query: Query<(&mut Transform, &CameraOrbit), With<Camera3d>>,
) {
    let damping = 1.0 - (-8.0 * time.delta_secs()).exp();

    // Update camera
    if let Ok((mut transform, orbit)) = camera_query.get_single_mut() {
        transform.translation = transform
            .translation
            .lerp(orbit.target_transform.translation, damping);
        transform.rotation = transform
            .rotation
            .slerp(orbit.target_transform.rotation, damping);
    }
}
