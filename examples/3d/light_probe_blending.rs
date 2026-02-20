//! Demonstrates blending between multiple reflection probes.
//!
//! This example shows a reflective sphere that moves between two rooms, each of
//! which contains a reflection probe with a falloff range. Bevy performs a
//! blend between the two reflection probes as the sphere moves.

use std::f32::consts::{FRAC_PI_4, PI};

use bevy::{
    camera::Hdr,
    camera_controller::free_camera::{self, FreeCamera, FreeCameraPlugin},
    color::palettes::css::{CORNFLOWER_BLUE, CRIMSON, TAN, WHITE},
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    light::ParallaxCorrection,
    math::ops::{atan2, cos, sin},
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::widgets::{WidgetClickEvent, WidgetClickSender};

#[path = "../helpers/widgets.rs"]
mod widgets;

/// The settings that the user has chosen.
#[derive(Resource, Default)]
struct AppStatus {
    /// Whether the gizmos that show the boundaries of the light probe regions
    /// are to be shown.
    gizmos_enabled: GizmosEnabled,
    /// Which object to show: either a reflective sphere or a reflective prism.
    object_to_show: ObjectToShow,
    /// Whether to use an orbital pan/zoom camera or a free camera.
    camera_mode: CameraMode,
}

/// Whether the gizmos that show the boundaries of the light probe regions are
/// to be shown.
#[derive(Clone, Copy, Default, PartialEq)]
enum GizmosEnabled {
    /// The gizmos are shown.
    #[default]
    On,
    /// The gizmos are hidden.
    Off,
}

/// Which reflective object to show.
#[derive(Clone, Copy, Default, PartialEq)]
enum ObjectToShow {
    /// A reflective sphere that moves between rooms.
    #[default]
    Sphere,
    /// A reflective prism that is static and stretches across the length of the
    /// two rooms.
    Prism,
}

/// How the user can control the camera.
#[derive(Clone, Copy, Default, PartialEq)]
enum CameraMode {
    /// The camera is a pan/zoom orbital camera controllable with dragging and
    /// the mouse wheel.
    #[default]
    Orbit,
    /// The camera is a free camera controllable by clicking and dragging and
    /// using the WASDEQ controls.
    Free,
}

/// A marker component for the reflective sphere.
#[derive(Clone, Copy, Component, Debug)]
struct ReflectiveSphere;

/// A marker component for the reflective prism.
#[derive(Clone, Copy, Component, Debug)]
struct ReflectivePrism;

/// A marker component for the help text at the top of the screen.
#[derive(Clone, Copy, Component, Debug)]
struct HelpText;

/// The speed at which the sphere moves, as a ratio of the total distance it
/// travels to seconds.
///
/// Specifically, the value of 0.3 means that it moves 3/10 of the way to the
/// other side per second.
const SPHERE_MOVEMENT_SPEED: f32 = 0.3;

/// The side length of each room, in meters.
const ROOM_SIDE_LENGTH: f32 = 10.0;

/// The number of meters that separates the center of each room.
const ROOM_SEPARATION: f32 = 11.0;

/// The side length of the light probe cube, in meters.
const LIGHT_PROBE_SIDE_LENGTH: f32 = 15.0;

/// The distance over which the light probe fades out, expressed as a fraction
/// of the side length of the probe.
const LIGHT_PROBE_FALLOFF: f32 = 0.5;

/// The side length of the simulated reflected area for each light probe,
/// specified as a half-extent in light probe space.
///
/// We want this side length, in world space, to be half of the world-space room
/// side length. Since the light probe is scaled by `LIGHT_PROBE_SIDE_LENGTH`,
/// we divide the room side length by the light probe side length to get this
/// value, and multiply by 0.5 to convert from a full extent to a half-extent.
/// That way, when Bevy applies the `LIGHT_PROBE_SIDE_LENGTH` scale, the light
/// probe side length factor cancels, and we're left with a parallax correction
/// side length of `ROOM_SIDE_LENGTH` in world space.
///
/// A small epsilon value of 0.01 is added in order to ensure that the light
/// probe parallax bounds encompass the entire room. Otherwise, unsightly
/// Z-fighting can occur on the room walls.
const LIGHT_PROBE_PARALLAX_CORRECTION_SIDE_LENGTH: f32 =
    ROOM_SIDE_LENGTH / LIGHT_PROBE_SIDE_LENGTH * 0.5 + 0.01;

/// The number of radians of inclination (pitch) that one pixel of mouse
/// movement corresponds to.
const CAMERA_ORBIT_SPEED_INCLINATION: f32 = 0.003;

/// The number of radians of azumith (yaw) that one pixel of mouse movement
/// corresponds to.
const CAMERA_ORBIT_SPEED_AZIMUTH: f32 = 0.004;

/// The number of meters that one line of mouse scroll corresponds to.
const CAMERA_ZOOM_SPEED: f32 = 0.15;

/// Information about the orbital pan/zoom camera.
///
/// These are in [spherical coordinates].
///
/// [spherical coordinates]: https://en.wikipedia.org/wiki/Spherical_coordinate_system
#[derive(Component)]
struct OrbitCamera {
    /// The distance between the camera and the sphere, in meters.
    radius: f32,
    /// The camera latitude in radians, relative to the sphere.
    inclination: f32,
    /// The camera longitude in radians, relative to the sphere.
    azimuth: f32,
}

/// The brightness of the light probe.
const LIGHT_PROBE_INTENSITY: f32 = 500.0;

/// The entry point.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Light Probe Blending Example".into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(FreeCameraPlugin)
        .init_resource::<AppStatus>()
        .add_message::<WidgetClickEvent<GizmosEnabled>>()
        .add_message::<WidgetClickEvent<ObjectToShow>>()
        .add_message::<WidgetClickEvent<CameraMode>>()
        .add_systems(Startup, setup)
        .add_systems(Update, (move_sphere, orbit_camera).chain())
        .add_systems(
            Update,
            (
                widgets::handle_ui_interactions::<GizmosEnabled>,
                handle_gizmos_enabled_change,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                widgets::handle_ui_interactions::<ObjectToShow>,
                handle_object_to_show_change,
            )
                .chain(),
        )
        .add_systems(
            Update,
            (
                widgets::handle_ui_interactions::<CameraMode>,
                handle_camera_mode_change,
            )
                .chain()
                .after(free_camera::run_freecamera_controller),
        )
        .add_systems(
            Update,
            update_radio_buttons
                .after(widgets::handle_ui_interactions::<GizmosEnabled>)
                .after(widgets::handle_ui_interactions::<ObjectToShow>)
                .after(widgets::handle_ui_interactions::<CameraMode>),
        )
        .add_systems(Update, draw_gizmos)
        .run();
}

