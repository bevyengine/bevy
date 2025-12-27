//! Demonstrates parallax-corrected cubemap reflections.

use bevy::{light::ParallaxCorrect, math::ops, prelude::*, render::view::Hdr};

use crate::widgets::{WidgetClickEvent, WidgetClickSender};

#[path = "../helpers/widgets.rs"]
mod widgets;

/// A marker component for the inner rotating reflective cube.
#[derive(Clone, Component)]
struct InnerCube;

/// The brightness of the cubemap.
///
/// Since the cubemap image was baked in Blender, which uses a different
/// exposure setting than that of Bevy, we need this factor in order to make the
/// exposure of the baked image match ours.
const ENVIRONMENT_MAP_INTENSITY: f32 = 2000.0;

/// The speed at which the camera rotates in radians per second.
const CAMERA_ROTATION_SPEED: f32 = 0.25;

/// The speed at which the rotating inner cube rotates about the X axis, in radians per second.
const INNER_CUBE_ROTATION_SPEED_X: f32 = 1.5;
/// The speed at which the rotating inner cube rotates about the Z axis, in radians per second.
const INNER_CUBE_ROTATION_SPEED_Z: f32 = 1.3;

/// The current value of user-customizable settings for this demo.
#[derive(Resource, Default)]
struct AppStatus {
    /// Whether parallax correction is enabled.
    pccm_enabled: PccmEnableStatus,
}

/// Whether parallax correction is enabled.
#[derive(Clone, Copy, PartialEq, Default)]
enum PccmEnableStatus {
    /// Parallax correction is enabled.
    #[default]
    Enabled,
    /// Parallax correction is disabled.
    Disabled,
}

/// The example entry point.
fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Parallax-Corrected Cubemaps Example".into(),
                ..default()
            }),
            ..default()
        }))
        .init_resource::<AppStatus>()
        .add_message::<WidgetClickEvent<PccmEnableStatus>>()
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, rotate_inner_cube)
        .add_systems(Update, widgets::handle_ui_interactions::<PccmEnableStatus>)
        .add_systems(
            Update,
            (handle_pccm_enable_change, update_radio_buttons)
                .after(widgets::handle_ui_interactions::<PccmEnableStatus>),
        )
        .add_systems(Update, rotate_camera)
        .run();
}

/// Creates the initial scene.
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Spawn the glTF scene.
    commands.spawn(SceneRoot(
        asset_server.load("models/PCCMExample/PCCMExample.glb#Scene0"),
    ));

    spawn_camera(&mut commands);
    spawn_inner_cube(&mut commands, &mut meshes, &mut materials);
    spawn_reflection_probe(&mut commands, &asset_server);
    spawn_buttons(&mut commands);
}

/// Spawns the rotating camera.
fn spawn_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0.0, 0.0, 3.5).looking_at(Vec3::ZERO, Dir3::Y),
        Hdr,
    ));
}

/// Spawns the inner rotating reflective cube in the center of the scene.
fn spawn_inner_cube(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let cube_mesh = meshes.add(
        Cuboid::default()
            .mesh()
            .build()
            .with_duplicated_vertices()
            .with_computed_flat_normals(),
    );
    let cube_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        metallic: 1.0,
        perceptual_roughness: 0.0,
        ..default()
    });

    commands.spawn((
        Mesh3d(cube_mesh),
        MeshMaterial3d(cube_material),
        Transform::default(),
        InnerCube,
    ));
}

/// Spawns the reflection probe (i.e. cubemap reflection) in the center of the scene.
fn spawn_reflection_probe(commands: &mut Commands, asset_server: &AssetServer) {
    let diffuse_map = asset_server.load("environment_maps/BevyPCCMExample_diffuse.ktx2");
    let specular_map = asset_server.load("environment_maps/BevyPCCMExample_specular.ktx2");
    commands.spawn((
        LightProbe,
        EnvironmentMapLight {
            diffuse_map,
            specular_map,
            intensity: ENVIRONMENT_MAP_INTENSITY,
            ..default()
        },
        Transform::from_scale(Vec3::splat(5.0)),
        ParallaxCorrect,
    ));
}

/// Spawns the buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands) {
    commands.spawn((
        widgets::main_ui_node(),
        children![widgets::option_buttons(
            "Parallax Correction",
            &[
                (PccmEnableStatus::Enabled, "On"),
                (PccmEnableStatus::Disabled, "Off"),
            ],
        )],
    ));
}

/// Rotates the inner reflective cube every frame.
fn rotate_inner_cube(mut cubes_query: Query<&mut Transform, With<InnerCube>>, time: Res<Time>) {
    for mut transform in &mut cubes_query {
        transform.rotate_x(INNER_CUBE_ROTATION_SPEED_X * time.delta_secs());
        transform.rotate_z(INNER_CUBE_ROTATION_SPEED_Z * time.delta_secs());
    }
}

/// Rotates the camera every frame.
fn rotate_camera(cameras_query: Query<&mut Transform, With<Camera3d>>, time: Res<Time>) {
    let theta = time.elapsed_secs() * CAMERA_ROTATION_SPEED;
    for mut camera_transform in cameras_query {
        let distance_from_center = camera_transform.translation.length();
        *camera_transform = camera_transform
            .with_translation(vec3(ops::sin(theta), 0.0, ops::cos(theta)) * distance_from_center)
            .looking_at(Vec3::ZERO, Vec3::Y);
    }
}

/// Handles a change to the parallax correction setting UI.
fn handle_pccm_enable_change(
    mut commands: Commands,
    light_probe_query: Query<Entity, With<LightProbe>>,
    mut app_status: ResMut<AppStatus>,
    mut messages: MessageReader<WidgetClickEvent<PccmEnableStatus>>,
) {
    let Some(light_probe_entity) = light_probe_query.iter().next() else {
        return;
    };

    for message in messages.read() {
        // The UI message contains the `PccmEnableStatus` value that the user
        // selected.
        app_status.pccm_enabled = **message;

        // Add or remove the `ParallaxCorrect` component as appropriate.
        match **message {
            PccmEnableStatus::Enabled => {
                commands.entity(light_probe_entity).insert(ParallaxCorrect);
            }
            PccmEnableStatus::Disabled => {
                commands
                    .entity(light_probe_entity)
                    .remove::<ParallaxCorrect>();
            }
        }
    }
}

/// Updates the state of the UI at the bottom of the screen to reflect the
/// current application settings.
fn update_radio_buttons(
    mut widgets_query: Query<(
        Entity,
        Option<&mut BackgroundColor>,
        Has<Text>,
        &WidgetClickSender<PccmEnableStatus>,
    )>,
    app_status: Res<AppStatus>,
    mut text_ui_writer: TextUiWriter,
) {
    for (entity, maybe_bg_color, has_text, sender) in &mut widgets_query {
        // The `sender` value contains the `PccmEnableStatus` that the user
        // selected.
        let selected = app_status.pccm_enabled == **sender;

        if let Some(mut bg_color) = maybe_bg_color {
            widgets::update_ui_radio_button(&mut bg_color, selected);
        }
        if has_text {
            widgets::update_ui_radio_button_text(entity, &mut text_ui_writer, selected);
        }
    }
}
