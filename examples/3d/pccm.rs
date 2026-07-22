//! Demonstrates parallax-corrected cubemap reflections.

use core::f32;

use bevy::{
    camera::Hdr,
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    feathers::{theme::UiTheme, FeathersPlugins},
    light::ParallaxCorrection,
    prelude::*,
    ui_widgets::{radio_self_update, ValueChange},
};

#[path = "../helpers/radio.rs"]
mod radio;

#[path = "../helpers/theme.rs"]
mod theme;

use crate::radio::{feathers_option_buttons, main_ui_node_scene, RadioButtonOptionValue};
use crate::theme::basic_example_theme;

/// A marker component for the inner rotating reflective cube.
#[derive(Clone, Component)]
struct InnerCube;

/// The brightness of the cubemap.
///
/// Since the cubemap image was baked in Blender, which uses a different
/// exposure setting than that of Bevy, we need this factor in order to make the
/// exposure of the baked image match ours.
const ENVIRONMENT_MAP_INTENSITY: f32 = 100.0;

const OUTER_CUBE_URL: &str =
    "https://github.com/bevyengine/bevy_asset_files/raw/main/pccm_example/outer_cube.glb#Scene0";
const ENV_DIFFUSE_URL: &str =
    "https://github.com/bevyengine/bevy_asset_files/raw/main/pccm_example/env_diffuse.ktx2";
const ENV_SPECULAR_URL: &str =
    "https://github.com/bevyengine/bevy_asset_files/raw/main/pccm_example/env_specular.ktx2";

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
        .insert_resource(UiTheme(basic_example_theme(Color::WHITE)))
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Bevy Parallax-Corrected Cubemaps Example".into(),
                    ..default()
                }),
                ..default()
            }),
            FreeCameraPlugin,
            FeathersPlugins,
        ))
        .init_resource::<AppStatus>()
        .add_systems(Startup, setup)
        .add_observer(handle_pccm_enable_change)
        .add_observer(radio_self_update)
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
    commands.spawn(WorldAssetRoot(asset_server.load(OUTER_CUBE_URL)));

    spawn_camera(&mut commands);
    spawn_inner_cube(&mut commands, &mut meshes, &mut materials);
    spawn_reflection_probe(&mut commands, &asset_server);
    spawn_buttons(&mut commands);
}

/// Spawns the camera.
fn spawn_camera(commands: &mut Commands) {
    commands.spawn((
        Camera3d::default(),
        FreeCamera::default(),
        Transform::from_xyz(0.0, 0.0, 4.0).looking_at(Vec3::new(0.0, -2.5, 0.0), Dir3::Y),
        Hdr,
    ));
}

/// Spawns the inner reflective cube in the scene.
fn spawn_inner_cube(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let cube_mesh = meshes.add(
        Cuboid {
            half_size: Vec3::new(5.0, 1.0, 2.0),
        }
        .mesh()
        .build()
        .with_duplicated_vertices()
        .with_computed_flat_normals(),
    );
    let cube_material = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        metallic: 1.0,
        reflectance: 1.0,
        perceptual_roughness: 0.0,
        ..default()
    });

    commands.spawn((
        Mesh3d(cube_mesh),
        MeshMaterial3d(cube_material),
        Transform::from_xyz(0.0, -4.0, -2.5),
        InnerCube,
    ));
}

/// Spawns the reflection probe (i.e. cubemap reflection) in the center of the scene.
fn spawn_reflection_probe(commands: &mut Commands, asset_server: &AssetServer) {
    let diffuse_map = asset_server.load(ENV_DIFFUSE_URL);
    let specular_map = asset_server.load(ENV_SPECULAR_URL);
    commands.spawn((
        LightProbe::default(),
        EnvironmentMapLight {
            diffuse_map,
            specular_map,
            intensity: ENVIRONMENT_MAP_INTENSITY,
            ..default()
        },
        // HACK: slightly larger than 10.0 to avoid z-fighting from the outer cube
        // faces being partially inside and partially outside the light probe influence
        // volume. We should have a smooth falloff probe transition option at some point.
        Transform::from_scale(Vec3::splat(10.01)),
    ));
}

/// Spawns the buttons at the bottom of the screen.
fn spawn_buttons(commands: &mut Commands) {
    commands.spawn_scene(bsn! {
        main_ui_node_scene()
        Children [
            feathers_option_buttons(
                "Parallax Correction",
                &[
                    (PccmEnableStatus::Enabled, "On"),
                    (PccmEnableStatus::Disabled, "Off"),
                ],
            )
        ]
    });
}

/// Handles a change to the parallax correction setting UI.
fn handle_pccm_enable_change(
    event: On<ValueChange<Entity>>,
    new_value_query: Query<&RadioButtonOptionValue<PccmEnableStatus>>,
    mut commands: Commands,
    light_probe_query: Query<Entity, With<LightProbe>>,
    mut app_status: ResMut<AppStatus>,
) {
    let Ok(RadioButtonOptionValue(pccm_enabled)) = new_value_query.get(event.value) else {
        return;
    };
    app_status.pccm_enabled = *pccm_enabled;

    let Some(light_probe_entity) = light_probe_query.iter().next() else {
        return;
    };
    // Add the appropriate variant of the `ParallaxCorrection` component.
    match app_status.pccm_enabled {
        PccmEnableStatus::Enabled => {
            commands
                .entity(light_probe_entity)
                .insert(ParallaxCorrection::Auto);
        }
        PccmEnableStatus::Disabled => {
            commands
                .entity(light_probe_entity)
                .insert(ParallaxCorrection::None);
        }
    }
}