/// Performs initial setup of the scene.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut gizmo_config_store: ResMut<GizmoConfigStore>,
) {
    adjust_gizmo_settings(&mut gizmo_config_store);

    let reflective_material = create_reflective_material(&mut materials);

    spawn_camera(&mut commands);
    spawn_gltf_scene(&mut commands, &asset_server);
    spawn_reflective_sphere(&mut commands, &mut meshes, reflective_material.clone());
    spawn_reflective_prism(&mut commands, &mut meshes, reflective_material);
    spawn_light_probes(&mut commands, &asset_server);
    spawn_buttons(&mut commands);
    spawn_help_text(&mut commands);
}

/// Adjusts the gizmo settings so that the gizmos appear on top of all other
/// geometry.
///
/// If we didn't do this, then the rooms would cover up many of the gizmos.
fn adjust_gizmo_settings(gizmo_config_store: &mut GizmoConfigStore) {
    for (_, gizmo_config, _) in &mut gizmo_config_store.iter_mut() {
        gizmo_config.depth_bias = -1.0;
    }
}

/// Creates the perfectly-reflective material that the sphere and prism use.
fn create_reflective_material(
    materials: &mut Assets<StandardMaterial>,
) -> Handle<StandardMaterial> {
    materials.add(StandardMaterial {
        base_color: WHITE.into(),
        metallic: 1.0,
        reflectance: 1.0,
        perceptual_roughness: 0.0,
        ..default()
    })
}

/// Spawns the orbital pan/zoom camera.
fn spawn_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::IDENTITY,
        Hdr,
        OrbitCamera {
            radius: 3.0,
            inclination: 7.0 * FRAC_PI_4,
            azimuth: FRAC_PI_4,
        },
    ));
}

/// Spawns the glTF scene that contains the two rooms.
fn spawn_gltf_scene(commands: &mut Commands, asset_server: &AssetServer) {
    commands.spawn(SceneRoot(asset_server.load(
        GltfAssetLabel::Scene(0).from_asset(get_web_asset_url("two_rooms.glb")),
    )));
}

/// Spawns the reflective sphere, creating its mesh in the process.
fn spawn_reflective_sphere(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
) {
    // Create a mesh.
    let sphere = meshes.add(Sphere::default().mesh().uv(32, 18));

    // Spawn the sphere.
    commands.spawn((
        Mesh3d(sphere),
        MeshMaterial3d(material),
        Transform::IDENTITY,
        ReflectiveSphere,
    ));
}

/// Spawns the reflective prism, creating its mesh in the process.
///
/// The reflective prism starts invisible, but the user can toggle it on and off
/// as desired.
fn spawn_reflective_prism(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    material: Handle<StandardMaterial>,
) {
    // Create a mesh.
    let cube = meshes.add(
        Cuboid {
            half_size: vec3(2.0, 1.0, 10.0),
        }
        .mesh()
        .build()
        // We use flat normals so that the surface appears flat, not curved.
        .with_duplicated_vertices()
        .with_computed_flat_normals(),
    );

    // Spawn the cube.
    commands.spawn((
        Mesh3d(cube),
        MeshMaterial3d(material),
        Transform::from_xyz(0.0, -4.0, -5.5),
        ReflectivePrism,
        Visibility::Hidden,
    ));
}

/// Spawns the two light probes, one for each room.
fn spawn_light_probes(commands: &mut Commands, asset_server: &AssetServer) {
    // Spawn the first room's light probe.
    commands.spawn((
        LightProbe {
            falloff: Vec3::splat(LIGHT_PROBE_FALLOFF),
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load(get_web_asset_url("diffuse_room1.ktx2")),
            specular_map: asset_server.load(get_web_asset_url("specular_room1.ktx2")),
            intensity: LIGHT_PROBE_INTENSITY,
            ..default()
        },
        Transform::from_scale(vec3(1.0, -1.0, 1.0) * LIGHT_PROBE_SIDE_LENGTH)
            .with_rotation(Quat::from_rotation_x(PI)),
        ParallaxCorrection::Custom(Vec3::splat(LIGHT_PROBE_PARALLAX_CORRECTION_SIDE_LENGTH)),
    ));

    // Spawn the second room's light probe.
    commands.spawn((
        LightProbe {
            falloff: Vec3::splat(LIGHT_PROBE_FALLOFF),
        },
        EnvironmentMapLight {
            diffuse_map: asset_server.load(get_web_asset_url("diffuse_room2.ktx2")),
            specular_map: asset_server.load(get_web_asset_url("specular_room2.ktx2")),
            intensity: LIGHT_PROBE_INTENSITY,
            ..default()
        },
        Transform::from_scale(vec3(1.0, -1.0, 1.0) * LIGHT_PROBE_SIDE_LENGTH)
            .with_rotation(Quat::from_rotation_x(PI))
            .with_translation(vec3(0.0, 0.0, -ROOM_SEPARATION)),
        ParallaxCorrection::Custom(Vec3::splat(LIGHT_PROBE_PARALLAX_CORRECTION_SIDE_LENGTH)),
    ));
}

/// Spawns the radio buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands) {
    commands.spawn((
        widgets::main_ui_node(),
        children![
            widgets::option_buttons(
                "Gizmos",
                &[(GizmosEnabled::On, "On"), (GizmosEnabled::Off, "Off"),]
            ),
            widgets::option_buttons(
                "Object to Show",
                &[
                    (ObjectToShow::Sphere, "Sphere"),
                    (ObjectToShow::Prism, "Prism"),
                ]
            ),
            widgets::option_buttons(
                "Camera Mode",
                &[(CameraMode::Orbit, "Orbit"), (CameraMode::Free, "Free"),]
            ),
        ],
    ));
}

/// Spawns the help text at the top of the screen.
fn spawn_help_text(commands: &mut Commands) {
    commands.spawn((
        Text::new(""),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
        HelpText,
    ));
}

/// Moves the sphere a bit every frame.
fn move_sphere(mut spheres: Query<&mut Transform, With<ReflectiveSphere>>, time: Res<Time>) {
    let Some(t) = SmoothStepCurve
        .ping_pong()
        .unwrap()
        .forever()
        .unwrap()
        .sample(time.elapsed_secs() * SPHERE_MOVEMENT_SPEED)
    else {
        return;
    };
    for mut sphere_transform in &mut spheres {
        sphere_transform.translation.z = -ROOM_SEPARATION * t;
    }
}

/// Processes requests from the user to move the camera.
fn orbit_camera(
    mut cameras: Query<(&mut Transform, &mut OrbitCamera)>,
    spheres: Query<&Transform, (With<ReflectiveSphere>, Without<OrbitCamera>)>,
    mouse_buttons: Res<ButtonInput<MouseButton>>,
    mouse_motion: Res<AccumulatedMouseMotion>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    // Grab the sphere transform.
    let Some(sphere_transform) = spheres.iter().next() else {
        return;
    };

    for (mut camera_transform, mut orbit_camera) in &mut cameras {
        // Only pan if the left mouse button is pressed.
        if mouse_buttons.pressed(MouseButton::Left) {
            let delta = mouse_motion.delta;
            orbit_camera.azimuth -= delta.x * CAMERA_ORBIT_SPEED_AZIMUTH;
            orbit_camera.inclination += delta.y * CAMERA_ORBIT_SPEED_INCLINATION;
        }

        // Zooming doesn't require a mouse button press, as it uses the mouse
        // wheel.
        orbit_camera.radius =
            (orbit_camera.radius - CAMERA_ZOOM_SPEED * mouse_scroll.delta.y).max(0.01);

        // Calculate the new translation using the [spherical coordinates
        // formula].
        //
        // [spherical coordinates formula]:
        // https://en.wikipedia.org/wiki/Spherical_coordinate_system#Cartesian_coordinates
        let new_translation = orbit_camera.radius
            * vec3(
                sin(orbit_camera.inclination) * cos(orbit_camera.azimuth),
                cos(orbit_camera.inclination),
                sin(orbit_camera.inclination) * sin(orbit_camera.azimuth),
            );

        // Write in the new transform.
        *camera_transform =
            Transform::from_translation(new_translation + sphere_transform.translation)
                .looking_at(sphere_transform.translation, Vec3::Y);
    }
}

/// A system that toggles gizmos on or off when the user clicks on one of the
/// corresponding radio buttons.
fn handle_gizmos_enabled_change(
    mut help_text_query: Query<&mut Text, With<HelpText>>,
    mut app_status: ResMut<AppStatus>,
    mut messages: MessageReader<WidgetClickEvent<GizmosEnabled>>,
) {
    let mut any_changes = false;
    for message in messages.read() {
        app_status.gizmos_enabled = **message;
        any_changes = true;
    }

    if any_changes {
        set_help_text(&app_status, &mut help_text_query);
    }
}

/// A system that toggles object visibility when the user clicks on one of the
/// corresponding radio buttons.
fn handle_object_to_show_change(
    mut spheres_query: Query<&mut Visibility, (With<ReflectiveSphere>, Without<ReflectivePrism>)>,
    mut prisms_query: Query<&mut Visibility, (With<ReflectivePrism>, Without<ReflectiveSphere>)>,
    mut app_status: ResMut<AppStatus>,
    mut messages: MessageReader<WidgetClickEvent<ObjectToShow>>,
) {
    for message in messages.read() {
        app_status.object_to_show = **message;

        for mut sphere_visibility in &mut spheres_query {
            *sphere_visibility = match **message {
                ObjectToShow::Sphere => Visibility::Inherited,
                ObjectToShow::Prism => Visibility::Hidden,
            }
        }
        for mut prism_visibility in &mut prisms_query {
            *prism_visibility = match **message {
                ObjectToShow::Sphere => Visibility::Hidden,
                ObjectToShow::Prism => Visibility::Inherited,
            }
        }
    }
}

/// A system that toggles the camera mode when the user clicks on one of the
/// corresponding radio buttons.
fn handle_camera_mode_change(
    mut commands: Commands,
    cameras_query: Query<(Entity, &Transform), With<Camera3d>>,
    sphere_query: Query<&Transform, (With<ReflectiveSphere>, Without<Camera3d>)>,
    mut help_text_query: Query<&mut Text, With<HelpText>>,
    mut windows_query: Query<&mut CursorOptions>,
    mut app_status: ResMut<AppStatus>,
    mut messages: MessageReader<WidgetClickEvent<CameraMode>>,
) {
    let Some(sphere_transform) = sphere_query.iter().next() else {
        return;
    };

    let mut any_changes = false;
    for message in messages.read() {
        app_status.camera_mode = **message;

        match **message {
            CameraMode::Orbit => {
                for (camera_entity, camera_transform) in &cameras_query {
                    // Convert from Cartesian coordinates back to spherical
                    // coordinates.
                    let relative_camera_position =
                        camera_transform.translation - sphere_transform.translation;
                    let radius = relative_camera_position.length();
                    let inclination = atan2(
                        relative_camera_position.xz().length() / radius,
                        relative_camera_position.y / radius,
                    );
                    let azimuth = atan2(
                        relative_camera_position.z * relative_camera_position.xz().length_recip(),
                        relative_camera_position.x * relative_camera_position.xz().length_recip(),
                    );

                    commands
                        .entity(camera_entity)
                        .remove::<FreeCamera>()
                        .insert(OrbitCamera {
                            radius,
                            inclination,
                            azimuth,
                        });
                }
            }

            CameraMode::Free => {
                for (camera_entity, _) in &cameras_query {
                    commands
                        .entity(camera_entity)
                        .remove::<OrbitCamera>()
                        .insert(FreeCamera::default());
                }
            }
        }

        any_changes = true;
    }

    if any_changes {
        set_help_text(&app_status, &mut help_text_query);

        // Reset the cursor grab mode, because the free camera controller may
        // have enabled it, and we don't want the cursor to disappear.
        for mut cursor_options in &mut windows_query {
            cursor_options.grab_mode = CursorGrabMode::None;
            cursor_options.visible = true;
        }
    }
}

/// A system that updates the radio buttons at the bottom of the screen to
/// reflect whether gizmos are enabled or not.
fn update_radio_buttons(
    mut widgets_query: Query<(
        Entity,
        Option<&mut BackgroundColor>,
        Has<Text>,
        AnyOf<(
            &WidgetClickSender<GizmosEnabled>,
            &WidgetClickSender<ObjectToShow>,
            &WidgetClickSender<CameraMode>,
        )>,
    )>,
    app_status: Res<AppStatus>,
    mut text_ui_writer: TextUiWriter,
) {
    for (
        entity,
        maybe_bg_color,
        has_text,
        (maybe_gizmos_enabled, maybe_object_to_show, maybe_camera_mode),
    ) in &mut widgets_query
    {
        let selected = if let Some(sender) = maybe_gizmos_enabled {
            app_status.gizmos_enabled == **sender
        } else if let Some(sender) = maybe_object_to_show {
            app_status.object_to_show == **sender
        } else if let Some(sender) = maybe_camera_mode {
            app_status.camera_mode == **sender
        } else {
            continue;
        };

        if let Some(mut bg_color) = maybe_bg_color {
            widgets::update_ui_radio_button(&mut bg_color, selected);
        }
        if has_text {
            widgets::update_ui_radio_button_text(entity, &mut text_ui_writer, selected);
        }
    }
}

/// Draws gizmos that show the boundaries of the various boxes associated with
/// the light probes in the scene.
fn draw_gizmos(
    light_probes: Query<(&LightProbe, &ParallaxCorrection, &Transform)>,
    app_status: Res<AppStatus>,
    mut gizmos: Gizmos,
) {
    // If the user has gizmos disabled, bail.
    if matches!(app_status.gizmos_enabled, GizmosEnabled::Off) {
        return;
    }

    for (light_probe, parallax_correction, transform) in &light_probes {
        // Draw light probe bounds.
        gizmos.cube(*transform, TAN);

        // Draw light probe falloff.
        gizmos.cube(
            Transform {
                scale: transform.scale * (Vec3::ONE - light_probe.falloff),
                ..*transform
            },
            CRIMSON,
        );

        // Draw light probe parallax correction bounds.
        if let ParallaxCorrection::Custom(parallax_correction_bounds) = *parallax_correction {
            gizmos.cube(
                Transform {
                    scale: transform.scale * parallax_correction_bounds,
                    ..*transform
                },
                CORNFLOWER_BLUE,
            );
        }
    }
}

/// Updates the help text at the top of the screen to reflect a change in camera
/// or gizmo application settings.
fn set_help_text(app_status: &AppStatus, help_text_query: &mut Query<&mut Text, With<HelpText>>) {
    for mut ui_text in help_text_query {
        let mut help_text = String::new();
        match app_status.camera_mode {
            CameraMode::Orbit => {
                help_text.push_str(
                    "Click and drag to orbit the camera\nUse the mouse wheel to zoom the camera\n",
                );
            }
            CameraMode::Free => {
                help_text.push_str(
                    "Click and drag to rotate the camera\nUse WASDEQ to move the camera\n",
                );
            }
        }

        help_text.push('\n');

        if matches!(app_status.gizmos_enabled, GizmosEnabled::On) {
            help_text.push_str(
                "\
Gizmos:
Tan: Light probe bounds
Red: Light probe falloff bounds
Blue: Parallax correction bounds",
            );
        }

        *ui_text = Text::new(help_text);
    }
}

/// Returns the GitHub download URL for the given asset.
///
/// The files are expected to be in the `light_probe_blending` directory in the
/// [repository].
///
/// [repository]: https://github.com/bevyengine/bevy_asset_files
fn get_web_asset_url(name: &str) -> String {
    format!(
        "https://raw.githubusercontent.com/bevyengine/bevy_asset_files/refs/heads/main/\
light_probe_blending/{}",
        name
    )
}
